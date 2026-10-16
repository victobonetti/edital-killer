use rusqlite::{Connection, OptionalExtension};

use crate::domain::{Settings, UsoClaude};
use crate::error::AppResult;

const SETTINGS: &str = "settings";
const HEALTH_JA_PASSOU: &str = "claude_health_ja_passou";

fn get(conn: &Connection, chave: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row("SELECT valor FROM config WHERE chave = ?1", [chave], |r| {
            r.get(0)
        })
        .optional()?)
}

fn set(conn: &Connection, chave: &str, valor: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO config (chave, valor) VALUES (?1, ?2)
         ON CONFLICT(chave) DO UPDATE SET valor = excluded.valor",
        [chave, valor],
    )?;
    Ok(())
}

pub fn settings(conn: &Connection) -> AppResult<Settings> {
    match get(conn, SETTINGS)? {
        // Config corrompida ou de formato antigo não deve impedir o app de abrir.
        Some(json) => Ok(serde_json::from_str(&json).unwrap_or_default()),
        None => Ok(Settings::default()),
    }
}

pub fn save_settings(conn: &Connection, s: &Settings) -> AppResult<()> {
    set(conn, SETTINGS, &serde_json::to_string(s)?)
}

pub fn health_ja_passou(conn: &Connection) -> AppResult<bool> {
    Ok(get(conn, HEALTH_JA_PASSOU)?.is_some())
}

pub fn marcar_health_passou(conn: &Connection) -> AppResult<()> {
    set(conn, HEALTH_JA_PASSOU, "1")
}

const USO_CLAUDE: &str = "claude_uso";
const FILA_PAUSADA_ATE: &str = "fila_pausada_ate";

pub fn uso_claude(conn: &Connection) -> AppResult<Option<UsoClaude>> {
    Ok(get(conn, USO_CLAUDE)?.and_then(|j| serde_json::from_str(&j).ok()))
}

pub fn salvar_uso_claude(conn: &Connection, uso: &UsoClaude) -> AppResult<()> {
    set(conn, USO_CLAUDE, &serde_json::to_string(uso)?)
}

/// ISO 8601 até quando a fila fica pausada por limite de uso.
pub fn fila_pausada_ate(conn: &Connection) -> AppResult<Option<String>> {
    get(conn, FILA_PAUSADA_ATE)
}

pub fn salvar_fila_pausada_ate(conn: &Connection, ate: Option<&str>) -> AppResult<()> {
    match ate {
        Some(ate) => set(conn, FILA_PAUSADA_ATE, ate),
        None => {
            conn.execute("DELETE FROM config WHERE chave = ?1", [FILA_PAUSADA_ATE])?;
            Ok(())
        }
    }
}
