//! Execução de um job: `claude -p` no workspace do job, prompt pela stdin,
//! saída estruturada validada pelo `--json-schema`. Stdout e stderr ficam no
//! workspace para auditoria. Erros viram códigos do catálogo; o texto cru do
//! CLI só vai para o `detail`.

use std::path::Path;
use std::process::Stdio;

use serde_json::Value;
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, ErrorCode};
use crate::llm::backend::{Execucao, FalhaJob};
use crate::llm::cancel::CancelToken;

/// Ferramentas que o `claude` pode usar: só leitura, só dentro do workspace.
const PERMITIDAS: &str = "Read,Grep,Glob";
const PROIBIDAS: &str = "Bash,Edit,Write,NotebookEdit,WebFetch,WebSearch,Task";

pub fn argumentos(ex: &Execucao) -> Vec<String> {
    let mut args: Vec<String> = [
        "-p",
        "--output-format",
        "json",
        "--json-schema",
        &ex.schema,
        "--allowedTools",
        PERMITIDAS,
        "--disallowedTools",
        PROIBIDAS,
        "--strict-mcp-config",
        "--no-session-persistence",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    if let Some(m) = &ex.modelo {
        args.push("--model".into());
        args.push(m.clone());
    }
    args
}

pub async fn executar(claude: &Path, ex: Execucao, cancel: CancelToken) -> Result<Value, FalhaJob> {
    let mut cmd = super::processo::comando(claude);
    cmd.args(argumentos(&ex))
        .current_dir(&ex.workspace)
        .stdin(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| {
        falha(
            ErrorCode::ClaudeNotRunnable,
            format!("não consegui executar {}: {e}", claude.display()),
        )
    })?;

    // O prompt vai pela stdin; fechar o pipe sinaliza o fim.
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(ex.prompt.as_bytes())
            .await
            .map_err(|e| falha(ErrorCode::ClaudeExecFailed, format!("stdin: {e}")))?;
    }

    let saida = tokio::select! {
        r = child.wait_with_output() => r.map_err(|e| falha(ErrorCode::ClaudeExecFailed, e.to_string()))?,
        _ = tokio::time::sleep(ex.timeout) => {
            // `kill_on_drop` mata o processo ao sair deste ramo.
            return Err(falha(ErrorCode::ClaudeTimeout, format!("sem resposta em {}s", ex.timeout.as_secs())));
        }
        _ = cancel.esperar() => return Err(falha(ErrorCode::JobCancelled, "cancelado pelo usuário")),
    };

    let stdout = String::from_utf8_lossy(&saida.stdout);
    let stderr = String::from_utf8_lossy(&saida.stderr);
    guardar(&ex.workspace, "saida.json", &stdout);
    guardar(&ex.workspace, "stderr.txt", &stderr);
    interpretar(&stdout, &stderr)
}

fn guardar(workspace: &Path, nome: &str, conteudo: &str) {
    if let Err(e) = std::fs::write(workspace.join(nome), conteudo) {
        tracing::warn!(arquivo = nome, erro = %e, "falha ao gravar saída do job");
    }
}

fn falha(code: ErrorCode, detail: impl Into<String>) -> FalhaJob {
    FalhaJob {
        erro: AppError::new(code, detail),
        retorno_em: None,
    }
}

/// Traduz a saída do `claude -p --output-format json` em resultado ou erro do catálogo.
pub fn interpretar(stdout: &str, stderr: &str) -> Result<Value, FalhaJob> {
    let json: Option<Value> = stdout
        .find('{')
        .and_then(|i| serde_json::from_str(stdout[i..].trim()).ok());

    let Some(json) = json else {
        let texto = if stderr.trim().is_empty() { stdout } else { stderr };
        return Err(classificar(texto.trim(), None, None));
    };

    let texto = json.get("result").and_then(Value::as_str).unwrap_or_default();
    if json.get("is_error").and_then(Value::as_bool) == Some(true) {
        let status = json.get("api_error_status").and_then(Value::as_u64);
        let code = json
            .get("api_error_code")
            .or_else(|| json.get("api_error"))
            .and_then(Value::as_str);
        let msg = if texto.is_empty() { stderr.trim() } else { texto };
        return Err(classificar(msg, status, code));
    }

    if let Some(v) = json.get("structured_output").filter(|v| !v.is_null()) {
        return Ok(v.clone());
    }
    // CLI sem `structured_output`: o JSON pode vir como texto do resultado.
    texto
        .find('{')
        .zip(texto.rfind('}'))
        .and_then(|(a, b)| serde_json::from_str(&texto[a..=b]).ok())
        .ok_or_else(|| {
            falha(
                ErrorCode::ClaudeOutputInvalid,
                format!("resposta sem JSON: {}", resumo(texto)),
            )
        })
}

