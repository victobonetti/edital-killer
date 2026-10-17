//! Health check: `claude --version` e `claude auth status`. Do `auth status`
//! só lemos `loggedIn` e `authMethod`; o resto (e-mail, organização) é ignorado.

use std::path::Path;
use std::process::{Output, Stdio};
use std::time::Duration;

use serde::Deserialize;

use crate::error::{AppError, AppResult, ErrorCode};

const TIMEOUT_VERSAO: Duration = Duration::from_secs(15);
const TIMEOUT_AUTH: Duration = Duration::from_secs(20);

pub async fn versao(claude: &Path) -> AppResult<String> {
    let out = executar(claude, &["--version"], TIMEOUT_VERSAO).await?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() {
        return Err(AppError::new(
            ErrorCode::ClaudeNotRunnable,
            format!(
                "--version saiu com {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        ));
    }
    parse_versao(&stdout).ok_or_else(|| {
        AppError::new(
            ErrorCode::ClaudeNotRunnable,
            format!("saída inesperada de --version: {}", stdout.trim()),
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Login {
    pub logado: bool,
    pub metodo: Option<String>,
}

/// `Ok(None)` quando o CLI não informa o login num formato conhecido.
pub async fn login(claude: &Path) -> AppResult<Option<Login>> {
    let out = executar(claude, &["auth", "status"], TIMEOUT_AUTH).await?;
    // Sem login o comando pode sair com erro e ainda assim imprimir o JSON.
    Ok(parse_auth_status(&String::from_utf8_lossy(&out.stdout)))
}

async fn executar(claude: &Path, args: &[&str], timeout: Duration) -> AppResult<Output> {
    let mut cmd = super::processo::comando(claude);
    cmd.args(args).stdin(Stdio::null());
    let child = cmd.spawn().map_err(|e| {
        AppError::new(
            ErrorCode::ClaudeNotRunnable,
            format!("não consegui executar {}: {e}", claude.display()),
        )
    })?;
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(r) => r.map_err(|e| AppError::new(ErrorCode::ClaudeNotRunnable, e.to_string())),
        Err(_) => Err(AppError::new(
            ErrorCode::ClaudeTimeout,
            format!("claude {} não respondeu em {}s", args.join(" "), timeout.as_secs()),
        )),
    }
}

fn parse_versao(stdout: &str) -> Option<String> {
    stdout
        .split_whitespace()
        .find(|t| t.contains('.') && t.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .map(str::to_owned)
}

fn parse_auth_status(stdout: &str) -> Option<Login> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct AuthStatus {
        logged_in: bool,
        auth_method: Option<String>,
    }
    let inicio = stdout.find('{')?;
    let fim = stdout.rfind('}')?;
    let s: AuthStatus = serde_json::from_str(stdout.get(inicio..=fim)?).ok()?;
    Some(Login {
        logado: s.logged_in,
        metodo: s.auth_method.filter(|m| !m.is_empty() && m != "none"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versao_do_cli() {
        assert_eq!(parse_versao("2.1.291 (Claude Code)\n").as_deref(), Some("2.1.291"));
        assert_eq!(parse_versao("claude 1.0.3").as_deref(), Some("1.0.3"));
        assert_eq!(parse_versao("erro"), None);
    }

    #[test]
    fn auth_status_logado() {
        let out = r#"{"loggedIn": true, "authMethod": "claude.ai", "email": "x@y.z"}"#;
        assert_eq!(
            parse_auth_status(out),
            Some(Login {
                logado: true,
                metodo: Some("claude.ai".into())
            })
        );
    }

    #[test]
    fn auth_status_deslogado_e_lixo() {
        let out = "aviso qualquer\n{\"loggedIn\": false, \"authMethod\": \"none\"}\n";
        assert_eq!(
            parse_auth_status(out),
            Some(Login {
                logado: false,
                metodo: None
            })
        );
        assert_eq!(parse_auth_status("Unknown command: auth"), None);
    }
}
