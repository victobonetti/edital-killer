use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::domain::{ConsumoJob, Job, JobStatus};
use crate::error::{AppError, AppResult, ErrorCode};

const COLS: &str = "id, edital_id, tipo, status, criado_em, iniciado_em, finalizado_em, erro_code, erro_detail,
     tentativas, modelo, tokens_entrada, tokens_saida, tokens_cache_escrita, tokens_cache_leitura,
     tokens_raciocinio, custo_estimado_usd, duracao_ms, utilizacao_5h, utilizacao_7d";

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

pub fn inserir(
    conn: &Connection,
    id: &str,
    edital_id: Option<&str>,
    tipo: &str,
    agora: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO job (id, edital_id, tipo, status, criado_em) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, edital_id, tipo, JobStatus::NaFila.as_str(), agora],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Job> {
    let sql = format!("SELECT {COLS} FROM job WHERE id = ?1");
    conn.query_row(&sql, [id], from_row)
        .optional()?
        .ok_or_else(|| AppError::new(ErrorCode::JobNotFound, format!("job {id}")))
}

pub fn recentes(conn: &Connection, limite: u32) -> AppResult<Vec<Job>> {
    let sql = format!("SELECT {COLS} FROM job ORDER BY criado_em DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql)?;
    let jobs = stmt
        .query_map([limite], from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(jobs)
}

/// Próximo job da fila, em ordem de criação.
pub fn proximo_na_fila(conn: &Connection) -> AppResult<Option<Job>> {
    let sql = format!("SELECT {COLS} FROM job WHERE status = ?1 ORDER BY criado_em, rowid LIMIT 1");
    Ok(conn
        .query_row(&sql, [JobStatus::NaFila.as_str()], from_row)
        .optional()?)
}

pub fn contar_na_fila(conn: &Connection) -> AppResult<u32> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM job WHERE status = ?1",
        [JobStatus::NaFila.as_str()],
        |r| r.get(0),
    )?)
}

/// `false` se o job saiu da fila nesse meio-tempo (ex.: cancelado).
pub fn marcar_executando(conn: &Connection, id: &str, agora: &str) -> AppResult<bool> {
    let n = conn.execute(
        "UPDATE job SET status = ?1, iniciado_em = ?2, erro_code = NULL, erro_detail = NULL
         WHERE id = ?3 AND status = ?4",
        params![JobStatus::Executando.as_str(), agora, id, JobStatus::NaFila.as_str()],
    )?;
    Ok(n == 1)
}

/// Recusa por limite de uso: o job volta para a fila, na mesma posição.
pub fn voltar_para_fila(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE job SET status = ?1, iniciado_em = NULL WHERE id = ?2",
        params![JobStatus::NaFila.as_str(), id],
    )?;
    Ok(())
}

pub fn contar_tentativa(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE job SET tentativas = tentativas + 1 WHERE id = ?1",
        [id],
    )?;
    Ok(())
}

/// Grava o consumo acumulado e a última utilização vista.
pub fn registrar_consumo(
    conn: &Connection,
    id: &str,
    c: &ConsumoJob,
    utilizacao_5h: Option<f64>,
    utilizacao_7d: Option<f64>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE job SET modelo = ?1, tokens_entrada = ?2, tokens_saida = ?3, tokens_cache_escrita = ?4,
             tokens_cache_leitura = ?5, tokens_raciocinio = ?6, custo_estimado_usd = ?7, duracao_ms = ?8,
             utilizacao_5h = COALESCE(?9, utilizacao_5h), utilizacao_7d = COALESCE(?10, utilizacao_7d)
         WHERE id = ?11",
        params![
            c.modelo,
            c.tokens_entrada,
            c.tokens_saida,
            c.tokens_cache_escrita,
            c.tokens_cache_leitura,
            c.tokens_raciocinio,
            c.custo_estimado_usd,
            c.duracao_ms,
            utilizacao_5h,
            utilizacao_7d,
            id
        ],
    )?;
    Ok(())
}

pub fn concluir(conn: &Connection, id: &str, resultado: &str, agora: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE job SET status = ?1, resultado = ?2, finalizado_em = ?3 WHERE id = ?4",
        params![JobStatus::Concluido.as_str(), resultado, agora, id],
    )?;
    Ok(())
}