fn classificar(msg: &str, status: Option<u64>, code: Option<&str>) -> FalhaJob {
    let baixo = msg.to_lowercase();
    let code_l = code.unwrap_or_default().to_lowercase();
    let detail = match (status, code) {
        (Some(s), Some(c)) => format!("{s} {c}: {}", resumo(msg)),
        _ => resumo(msg),
    };

    let erro = if code_l.contains("credit") || baixo.contains("requires usage credits") {
        ErrorCode::ClaudeModelUnavailable
    } else if status == Some(429) || baixo.contains("usage limit") || baixo.contains("limit reached")
        || baixo.contains("hit your limit")
    {
        return FalhaJob {
            erro: AppError::new(ErrorCode::ClaudeUsageLimit, detail),
            retorno_em: retorno_do_limite(msg),
        };
    } else if status == Some(401)
        || baixo.contains("/login")
        || baixo.contains("not logged in")
        || baixo.contains("invalid api key")
    {
        ErrorCode::ClaudeNotLoggedIn
    } else if code_l.contains("model") || baixo.contains("model not found") || baixo.contains("invalid model") {
        ErrorCode::ClaudeModelUnavailable
    } else {
        ErrorCode::ClaudeExecFailed
    };
    FalhaJob {
        erro: AppError::new(erro, detail),
        retorno_em: None,
    }
}

/// O CLI anuncia o fim do limite como `...limit reached|<epoch>`; outros formatos ficam sem horário.
fn retorno_do_limite(msg: &str) -> Option<String> {
    let (_, depois) = msg.rsplit_once('|')?;
    let digitos: String = depois.chars().take_while(char::is_ascii_digit).collect();
    let epoch: i64 = digitos.parse().ok()?;
    chrono::DateTime::from_timestamp(epoch, 0).map(|d| d.to_rfc3339())
}

