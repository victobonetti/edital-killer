//! Descoberta do executável `claude`: caminho manual das Configurações, depois
//! `PATH`, depois locais de instalação conhecidos (apps GUI no macOS costumam
//! herdar um `PATH` mínimo, sem `~/.local/bin`).

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult, ErrorCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encontrado {
    pub caminho: PathBuf,
    pub origem: &'static str,
}

pub fn descobrir(manual: Option<&str>) -> AppResult<Encontrado> {
    if let Some(m) = manual.map(str::trim).filter(|m| !m.is_empty()) {
        let p = PathBuf::from(m);
        return if p.is_file() {
            Ok(Encontrado {
                caminho: p,
                origem: "manual",
            })
        } else {
            Err(AppError::new(
                ErrorCode::ClaudeNotFound,
                format!("caminho configurado não existe: {m}"),
            ))
        };
    }
    if let Ok(p) = which::which("claude") {
        return Ok(Encontrado {
            caminho: p,
            origem: "path",
        });
    }
    candidatos(home().as_deref())
        .into_iter()
        .find(|p| p.is_file())
        .map(|caminho| Encontrado {
            caminho,
            origem: "local_conhecido",
        })
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::ClaudeNotFound,
                "claude não está no PATH nem nos locais de instalação conhecidos",
            )
        })
}

fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

fn candidatos(home: Option<&Path>) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if cfg!(windows) {
        if let Some(h) = home {
            v.push(h.join(".local").join("bin").join("claude.exe"));
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            v.push(PathBuf::from(appdata).join("npm").join("claude.cmd"));
        }
    } else {
        if let Some(h) = home {
            v.push(h.join(".local/bin/claude"));
            v.push(h.join(".claude/local/claude"));
            v.push(h.join(".npm-global/bin/claude"));
        }
        v.push("/opt/homebrew/bin/claude".into());
        v.push("/usr/local/bin/claude".into());
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_inexistente_e_erro() {
        let err = descobrir(Some("/nao/existe/claude")).unwrap_err();
        assert_eq!(err.code, ErrorCode::ClaudeNotFound);
    }

    #[test]
    fn manual_existente_vence() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("claude-custom");
        std::fs::write(&p, b"").unwrap();
        let e = descobrir(Some(p.to_str().unwrap())).unwrap();
        assert_eq!(e.caminho, p);
        assert_eq!(e.origem, "manual");
    }

    #[test]
    fn candidatos_incluem_local_bin() {
        let c = candidatos(Some(Path::new("/h")));
        assert!(c.iter().any(|p| p.starts_with("/h/.local")));
    }
}
