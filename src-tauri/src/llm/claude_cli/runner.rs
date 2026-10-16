//! Uma execução do `claude` sobre um workspace de job.
//!
//! Entrada no workspace: `system.md`, `prompt.md` (vai por stdin) e `schema.json`.
//! Saída no workspace: `saida.jsonl` (stdout cru) e `stderr.txt`.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::watch;

use super::processo::{self, Arvore};
use super::stream::{self, Evento, RateLimitInfo, Resultado};

pub const SYSTEM: &str = "system.md";
pub const PROMPT: &str = "prompt.md";
pub const SCHEMA: &str = "schema.json";
pub const SAIDA: &str = "saida.jsonl";
pub const STDERR: &str = "stderr.txt";

pub struct Execucao<'a> {
    pub claude: &'a Path,
    pub workspace: &'a Path,
    pub modelo: Option<&'a str>,
    pub timeout: Duration,
}

/// O processo terminou (com ou sem `result`).
#[derive(Debug)]
pub struct Saida {
    pub resultado: Option<Resultado>,
    pub ultimo_rate_limit: Option<RateLimitInfo>,
    /// Fim do stderr, para o `detail` quando não há `result`.
    pub stderr_fim: String,
    pub codigo_saida: Option<i32>,
}

#[derive(Debug, PartialEq)]
pub enum Falha {
    Cancelado,
    Timeout(Duration),
    /// Não deu para iniciar ou falar com o processo.
    Processo(String),
}

/// Argumentos fixos. O prompt de sistema substitui o do Claude Code e o job fica
/// isolado da configuração pessoal (hooks, MCP, CLAUDE.md); ver a spec, Item 3.
pub fn argumentos(schema: &str, modelo: Option<&str>) -> Vec<String> {
    let mut a: Vec<String> = [
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--no-session-persistence",
        "--tools",
        "",
        "--strict-mcp-config",
        "--setting-sources",
        "",
        "--system-prompt-file",
        SYSTEM,
        "--json-schema",
        schema,
    ]
    .map(String::from)
    .into();
    if let Some(m) = modelo {
        a.extend(["--model".into(), m.into()]);
    }
    a
}

/// Roda o `claude` até terminar, estourar o tempo ou ser cancelado. `on_rate_limit`
/// é chamado a cada `rate_limit_event`, para a UI ver a utilização ao vivo.
pub async fn executar(
    ex: &Execucao<'_>,
    mut cancelar: watch::Receiver<bool>,
    mut on_rate_limit: impl FnMut(&RateLimitInfo) + Send,
) -> Result<Saida, Falha> {
    let io = |e: std::io::Error| Falha::Processo(e.to_string());
    let schema = std::fs::read_to_string(ex.workspace.join(SCHEMA)).map_err(io)?;
    let prompt = std::fs::read(ex.workspace.join(PROMPT)).map_err(io)?;

    let mut cmd = Command::new(ex.claude);
    cmd.current_dir(ex.workspace)
        .args(argumentos(&schema, ex.modelo))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    processo::preparar(&mut cmd);
    let mut child = cmd
        .spawn()
        .map_err(|e| Falha::Processo(format!("não consegui executar {}: {e}", ex.claude.display())))?;
    let arvore = Arvore::anexar(&child);

    let mut stdin = child.stdin.take().expect("stdin piped");
    let stdout = child.stdout.take().expect("stdout piped");
    let mut stderr = child.stderr.take().expect("stderr piped");
    let caminho_saida = ex.workspace.join(SAIDA);

    let trabalho = async {
        // Escrever, ler stdout e ler stderr ao mesmo tempo: um prompt grande
        // pode encher o pipe enquanto o filho já escreve.
        let escrever = async {
            let r = stdin.write_all(&prompt).await;
            drop(stdin);
            r
        };
        let ler_stderr = async {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf).await;
            buf
        };
        let ler_stdout = async {
            let mut arquivo = tokio::fs::File::create(&caminho_saida).await?;
            let mut linhas = BufReader::new(stdout).lines();
            let mut resultado = None;
            let mut rate = None;
            while let Some(linha) = linhas.next_line().await? {
                arquivo.write_all(linha.as_bytes()).await?;
                arquivo.write_all(b"\n").await?;
                match stream::parse_linha(&linha) {
                    Evento::RateLimit(info) => {
                        on_rate_limit(&info);
                        rate = Some(info);
                    }
                    Evento::Resultado(r) => resultado = Some(r),
                    Evento::Outro => {}
                }
            }
            arquivo.flush().await?;
            Ok::<_, std::io::Error>((resultado, rate))
        };
        let (escrita, stderr_bytes, lidos) = tokio::join!(escrever, ler_stderr, ler_stdout);
        let status = child.wait().await.map_err(io)?;
        let _ = std::fs::write(ex.workspace.join(STDERR), &stderr_bytes);
        let (resultado, ultimo_rate_limit) = lidos.map_err(io)?;
        // Escrita falhou e não veio resultado: o processo morreu cedo; o stderr explica.
        if let (Err(e), None) = (&escrita, &resultado) {
            tracing::warn!(erro = %e, "falha ao escrever o prompt no stdin do claude");
        }
        let stderr_txt = String::from_utf8_lossy(&stderr_bytes);
        let fim: Vec<&str> = stderr_txt.trim().lines().rev().take(15).collect();
        Ok(Saida {
            resultado,
            ultimo_rate_limit,
            stderr_fim: fim.into_iter().rev().collect::<Vec<_>>().join("\n"),
            codigo_saida: status.code(),
        })
    };

    let cancelado = async {
        // Sender descartado não é cancelamento.
        if cancelar.wait_for(|c| *c).await.is_err() {
            std::future::pending::<()>().await;
        }
    };

    tokio::select! {
        r = trabalho => r,
        _ = tokio::time::sleep(ex.timeout) => {
            arvore.matar();
            Err(Falha::Timeout(ex.timeout))
        }
        _ = cancelado => {
            arvore.matar();
            Err(Falha::Cancelado)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argumentos_isolam_o_job() {
        let a = argumentos("{}", Some("haiku"));
        let pos = |f: &str| a.iter().position(|x| x == f).unwrap();
        assert_eq!(a[pos("--tools") + 1], "");
        assert_eq!(a[pos("--setting-sources") + 1], "");
        assert_eq!(a[pos("--output-format") + 1], "stream-json");
        assert_eq!(a[pos("--json-schema") + 1], "{}");
        assert_eq!(a[pos("--model") + 1], "haiku");
        assert!(a.contains(&"--strict-mcp-config".to_owned()));
        assert!(!argumentos("{}", None).contains(&"--model".to_owned()));
    }
}
