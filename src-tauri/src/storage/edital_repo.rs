use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::domain::normalize::normalizar;
use crate::domain::{Edital, EditalStatus, EditalUpdate, PaginaInput};
use crate::error::{AppError, AppResult, ErrorCode};

const COLS: &str = "id, nome, orgao, banca, arquivo_original, sha256, total_paginas, status, importado_em,
     numero_edital, data_prova, inscricoes_inicio, inscricoes_fim, metadados_editados";

pub struct NovoEdital<'a> {
    pub id: &'a str,
    pub nome: &'a str,
    pub arquivo_original: &'a str,
    pub sha256: &'a str,
    pub importado_em: &'a str,
}

pub fn inserir(conn: &Connection, e: &NovoEdital) -> AppResult<()> {
    conn.execute(
        "INSERT INTO edital (id, nome, arquivo_original, sha256, status, importado_em)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            e.id,
            e.nome,
            e.arquivo_original,
            e.sha256,
            EditalStatus::Importando.as_str(),
            e.importado_em
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Edital> {
    find(conn, id)?
        .ok_or_else(|| AppError::new(ErrorCode::EditalNotFound, format!("edital {id}")))
}

pub fn find(conn: &Connection, id: &str) -> AppResult<Option<Edital>> {
    let sql = format!("SELECT {COLS} FROM edital WHERE id = ?1");
    let e = conn.query_row(&sql, [id], from_row).optional()?;
    e.map(|e| com_paginas_sem_texto(conn, e)).transpose()
}

pub fn find_by_sha256(conn: &Connection, sha256: &str) -> AppResult<Option<Edital>> {
    let sql = format!("SELECT {COLS} FROM edital WHERE sha256 = ?1");
    let e = conn.query_row(&sql, [sha256], from_row).optional()?;
    e.map(|e| com_paginas_sem_texto(conn, e)).transpose()
}

pub fn listar(conn: &Connection) -> AppResult<Vec<Edital>> {
    let sql = format!("SELECT {COLS} FROM edital ORDER BY importado_em DESC");
    let mut stmt = conn.prepare(&sql)?;
    let editais = stmt
        .query_map([], from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    editais
        .into_iter()
        .map(|e| com_paginas_sem_texto(conn, e))
        .collect()
}

pub fn atualizar(conn: &Connection, id: &str, u: &EditalUpdate) -> AppResult<Edital> {
    let nome = u.nome.trim();
    if nome.is_empty() {
        return Err(AppError::new(ErrorCode::InvalidInput, "nome vazio"));
    }
    let limpa = |s: &Option<String>| {
        s.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let data_prova = limpa(&u.data_prova);
    if data_prova.as_deref().is_some_and(|d| !data_valida(d)) {
        return Err(AppError::new(ErrorCode::InvalidInput, "data da prova deve ser AAAA-MM-DD"));
    }
    let n = conn.execute(
        "UPDATE edital SET nome = ?2, orgao = ?3, banca = ?4, numero_edital = ?5, data_prova = ?6,
         metadados_editados = 1 WHERE id = ?1",
        params![id, nome, limpa(&u.orgao), limpa(&u.banca), limpa(&u.numero_edital), data_prova],
    )?;
    if n == 0 {
        return Err(AppError::new(ErrorCode::EditalNotFound, format!("edital {id}")));
    }
    get(conn, id)
}

/// Metadados lidos do edital pela análise. Campos `None` não mudam.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MetadadosLidos {
    pub nome: Option<String>,
    pub orgao: Option<String>,
    pub banca: Option<String>,
    pub numero_edital: Option<String>,
    pub data_prova: Option<String>,
    pub inscricoes_inicio: Option<String>,
    pub inscricoes_fim: Option<String>,
}

/// Aplica os metadados da análise, a menos que o usuário já tenha editado o cabeçalho.
pub fn aplicar_metadados(conn: &Connection, id: &str, m: &MetadadosLidos) -> AppResult<bool> {
    let n = conn.execute(
        "UPDATE edital SET
            nome = COALESCE(?2, nome), orgao = COALESCE(?3, orgao), banca = COALESCE(?4, banca),
            numero_edital = COALESCE(?5, numero_edital), data_prova = COALESCE(?6, data_prova),
            inscricoes_inicio = COALESCE(?7, inscricoes_inicio), inscricoes_fim = COALESCE(?8, inscricoes_fim)
         WHERE id = ?1 AND metadados_editados = 0",
        params![
            id,
            m.nome,
            m.orgao,
            m.banca,
            m.numero_edital,
            m.data_prova,
            m.inscricoes_inicio,
            m.inscricoes_fim
        ],
    )?;
    Ok(n > 0)
}

/// `AAAA-MM-DD` com mês e dia plausíveis.
pub fn data_valida(s: &str) -> bool {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

pub fn remover(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM edital WHERE id = ?1", [id])?;
    Ok(())
}

/// Descarta páginas de uma importação anterior que não terminou.
pub fn limpar_paginas(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM pagina WHERE edital_id = ?1", [id])?;
    Ok(())
}

/// Salva um lote de páginas. Reenviar a mesma página substitui a anterior.
pub fn salvar_paginas(conn: &mut Connection, id: &str, paginas: &[PaginaInput]) -> AppResult<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO pagina
             (edital_id, numero, texto, itens, texto_normalizado, mapa_normalizado, sem_texto)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for p in paginas {
            if p.pagina == 0 {
                return Err(AppError::new(ErrorCode::InvalidInput, "página 0"));
            }
            let n = normalizar(&p.texto);
            let mapa: Vec<u8> = n.mapa.iter().flat_map(|o| o.to_le_bytes()).collect();
            stmt.execute(params![
                id,
                p.pagina,
                p.texto,
                serde_json::to_string(&p.itens)?,
                n.texto,
                mapa,
                n.texto.is_empty(),
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Fecha a importação: confere se todas as páginas chegaram e define o status.
pub fn finalizar_importacao(conn: &Connection, id: &str, total_paginas: u32) -> AppResult<Edital> {
    let (recebidas, com_texto): (u32, u32) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(sem_texto = 0), 0) FROM pagina
         WHERE edital_id = ?1 AND numero BETWEEN 1 AND ?2",
        params![id, total_paginas],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if total_paginas == 0 || recebidas != total_paginas {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            format!("esperadas {total_paginas} páginas, recebidas {recebidas}"),
        ));
    }
    let status = if com_texto == 0 {
        EditalStatus::SemTexto
    } else {
        EditalStatus::Importado
    };
    conn.execute(
        "UPDATE edital SET total_paginas = ?2, status = ?3 WHERE id = ?1",
        params![id, total_paginas, status.as_str()],
    )?;
    get(conn, id)
}

fn com_paginas_sem_texto(conn: &Connection, mut e: Edital) -> AppResult<Edital> {
    let mut stmt = conn.prepare_cached(
        "SELECT numero FROM pagina WHERE edital_id = ?1 AND sem_texto = 1 ORDER BY numero",
    )?;
    e.paginas_sem_texto = stmt
        .query_map([&e.id], |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(e)
}

fn from_row(r: &Row) -> rusqlite::Result<Edital> {
    let status: String = r.get(7)?;
    Ok(Edital {
        id: r.get(0)?,
        nome: r.get(1)?,
        orgao: r.get(2)?,
        banca: r.get(3)?,
        arquivo_original: r.get(4)?,
        sha256: r.get(5)?,
        total_paginas: r.get(6)?,
        paginas_sem_texto: Vec::new(),
        status: EditalStatus::parse(&status).unwrap_or(EditalStatus::Importando),
        importado_em: r.get(8)?,
        numero_edital: r.get(9)?,
        data_prova: r.get(10)?,
        inscricoes_inicio: r.get(11)?,
        inscricoes_fim: r.get(12)?,
        metadados_editados: r.get(13)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ItemTexto;
    use crate::storage::Db;

    fn pagina(n: u32, texto: &str) -> PaginaInput {
        PaginaInput {
            pagina: n,
            texto: texto.into(),
            itens: vec![ItemTexto {
                t: texto.into(),
                i: 0,
            }],
        }
    }

    fn novo(conn: &Connection, id: &str) {
        inserir(
            conn,
            &NovoEdital {
                id,
                nome: "Edital",
                arquivo_original: "edital.pdf",
                sha256: id,
                importado_em: "2026-10-06T00:00:00Z",
            },
        )
        .unwrap();
    }

    #[test]
    fn importa_com_paginas_sem_texto() {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        novo(&conn, "e1");
        salvar_paginas(&mut conn, "e1", &[pagina(1, "Conteúdo"), pagina(2, " \n ")]).unwrap();
        salvar_paginas(&mut conn, "e1", &[pagina(3, "Anexo")]).unwrap();
        let e = finalizar_importacao(&conn, "e1", 3).unwrap();
        assert_eq!(e.status, EditalStatus::Importado);
        assert_eq!(e.paginas_sem_texto, vec![2]);
        assert_eq!(e.total_paginas, Some(3));
    }

    #[test]
    fn sem_nenhum_texto_fica_sem_texto() {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        novo(&conn, "e1");
        salvar_paginas(&mut conn, "e1", &[pagina(1, ""), pagina(2, "  ")]).unwrap();
        let e = finalizar_importacao(&conn, "e1", 2).unwrap();
        assert_eq!(e.status, EditalStatus::SemTexto);
    }

    #[test]
    fn recusa_finalizar_com_paginas_faltando() {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        novo(&conn, "e1");
        salvar_paginas(&mut conn, "e1", &[pagina(1, "a")]).unwrap();
        let err = finalizar_importacao(&conn, "e1", 2).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn remover_apaga_paginas_em_cascata() {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        novo(&conn, "e1");
        salvar_paginas(&mut conn, "e1", &[pagina(1, "a")]).unwrap();
        remover(&conn, "e1").unwrap();
        let n: u32 = conn
            .query_row("SELECT COUNT(*) FROM pagina", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn atualiza_metadados() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        novo(&conn, "e1");
        let e = atualizar(
            &conn,
            "e1",
            &EditalUpdate {
                nome: " TCE-SP 2026 ".into(),
                orgao: Some("TCE-SP".into()),
                banca: Some("  ".into()),
                numero_edital: None,
                data_prova: Some("2027-03-14".into()),
            },
        )
        .unwrap();
        assert_eq!(e.nome, "TCE-SP 2026");
        assert_eq!(e.orgao.as_deref(), Some("TCE-SP"));
        assert_eq!(e.banca, None);
        assert_eq!(e.data_prova.as_deref(), Some("2027-03-14"));
        assert!(e.metadados_editados);
    }

    #[test]
    fn analise_preenche_metadados_so_sem_edicao() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        novo(&conn, "e1");
        let lidos = MetadadosLidos {
            nome: Some("Concurso Público nº 1/2026".into()),
            banca: Some("VUNESP".into()),
            ..Default::default()
        };
        assert!(aplicar_metadados(&conn, "e1", &lidos).unwrap());
        let e = get(&conn, "e1").unwrap();
        assert_eq!(e.nome, "Concurso Público nº 1/2026");
        assert_eq!(e.banca.as_deref(), Some("VUNESP"));
        assert_eq!(e.orgao, None);

        atualizar(
            &conn,
            "e1",
            &EditalUpdate {
                nome: "Meu nome".into(),
                orgao: None,
                banca: None,
                numero_edital: None,
                data_prova: None,
            },
        )
        .unwrap();
        assert!(!aplicar_metadados(&conn, "e1", &lidos).unwrap());
        assert_eq!(get(&conn, "e1").unwrap().nome, "Meu nome");
    }
}
