use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::domain::normalize::normalizar;
use crate::domain::qualidade::{classificar_pagina, MAX_PAGINAS_RUINS};
use crate::domain::{
    Edital, EditalStatus, EditalUpdate, OrigemTexto, PaginaInput, PaginaProblema, QualidadeTexto,
};
use crate::error::{AppError, AppResult, ErrorCode};

const COLS: &str =
    "id, nome, orgao, banca, arquivo_original, sha256, total_paginas, status, importado_em";

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
    e.map(|e| com_paginas(conn, e)).transpose()
}

pub fn find_by_sha256(conn: &Connection, sha256: &str) -> AppResult<Option<Edital>> {
    let sql = format!("SELECT {COLS} FROM edital WHERE sha256 = ?1");
    let e = conn.query_row(&sql, [sha256], from_row).optional()?;
    e.map(|e| com_paginas(conn, e)).transpose()
}

pub fn listar(conn: &Connection) -> AppResult<Vec<Edital>> {
    let sql = format!("SELECT {COLS} FROM edital ORDER BY importado_em DESC");
    let mut stmt = conn.prepare(&sql)?;
    let editais = stmt
        .query_map([], from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    editais
        .into_iter()
        .map(|e| com_paginas(conn, e))
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
    let n = conn.execute(
        "UPDATE edital SET nome = ?2, orgao = ?3, banca = ?4 WHERE id = ?1",
        params![id, nome, limpa(&u.orgao), limpa(&u.banca)],
    )?;
    if n == 0 {
        return Err(AppError::new(ErrorCode::EditalNotFound, format!("edital {id}")));
    }
    get(conn, id)
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

/// Salva um lote de páginas e classifica a qualidade de cada uma. Reenviar a mesma
/// página substitui a anterior, exceto quando o OCR sai pior que o texto nativo.
/// Devolve as páginas do lote que precisam de OCR.
pub fn salvar_paginas(conn: &mut Connection, id: &str, paginas: &[PaginaInput]) -> AppResult<Vec<u32>> {
    let tx = conn.transaction()?;
    let mut precisam_ocr = Vec::new();
    {
        let mut atual = tx.prepare(
            "SELECT qualidade FROM pagina WHERE edital_id = ?1 AND numero = ?2",
        )?;
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO pagina
             (edital_id, numero, texto, itens, texto_normalizado, mapa_normalizado, sem_texto,
              qualidade, origem, confianca)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )?;
        for p in paginas {
            if p.pagina == 0 {
                return Err(AppError::new(ErrorCode::InvalidInput, "página 0"));
            }
            let qualidade = classificar_pagina(&p.texto, p.origem, p.confianca);
            if p.origem == OrigemTexto::Ocr {
                let anterior: Option<String> = atual
                    .query_row(params![id, p.pagina], |r| r.get(0))
                    .optional()?;
                let anterior = anterior.as_deref().and_then(QualidadeTexto::parse);
                if anterior.is_some_and(|a| a.nota() > qualidade.nota()) {
                    tracing::info!(edital = %id, pagina = p.pagina, ?qualidade, "OCR pior que o texto nativo; mantido o nativo");
                    continue;
                }
            } else if qualidade != QualidadeTexto::Ok {
                precisam_ocr.push(p.pagina);
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
                qualidade == QualidadeTexto::SemTexto,
                qualidade.as_str(),
                p.origem.as_str(),
                p.confianca,
            ])?;
        }
    }
    tx.commit()?;
    Ok(precisam_ocr)
}

/// Status do edital pela regra da spec: todas as páginas ruins bloqueiam.
fn status_por_qualidade(conn: &Connection, id: &str) -> AppResult<EditalStatus> {
    let (total, ok): (u32, u32) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(qualidade = 'ok'), 0) FROM pagina WHERE edital_id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(if total > 0 && ok == 0 {
        EditalStatus::SemTexto
    } else {
        EditalStatus::Importado
    })
}