fn resumo(s: &str) -> String {
    const MAX: usize = 600;
    let s = s.trim();
    if s.chars().count() <= MAX {
        s.to_owned()
    } else {
        format!("{}…", s.chars().take(MAX).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn erro(r: Result<Value, FalhaJob>) -> FalhaJob {
        r.expect_err("esperava erro")
    }

    #[test]
    fn sucesso_com_structured_output() {
        let out = r#"{"type":"result","is_error":false,"result":"{\"a\":1}","structured_output":{"a":2}}"#;
        assert_eq!(interpretar(out, "").unwrap()["a"], 2);
    }

    #[test]
    fn sucesso_so_com_result_textual() {
        let out = r#"{"type":"result","is_error":false,"result":"Aqui está:\n{\"a\":1}\n"}"#;
        assert_eq!(interpretar(out, "").unwrap()["a"], 1);
    }

    #[test]
    fn resposta_sem_json_e_invalida() {
        let out = r#"{"type":"result","is_error":false,"result":"não consegui"}"#;
        assert_eq!(erro(interpretar(out, "")).erro.code, ErrorCode::ClaudeOutputInvalid);
    }

    #[test]
    fn modelo_que_exige_creditos() {
        let out = r#"{"is_error":true,"api_error_status":429,"api_error_code":"credits_required","result":"Fable 5.1 requires usage credits."}"#;
        assert_eq!(erro(interpretar(out, "")).erro.code, ErrorCode::ClaudeModelUnavailable);
    }

    #[test]
    fn limite_de_uso_com_horario() {
        let out = r#"{"is_error":true,"result":"Claude AI usage limit reached|1893456000"}"#;
        let f = erro(interpretar(out, ""));
        assert_eq!(f.erro.code, ErrorCode::ClaudeUsageLimit);
        assert_eq!(f.retorno_em.as_deref(), Some("2030-01-01T00:00:00+00:00"));
    }

    #[test]
    fn sem_login() {
        let out = r#"{"is_error":true,"result":"Invalid API key · Please run /login"}"#;
        assert_eq!(erro(interpretar(out, "")).erro.code, ErrorCode::ClaudeNotLoggedIn);
    }

    #[test]
    fn processo_quebrado_sem_stdout() {
        let f = erro(interpretar("", "panic: boom"));
        assert_eq!(f.erro.code, ErrorCode::ClaudeExecFailed);
        assert!(f.erro.detail.contains("boom"));
    }

    #[test]
    fn argumentos_trazem_schema_e_modelo() {
        let ex = Execucao {
            workspace: ".".into(),
            prompt: String::new(),
            schema: "{}".into(),
            modelo: Some("sonnet".into()),
            timeout: Duration::from_secs(600),
        };
        let a = argumentos(&ex);
        assert!(a.windows(2).any(|w| w == ["--json-schema", "{}"]));
        assert!(a.windows(2).any(|w| w == ["--model", "sonnet"]));
        assert!(!a.iter().any(|s| s == "--bare"));
    }

    // ---- Com o fake-claude: processo real, workspace real. ----

    use crate::test_support::fake_claude;

    fn workspace(cenario: &str, saida: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("fake-cenario.txt"), cenario).unwrap();
        std::fs::write(dir.path().join("fake-saida.json"), saida).unwrap();
        dir
    }

    fn execucao(dir: &tempfile::TempDir, timeout: Duration) -> Execucao {
        Execucao {
            workspace: dir.path().to_path_buf(),
            prompt: "Leia edital.txt".into(),
            schema: r#"{"type":"object"}"#.into(),
            modelo: Some("sonnet".into()),
            timeout,
        }
    }

    #[tokio::test]
    async fn executa_e_grava_auditoria() {
        let dir = workspace("ok", r#"{"cargos":[]}"#);
        let v = executar(&fake_claude(), execucao(&dir, Duration::from_secs(30)), CancelToken::default())
            .await
            .unwrap();
        assert!(v["cargos"].is_array());
        let stdin = std::fs::read_to_string(dir.path().join("fake-stdin.txt")).unwrap();
        assert_eq!(stdin, "Leia edital.txt");
        let args = std::fs::read_to_string(dir.path().join("fake-args.txt")).unwrap();
        assert!(args.lines().any(|l| l == "--no-session-persistence"));
        assert!(dir.path().join("saida.json").is_file());
        assert!(dir.path().join("stderr.txt").is_file());
    }

    #[tokio::test]
    async fn json_no_texto_do_resultado() {
        let dir = workspace("texto", r#"{"x":1}"#);
        let v = executar(&fake_claude(), execucao(&dir, Duration::from_secs(30)), CancelToken::default())
            .await
            .unwrap();
        assert_eq!(v["x"], 1);
    }

    #[tokio::test]
    async fn erros_do_cli_viram_codigos() {
        for (cenario, code) in [
            ("sem_json", ErrorCode::ClaudeOutputInvalid),
            ("limite_uso", ErrorCode::ClaudeUsageLimit),
            ("creditos", ErrorCode::ClaudeModelUnavailable),
            ("sem_login", ErrorCode::ClaudeNotLoggedIn),
            ("quebra", ErrorCode::ClaudeExecFailed),
        ] {
            let dir = workspace(cenario, "{}");
            let f = executar(&fake_claude(), execucao(&dir, Duration::from_secs(30)), CancelToken::default())
                .await
                .unwrap_err();
            assert_eq!(f.erro.code, code, "cenário {cenario}");
        }
    }

    #[tokio::test]
    async fn timeout_mata_o_processo() {
        let dir = workspace("demora", "{}");
        let inicio = std::time::Instant::now();
        let f = executar(&fake_claude(), execucao(&dir, Duration::from_secs(1)), CancelToken::default())
            .await
            .unwrap_err();
        assert_eq!(f.erro.code, ErrorCode::ClaudeTimeout);
        assert!(inicio.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test]
    async fn cancelamento_interrompe() {
        let dir = workspace("demora", "{}");
        let cancel = CancelToken::default();
        let c = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            c.cancelar();
        });
        let f = executar(&fake_claude(), execucao(&dir, Duration::from_secs(60)), cancel)
            .await
            .unwrap_err();
        assert_eq!(f.erro.code, ErrorCode::JobCancelled);
    }
}
