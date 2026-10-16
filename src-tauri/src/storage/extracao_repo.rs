use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::domain::{Bloco, Categoria, EstadoExtracao, Extracao, SecaoEdital, StatusExtracao};
use crate::error::{AppError, AppResult};
use crate::extracao::mapa::Secao;

/// Texto original de cada página, em ordem.
pub fn paginas(conn: &Connection, edital_id: &str) -> AppResult<Vec<(u32, String)>> {
    let mut stmt = conn.prepare("SELECT numero, texto FROM pagina WHERE edital_id = ?1 ORDER BY numero")?;
    let v = stmt
        .query_map([edital_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

/// Substitui o mapa do edital e zera o resultado da triagem anterior.
pub fn salvar_mapa(conn: &mut Connection, edital_id: &str, secoes: &[Secao]) -> AppResult<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM secao WHERE edital_id = ?1", [edital_id])?;
    tx.execute(
        "UPDATE edital SET e_edital = NULL, triagem_motivo = NULL WHERE id = ?1",
        [edital_id],
    )?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO secao (edital_id, ordem, titulo, pagina_ini, byte_ini, pagina_fim, byte_fim)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for s in secoes {
            stmt.execute(params![
                edital_id, s.ordem, s.titulo, s.pagina_ini, s.byte_ini, s.pagina_fim, s.byte_fim
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn secoes(conn: &Connection, edital_id: &str) -> AppResult<Vec<Secao>> {
    let mut stmt = conn.prepare(
        "SELECT ordem, titulo, pagina_ini, byte_ini, pagina_fim, byte_fim FROM secao
         WHERE edital_id = ?1 ORDER BY ordem",
    )?;
    let v = stmt
        .query_map([edital_id], |r| {
            Ok(Secao {
                ordem: r.get(0)?,
                titulo: r.get(1)?,
                pagina_ini: r.get(2)?,
                byte_ini: r.get(3)?,
                pagina_fim: r.get(4)?,
                byte_fim: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn salvar_categorias(conn: &Connection, edital_id: &str, ordem: u32, c: &[Categoria]) -> AppResult<()> {
    conn.execute(
        "UPDATE secao SET categorias = ?3 WHERE edital_id = ?1 AND ordem = ?2",
        params![edital_id, ordem, serde_json::to_string(c)?],
    )?;
    Ok(())
}

pub fn salvar_triagem(conn: &Connection, edital_id: &str, e_edital: bool, motivo: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE edital SET e_edital = ?2, triagem_motivo = ?3 WHERE id = ?1",
        params![edital_id, e_edital, motivo],
    )?;
    Ok(())
}

/// Cria a extração (status `pendente`) e devolve o id.
pub fn inserir(
    conn: &Connection,
    edital_id: &str,
    bloco: Bloco,
    cargo_id: Option<&str>,
    job_id: &str,
    versao: u32,
    agora: &str,
) -> AppResult<String> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO extracao (id, edital_id, bloco, cargo_id, job_id, versao, status, criado_em, atualizado_em)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            id,
            edital_id,
            bloco.as_str(),
            cargo_id,
            job_id,
            versao,
            StatusExtracao::Pendente.as_str(),
            agora
        ],
    )?;
    Ok(id)
}

/// Extração ligada a um job, com o id do edital.
pub fn por_job(conn: &Connection, job_id: &str) -> AppResult<Option<(Extracao, String)>> {
    let sql = format!("SELECT {COLS}, edital_id FROM extracao WHERE job_id = ?1");
    Ok(conn
        .query_row(&sql, [job_id], |r| Ok((from_row(r)?, r.get(9)?)))
        .optional()?)
}

pub fn finalizar(
    conn: &Connection,
    id: &str,
    status: StatusExtracao,
    erro: Option<&AppError>,
    agora: &str,
) -> AppResult<()> {
    let code = erro.and_then(|e| serde_json::to_value(e.code).ok()).and_then(|v| v.as_str().map(str::to_owned));
    conn.execute(
        "UPDATE extracao SET status = ?2, erro_code = ?3, erro_detail = ?4, atualizado_em = ?5 WHERE id = ?1",
        params![id, status.as_str(), code, erro.map(|e| &e.detail), agora],
    )?;
    Ok(())
}

/// Na abertura do app: extração pendente cujo job não está mais na fila nem
/// executando (app fechado no meio) vira `falhou` com `JOB_INTERROMPIDO`.
pub fn reconciliar(conn: &Connection, agora: &str) -> AppResult<usize> {
    Ok(conn.execute(
        "UPDATE extracao SET status = 'falhou', erro_code = 'JOB_INTERROMPIDO', atualizado_em = ?1
         WHERE status = 'pendente'
           AND (job_id IS NULL OR job_id NOT IN (SELECT id FROM job WHERE status IN ('na_fila', 'executando')))",
        [agora],
    )?)
}

pub fn pendente(conn: &Connection, edital_id: &str, bloco: Bloco) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM extracao WHERE edital_id = ?1 AND bloco = ?2 AND status = 'pendente')",
        params![edital_id, bloco.as_str()],
        |r| r.get(0),
    )?)
}

/// Tudo o que a tela do Edital mostra sobre o processamento.
pub fn estado(conn: &Connection, edital_id: &str) -> AppResult<EstadoExtracao> {
    let (e_edital, triagem_motivo): (Option<bool>, Option<String>) = conn.query_row(
        "SELECT e_edital, triagem_motivo FROM edital WHERE id = ?1",
        [edital_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let secoes = {
        let mut stmt = conn.prepare(
            "SELECT ordem, titulo, pagina_ini, pagina_fim, categorias FROM secao
             WHERE edital_id = ?1 ORDER BY ordem",
        )?;
        let v = stmt
            .query_map([edital_id], |r| {
                let categorias: Option<String> = r.get(4)?;
                Ok(SecaoEdital {
                    ordem: r.get(0)?,
                    titulo: r.get(1)?,
                    pagina_ini: r.get(2)?,
                    pagina_fim: r.get(3)?,
                    categorias: categorias.and_then(|c| serde_json::from_str(&c).ok()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        v
    };
    let extracoes = {
        let sql = format!("SELECT {COLS} FROM extracao WHERE edital_id = ?1 ORDER BY criado_em DESC, rowid DESC");
        let mut stmt = conn.prepare(&sql)?;
        let v = stmt.query_map([edital_id], from_row)?.collect::<Result<Vec<_>, _>>()?;
        v
    };
    Ok(EstadoExtracao {
        edital_id: edital_id.to_owned(),
        e_edital,
        triagem_motivo,
        secoes,
        extracoes,
    })
}

const COLS: &str = "id, bloco, cargo_id, job_id, versao, status, erro_code, erro_detail, atualizado_em";

fn from_row(r: &Row) -> rusqlite::Result<Extracao> {
    let bloco: String = r.get(1)?;
    let status: String = r.get(5)?;
    let erro_code: Option<String> = r.get(6)?;
    Ok(Extracao {
        id: r.get(0)?,
        bloco: Bloco::parse(&bloco).unwrap_or(Bloco::Triagem),
        cargo_id: r.get(2)?,
        job_id: r.get(3)?,
        versao: r.get(4)?,
        status: StatusExtracao::parse(&status).unwrap_or(StatusExtracao::Falhou),
        erro_code: erro_code.and_then(|c| serde_json::from_value(c.into()).ok()),
        erro_detail: r.get(7)?,
        atualizado_em: r.get(8)?,
    })
}