/// Fecha a importação: confere se todas as páginas chegaram e define o status.
pub fn finalizar_importacao(conn: &Connection, id: &str, total_paginas: u32) -> AppResult<Edital> {
    let recebidas: u32 = conn.query_row(
        "SELECT COUNT(*) FROM pagina WHERE edital_id = ?1 AND numero BETWEEN 1 AND ?2",
        params![id, total_paginas],
        |r| r.get(0),
    )?;
    if total_paginas == 0 || recebidas != total_paginas {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            format!("esperadas {total_paginas} páginas, recebidas {recebidas}"),
        ));
    }
    let status = status_por_qualidade(conn, id)?;
    conn.execute(
        "UPDATE edital SET total_paginas = ?2, status = ?3 WHERE id = ?1",
        params![id, total_paginas, status.as_str()],
    )?;
    get(conn, id)
}

/// Reclassifica todas as páginas já salvas (pós-migração 003) e atualiza o status
/// dos editais já importados. Devolve quantas páginas mudaram.
pub fn reclassificar_paginas(conn: &Connection) -> AppResult<usize> {
    let paginas: Vec<(String, u32, String, String, Option<f32>)> = {
        let mut stmt =
            conn.prepare("SELECT edital_id, numero, texto, origem, confianca FROM pagina")?;
        let v = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        v
    };
    let mut mudaram = 0;
    for (edital, numero, texto, origem, confianca) in &paginas {
        let origem = OrigemTexto::parse(origem).unwrap_or_default();
        let q = classificar_pagina(texto, origem, *confianca);
        mudaram += conn.execute(
            "UPDATE pagina SET qualidade = ?3, sem_texto = ?4
             WHERE edital_id = ?1 AND numero = ?2 AND qualidade <> ?3",
            params![edital, numero, q.as_str(), q == QualidadeTexto::SemTexto],
        )?;
    }
    let editais: Vec<String> = {
        let mut stmt = conn.prepare("SELECT id FROM edital WHERE status <> 'importando'")?;
        let v = stmt.query_map([], |r| r.get(0))?.collect::<Result<Vec<_>, _>>()?;
        v
    };
    for id in editais {
        let status = status_por_qualidade(conn, &id)?;
        conn.execute("UPDATE edital SET status = ?2 WHERE id = ?1", params![id, status.as_str()])?;
    }
    Ok(mudaram)
}

