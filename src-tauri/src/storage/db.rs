use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::error::{AppError, AppResult, ErrorCode};

/// Migrações versionadas, aplicadas em ordem. A versão atual fica em `PRAGMA user_version`.
/// Nunca edite uma migração já publicada: acrescente outra.
const MIGRATIONS: &[&str] = &[include_str!("migrations/001_inicial.sql")];

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> AppResult<Self> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn conn(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal, "conexão do banco envenenada"))
    }
}

fn migrate(conn: &mut Connection) -> AppResult<()> {
    let atual: usize = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if atual > MIGRATIONS.len() {
        return Err(AppError::new(
            ErrorCode::Database,
            format!(
                "banco na versão {atual}, app conhece até {}: o banco foi criado por uma versão mais nova do app",
                MIGRATIONS.len()
            ),
        ));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(atual) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i + 1)?;
        tx.commit()?;
        tracing::info!(versao = i + 1, "migração aplicada");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migra_do_zero_e_e_idempotente() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        drop(Db::open(&path).unwrap());
        let db = Db::open(&path).unwrap();
        let v: usize = db
            .conn()
            .unwrap()
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRATIONS.len());
    }
}
