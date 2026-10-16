use rusqlite::{Connection, Row};

use crate::domain::{Job, JobStatus};
use crate::error::AppResult;

/// Marca como `interrompido` todo job que estava `executando` quando o app fechou.
pub fn interromper_orfaos(conn: &Connection, agora: &str) -> AppResult<usize> {
    Ok(conn.execute(
        "UPDATE job SET status = ?1, finalizado_em = ?2 WHERE status = ?3",
        [
            JobStatus::Interrompido.as_str(),
            agora,
            JobStatus::Executando.as_str(),
        ],
    )?)
}

pub fn recentes(conn: &Connection, limite: u32) -> AppResult<Vec<Job>> {
    let mut stmt = conn.prepare(
        "SELECT id, edital_id, tipo, status, criado_em, iniciado_em, finalizado_em, erro_code, erro_detail
         FROM job ORDER BY criado_em DESC LIMIT ?1",
    )?;
    let jobs = stmt
        .query_map([limite], from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(jobs)
}

pub fn ids_do_edital(conn: &Connection, edital_id: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT id FROM job WHERE edital_id = ?1")?;
    let ids = stmt
        .query_map([edital_id], |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn from_row(r: &Row) -> rusqlite::Result<Job> {
    let status: String = r.get(3)?;
    let erro_code: Option<String> = r.get(7)?;
    Ok(Job {
        id: r.get(0)?,
        edital_id: r.get(1)?,
        tipo: r.get(2)?,
        status: JobStatus::parse(&status).unwrap_or(JobStatus::Falhou),
        criado_em: r.get(4)?,
        iniciado_em: r.get(5)?,
        finalizado_em: r.get(6)?,
        erro_code: erro_code.and_then(|c| serde_json::from_value(c.into()).ok()),
        erro_detail: r.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Db;

    #[test]
    fn interrompe_somente_executando() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        for (id, st) in [("a", "executando"), ("b", "concluido"), ("c", "na_fila")] {
            conn.execute(
                "INSERT INTO job (id, tipo, status, criado_em) VALUES (?1, 'x', ?2, ?1)",
                [id, st],
            )
            .unwrap();
        }
        assert_eq!(interromper_orfaos(&conn, "agora").unwrap(), 1);
        let jobs = recentes(&conn, 10).unwrap();
        let a = jobs.iter().find(|j| j.id == "a").unwrap();
        assert_eq!(a.status, JobStatus::Interrompido);
        assert_eq!(a.finalizado_em.as_deref(), Some("agora"));
    }
}