fn com_paginas(conn: &Connection, mut e: Edital) -> AppResult<Edital> {
    let mut stmt = conn.prepare_cached(
        "SELECT numero, qualidade, origem FROM pagina WHERE edital_id = ?1 ORDER BY numero",
    )?;
    let linhas = stmt
        .query_map([&e.id], |r| {
            Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    e.paginas_problema = linhas
        .iter()
        .filter_map(|(n, q, _)| {
            QualidadeTexto::parse(q)
                .filter(|q| *q != QualidadeTexto::Ok)
                .map(|qualidade| PaginaProblema { pagina: *n, qualidade })
        })
        .collect();
    e.paginas_ocr = linhas
        .iter()
        .filter(|(_, _, o)| o == "ocr")
        .map(|(n, _, _)| *n)
        .collect();
    e.exige_confirmacao = e.status == EditalStatus::Importado
        && !linhas.is_empty()
        && e.paginas_problema.len() as f64 / linhas.len() as f64 > MAX_PAGINAS_RUINS;
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
        paginas_problema: Vec::new(),
        paginas_ocr: Vec::new(),
        exige_confirmacao: false,
        status: EditalStatus::parse(&status).unwrap_or(EditalStatus::Importando),
        importado_em: r.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ItemTexto;
    use crate::storage::Db;

    const LEGIVEL: &str =
        "O concurso público será regido por este edital e executado pela banca organizadora.";

    fn pagina(n: u32, texto: &str) -> PaginaInput {
        PaginaInput {
            pagina: n,
            texto: texto.into(),
            itens: vec![ItemTexto {
                t: texto.into(),
                i: 0,
                b: None,
            }],
            origem: OrigemTexto::Pdf,
            confianca: None,
        }
    }

    fn ocr(n: u32, texto: &str, confianca: f32) -> PaginaInput {
        PaginaInput {
            origem: OrigemTexto::Ocr,
            confianca: Some(confianca),
            ..pagina(n, texto)
        }
    }

    fn problema(pagina: u32, qualidade: QualidadeTexto) -> PaginaProblema {
        PaginaProblema { pagina, qualidade }
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
        let ocr1 = salvar_paginas(&mut conn, "e1", &[pagina(1, LEGIVEL), pagina(2, " \n ")]).unwrap();
        let ocr2 = salvar_paginas(&mut conn, "e1", &[pagina(3, LEGIVEL)]).unwrap();
        assert_eq!((ocr1, ocr2), (vec![2], vec![]));
        let e = finalizar_importacao(&conn, "e1", 3).unwrap();
        assert_eq!(e.status, EditalStatus::Importado);
        assert_eq!(e.paginas_problema, vec![problema(2, QualidadeTexto::SemTexto)]);
        assert_eq!(e.total_paginas, Some(3));
        // 1 de 3 páginas ruins passa de 20%: processar pede confirmação.
        assert!(e.exige_confirmacao);
    }

    #[test]
    fn ocr_substitui_pagina_ruim() {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        novo(&conn, "e1");
        let ilegivel = LEGIVEL.replace('ç', "Ã§").replace('ú', "Ãº").replace('á', "Ã¡");
        let precisam = salvar_paginas(&mut conn, "e1", &[pagina(1, &ilegivel)]).unwrap();
        assert_eq!(precisam, vec![1]);
        let precisam = salvar_paginas(&mut conn, "e1", &[ocr(1, LEGIVEL, 92.0)]).unwrap();
        assert!(precisam.is_empty(), "OCR nunca pede outro OCR");
        let e = finalizar_importacao(&conn, "e1", 1).unwrap();
        assert_eq!(e.status, EditalStatus::Importado);
        assert!(e.paginas_problema.is_empty());
        assert_eq!(e.paginas_ocr, vec![1]);
        assert!(!e.exige_confirmacao);
    }

    #[test]
    fn ocr_pior_que_o_nativo_e_descartado() {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        novo(&conn, "e1");
        let ilegivel = LEGIVEL.replace('e', "\u{FFFD}");
        salvar_paginas(&mut conn, "e1", &[pagina(1, &ilegivel)]).unwrap();
        // OCR não achou nada: sem_texto é pior que suspeito.
        salvar_paginas(&mut conn, "e1", &[ocr(1, "", 0.0)]).unwrap();
        let e = finalizar_importacao(&conn, "e1", 1).unwrap();
        assert_eq!(e.paginas_problema, vec![problema(1, QualidadeTexto::Suspeito)]);
        assert!(e.paginas_ocr.is_empty());
        assert_eq!(e.status, EditalStatus::SemTexto);
    }

    #[test]
    fn ocr_com_confianca_baixa_continua_problema() {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        novo(&conn, "e1");
        salvar_paginas(&mut conn, "e1", &[pagina(1, LEGIVEL), pagina(2, "")]).unwrap();
        salvar_paginas(&mut conn, "e1", &[ocr(2, LEGIVEL, 40.0)]).unwrap();
        let e = finalizar_importacao(&conn, "e1", 2).unwrap();
        assert_eq!(e.paginas_problema, vec![problema(2, QualidadeTexto::Suspeito)]);
        assert_eq!(e.paginas_ocr, vec![2]);
    }

    #[test]
    fn reclassifica_paginas_antigas() {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        novo(&conn, "e1");
        salvar_paginas(&mut conn, "e1", &[pagina(1, LEGIVEL)]).unwrap();
        finalizar_importacao(&conn, "e1", 1).unwrap();
        // Página gravada antes da migração 003: texto ilegível, mas marcada como ok.
        let ilegivel = "\u{FFFD}".repeat(30);
        conn.execute("UPDATE pagina SET texto = ?1, qualidade = 'ok'", [&ilegivel])
            .unwrap();
        assert_eq!(reclassificar_paginas(&conn).unwrap(), 1);
        assert_eq!(get(&conn, "e1").unwrap().status, EditalStatus::SemTexto);
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
            },
        )
        .unwrap();
        assert_eq!(e.nome, "TCE-SP 2026");
        assert_eq!(e.orgao.as_deref(), Some("TCE-SP"));
        assert_eq!(e.banca, None);
    }
}
