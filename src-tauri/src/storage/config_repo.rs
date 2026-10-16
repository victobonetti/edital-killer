use rusqlite::{Connection, OptionalExtension};

use crate::domain::Settings;
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
