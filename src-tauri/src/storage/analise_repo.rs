use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::{Analise, Cargo, Materia, Topico};
use crate::error::{AppError, AppResult, ErrorCode};

/// Texto de uma página como foi salvo na importação.
pub struct PaginaTexto {
    pub numero: u32,
    pub texto: String,
    pub sem_texto: bool,
}

/// Página pronta para a verificação literal.
pub struct PaginaBusca {
    pub numero: u32,
    pub normalizado: String,
    /// Ver `domain::normalize`: offset UTF-16 no original de cada char do normalizado (+1 no fim).
    pub mapa: Vec<u32>,
}

pub fn paginas_texto(conn: &Connection, edital_id: &str) -> AppResult<Vec<PaginaTexto>> {
    let mut stmt = conn
        .prepare("SELECT numero, texto, sem_texto FROM pagina WHERE edital_id = ?1 ORDER BY numero")?;
    let v = stmt
        .query_map([edital_id], |r| {
            Ok(PaginaTexto {
                numero: r.get(0)?,
                texto: r.get(1)?,
                sem_texto: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn paginas_busca(conn: &Connection, edital_id: &str) -> AppResult<Vec<PaginaBusca>> {
    let mut stmt = conn.prepare(
        "SELECT numero, texto_normalizado, mapa_normalizado FROM pagina
         WHERE edital_id = ?1 AND sem_texto = 0 ORDER BY numero",
    )?;
    let v = stmt
        .query_map([edital_id], |r| {
            let mapa: Vec<u8> = r.get(2)?;
            Ok(PaginaBusca {
                numero: r.get(0)?,
                normalizado: r.get(1)?,
                mapa: mapa
                    .chunks_exact(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

/// Substitui os cargos do edital. Cargo escolhido e conteúdo anteriores são descartados.
pub fn substituir_cargos(conn: &mut Connection, edital_id: &str, cargos: &[Cargo]) -> AppResult<()> {
    let tx = conn.transaction()?;
    tx.execute("UPDATE edital SET cargo_escolhido = NULL WHERE id = ?1", [edital_id])?;
    tx.execute("DELETE FROM cargo WHERE edital_id = ?1", [edital_id])?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO cargo (id, edital_id, ordem, nome, vagas, escolaridade, pagina, verificado)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        for (i, c) in cargos.iter().enumerate() {
            stmt.execute(params![
                c.id,
                edital_id,
                i as u32,
                c.nome,
                c.vagas,
                c.escolaridade,
                c.pagina,
                c.verificado
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn escolher_cargo(conn: &Connection, edital_id: &str, cargo_id: &str) -> AppResult<()> {
    let existe: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM cargo WHERE id = ?1 AND edital_id = ?2)",
        [cargo_id, edital_id],
        |r| r.get(0),
    )?;
    if !existe {
        return Err(AppError::new(ErrorCode::InvalidInput, format!("cargo {cargo_id}")));
    }
    conn.execute(
        "UPDATE edital SET cargo_escolhido = ?2 WHERE id = ?1",
        [edital_id, cargo_id],
    )?;
    Ok(())
}

pub fn cargo_escolhido(conn: &Connection, edital_id: &str) -> AppResult<Option<Cargo>> {
    let id: Option<String> = conn
        .query_row("SELECT cargo_escolhido FROM edital WHERE id = ?1", [edital_id], |r| r.get(0))
        .optional()?
        .flatten();
    match id {
        Some(id) => Ok(cargos(conn, edital_id)?.into_iter().find(|c| c.id == id)),
        None => Ok(None),
    }
}

/// Substitui o conteúdo programático de um cargo.
pub fn substituir_conteudo(conn: &mut Connection, cargo_id: &str, materias: &[Materia]) -> AppResult<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM materia WHERE cargo_id = ?1", [cargo_id])?;
    {
        let mut m_stmt = tx.prepare(
            "INSERT INTO materia (id, cargo_id, ordem, nome, pagina) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        let mut t_stmt = tx.prepare(
            "INSERT INTO topico (id, materia_id, pai_id, ordem, texto, pagina, inicio, fim, verificado)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        for (i, m) in materias.iter().enumerate() {
            m_stmt.execute(params![m.id, cargo_id, i as u32, m.nome, m.pagina])?;
            for (j, t) in m.topicos.iter().enumerate() {
                let mut inserir = |t: &Topico, pai: Option<&str>, ordem: usize| {
                    t_stmt.execute(params![
                        t.id,
                        m.id,
                        pai,
                        ordem as u32,
                        t.texto,
                        t.pagina,
                        t.inicio,
                        t.fim,
                        t.verificado
                    ])
                };
                inserir(t, None, j)?;
                for (k, f) in t.filhos.iter().enumerate() {
                    inserir(f, Some(&t.id), k)?;
                }
            }
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn cargos(conn: &Connection, edital_id: &str) -> AppResult<Vec<Cargo>> {
    let mut stmt = conn.prepare(
        "SELECT id, nome, vagas, escolaridade, pagina, verificado FROM cargo
         WHERE edital_id = ?1 ORDER BY ordem",
    )?;
    let v = stmt
        .query_map([edital_id], |r| {
            Ok(Cargo {
                id: r.get(0)?,
                nome: r.get(1)?,
                vagas: r.get(2)?,
                escolaridade: r.get(3)?,
                pagina: r.get(4)?,
                verificado: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

/// `None` quando o cargo ainda não teve o conteúdo extraído.
pub fn materias(conn: &Connection, cargo_id: &str) -> AppResult<Option<Vec<Materia>>> {
    let mut stmt =
        conn.prepare("SELECT id, nome, pagina FROM materia WHERE cargo_id = ?1 ORDER BY ordem")?;
    let mut materias = stmt
        .query_map([cargo_id], |r| {
            Ok(Materia {
                id: r.get(0)?,
                nome: r.get(1)?,
                pagina: r.get(2)?,
                topicos: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if materias.is_empty() {
        return Ok(None);
    }
    let mut t_stmt = conn.prepare(
        "SELECT id, pai_id, texto, pagina, inicio, fim, verificado FROM topico
         WHERE materia_id = ?1 ORDER BY pai_id IS NOT NULL, ordem",
    )?;
    for m in &mut materias {
        let linhas = t_stmt
            .query_map([&m.id], |r| {
                Ok((
                    r.get::<_, Option<String>>(1)?,
                    Topico {
                        id: r.get(0)?,
                        texto: r.get(2)?,
                        pagina: r.get(3)?,
                        verificado: r.get(6)?,
                        inicio: r.get(4)?,
                        fim: r.get(5)?,
                        filhos: Vec::new(),
                    },
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        // Pais vêm antes dos filhos (ORDER BY pai_id IS NOT NULL).
        for (pai, t) in linhas {
            match pai.and_then(|p| m.topicos.iter_mut().find(|x| x.id == p)) {
                Some(p) => p.filhos.push(t),
                None => m.topicos.push(t),
            }
        }
    }
    Ok(Some(materias))
}

/// Cargos e conteúdo do edital, sem o andamento (que vem dos jobs).
pub fn carregar(conn: &Connection, edital_id: &str) -> AppResult<Analise> {
    let cargos = cargos(conn, edital_id)?;
    let escolhido = cargo_escolhido(conn, edital_id)?;
    let materias = match &escolhido {
        Some(c) => materias(conn, &c.id)?,
        None => None,
    };
    Ok(Analise {
        cargos,
        cargo_escolhido: escolhido.map(|c| c.id),
        materias,
        andamento: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::edital_repo::{inserir, NovoEdital};
    use crate::storage::Db;

    fn cargo(id: &str) -> Cargo {
        Cargo {
            id: id.into(),
            nome: format!("Cargo {id}"),
            vagas: Some(3),
            escolaridade: None,
            pagina: Some(2),
            verificado: true,
        }
    }

    fn topico(id: &str, filhos: Vec<Topico>) -> Topico {
        Topico {
            id: id.into(),
            texto: id.into(),
            pagina: Some(5),
            verificado: true,
            inicio: Some(1),
            fim: Some(4),
            filhos,
        }
    }

    #[test]
    fn grava_e_le_analise_completa() {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        inserir(
            &conn,
            &NovoEdital {
                id: "e1",
                nome: "E",
                arquivo_original: "e.pdf",
                sha256: "h",
                importado_em: "x",
            },
        )
        .unwrap();
        substituir_cargos(&mut conn, "e1", &[cargo("c1"), cargo("c2")]).unwrap();
        assert!(carregar(&conn, "e1").unwrap().materias.is_none());

        escolher_cargo(&conn, "e1", "c2").unwrap();
        assert!(escolher_cargo(&conn, "e1", "nao-existe").is_err());
        let m = Materia {
            id: "m1".into(),
            nome: "Português".into(),
            pagina: Some(5),
            topicos: vec![
                topico("t1", vec![topico("t1a", vec![]), topico("t1b", vec![])]),
                topico("t2", vec![]),
            ],
        };
        substituir_conteudo(&mut conn, "c2", &[m.clone()]).unwrap();

        let a = carregar(&conn, "e1").unwrap();
        assert_eq!(a.cargos.len(), 2);
        assert_eq!(a.cargo_escolhido.as_deref(), Some("c2"));
        assert_eq!(a.materias.unwrap(), vec![m]);

        // Nova localização zera escolha e conteúdo.
        substituir_cargos(&mut conn, "e1", &[cargo("c3")]).unwrap();
        let a = carregar(&conn, "e1").unwrap();
        assert_eq!(a.cargo_escolhido, None);
        let n: u32 = conn.query_row("SELECT COUNT(*) FROM topico", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }
}