/// `falhou`, `cancelado` ou `interrompido`, com o erro quando houver.
pub fn finalizar(
    conn: &Connection,
    id: &str,
    status: JobStatus,
    erro: Option<&AppError>,
    agora: &str,
) -> AppResult<()> {
    let code = erro.map(|e| codigo_str(e.code));
    conn.execute(
        "UPDATE job SET status = ?1, erro_code = ?2, erro_detail = ?3, finalizado_em = ?4 WHERE id = ?5",
        params![status.as_str(), code, erro.map(|e| &e.detail), agora, id],
    )?;
    Ok(())
}

/// Cancela jobs ainda na fila (todos, ou só `id`). Devolve quantos cancelou.
pub fn cancelar_na_fila(conn: &Connection, id: Option<&str>, agora: &str) -> AppResult<usize> {
    Ok(conn.execute(
        "UPDATE job SET status = ?1, finalizado_em = ?2 WHERE status = ?3 AND (?4 IS NULL OR id = ?4)",
        params![
            JobStatus::Cancelado.as_str(),
            agora,
            JobStatus::NaFila.as_str(),
            id
        ],
    )?)
}

pub fn resultado(conn: &Connection, id: &str) -> AppResult<Option<String>> {
    Ok(conn.query_row("SELECT resultado FROM job WHERE id = ?1", [id], |r| r.get(0))?)
}

pub fn ids_do_edital(conn: &Connection, edital_id: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT id FROM job WHERE edital_id = ?1")?;
    let ids = stmt
        .query_map([edital_id], |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn codigo_str(code: ErrorCode) -> String {
    match serde_json::to_value(code) {
        Ok(serde_json::Value::String(s)) => s,
        _ => format!("{code:?}"),
    }
}

fn from_row(r: &Row) -> rusqlite::Result<Job> {
    let status: String = r.get(3)?;
    let erro_code: Option<String> = r.get(7)?;
    let custo: Option<f64> = r.get(16)?;
    let consumo = custo.map(|custo| -> rusqlite::Result<ConsumoJob> {
        let n = |i: usize| r.get::<_, Option<u32>>(i).map(Option::unwrap_or_default);
        Ok(ConsumoJob {
            modelo: r.get(10)?,
            tokens_entrada: n(11)?,
            tokens_saida: n(12)?,
            tokens_cache_escrita: n(13)?,
            tokens_cache_leitura: n(14)?,
            tokens_raciocinio: n(15)?,
            custo_estimado_usd: custo,
            duracao_ms: n(17)?,
        })
    });
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
        tentativas: r.get(9)?,
        consumo: consumo.transpose()?,
        utilizacao_5h: r.get(18)?,
        utilizacao_7d: r.get(19)?,
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

    #[test]
    fn fila_em_ordem_e_erro_com_codigo_do_catalogo() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        inserir(&conn, "b", None, "t", "2026-01-02").unwrap();
        inserir(&conn, "a", None, "t", "2026-01-01").unwrap();
        assert_eq!(proximo_na_fila(&conn).unwrap().unwrap().id, "a");
        assert_eq!(contar_na_fila(&conn).unwrap(), 2);

        let erro = AppError::new(ErrorCode::ClaudeSaidaInvalida, "x");
        finalizar(&conn, "a", JobStatus::Falhou, Some(&erro), "agora").unwrap();
        let a = get(&conn, "a").unwrap();
        assert_eq!(a.erro_code, Some(ErrorCode::ClaudeSaidaInvalida));
        assert!(a.consumo.is_none());

        let c = ConsumoJob {
            tokens_entrada: 10,
            custo_estimado_usd: 0.5,
            ..Default::default()
        };
        registrar_consumo(&conn, "b", &c, Some(0.3), None).unwrap();
        let b = get(&conn, "b").unwrap();
        assert_eq!(b.consumo, Some(c));
        assert_eq!(b.utilizacao_5h, Some(0.3));

        assert_eq!(cancelar_na_fila(&conn, None, "agora").unwrap(), 1);
        assert!(proximo_na_fila(&conn).unwrap().is_none());
    }
}
