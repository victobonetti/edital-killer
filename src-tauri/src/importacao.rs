//! Importação do edital: validar, copiar e registrar o PDF. O texto das páginas
//! chega depois, extraído pelo pdf.js na UI (`storage::edital_repo::salvar_paginas`).
//! Nenhuma chamada ao Claude acontece aqui.

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::domain::{EditalStatus, ImportStart};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::storage::edital_repo::{self, NovoEdital};
use crate::storage::{job_repo, AppPaths, Db};

/// A especificação do PDF tolera lixo antes do cabeçalho; leitores aceitam até 1024 bytes.
const JANELA_CABECALHO: usize = 1024;

pub fn tem_cabecalho_pdf(bytes: &[u8]) -> bool {
    let janela = &bytes[..bytes.len().min(JANELA_CABECALHO)];
    janela.windows(5).any(|w| w == b"%PDF-")
}

pub fn iniciar(db: &Db, paths: &AppPaths, origem: &Path, agora: &str) -> AppResult<ImportStart> {
    let bytes = fs::read(origem)?;
    if !tem_cabecalho_pdf(&bytes) {
        return Err(AppError::new(
            ErrorCode::ImportNotPdf,
            origem.display().to_string(),
        ));
    }
    let sha256 = hex::encode(Sha256::digest(&bytes));

    let conn = db.conn()?;
    if let Some(existente) = edital_repo::find_by_sha256(&conn, &sha256)? {
        if existente.status != EditalStatus::Importando {
            return Ok(ImportStart {
                edital: existente,
                duplicado: true,
            });
        }
        // Importação anterior do mesmo PDF não terminou: recomeça a extração.
        edital_repo::limpar_paginas(&conn, &existente.id)?;
        gravar_copia(paths, &existente.id, &bytes)?;
        return Ok(ImportStart {
            edital: existente,
            duplicado: false,
        });
    }

    let id = uuid::Uuid::new_v4().to_string();
    let arquivo = origem
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "edital.pdf".into());
    let nome = Path::new(&arquivo)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| arquivo.clone());

    gravar_copia(paths, &id, &bytes)?;
    let inserido = edital_repo::inserir(
        &conn,
        &NovoEdital {
            id: &id,
            nome: &nome,
            arquivo_original: &arquivo,
            sha256: &sha256,
            importado_em: agora,
        },
    );
    if let Err(e) = inserido {
        let _ = fs::remove_dir_all(paths.edital_dir(&id));
        return Err(e);
    }
    tracing::info!(edital = %id, arquivo = %arquivo, "PDF importado");
    Ok(ImportStart {
        edital: edital_repo::get(&conn, &id)?,
        duplicado: false,
    })
}

/// Recomeça a extração de um edital cuja importação ficou pela metade.
pub fn retomar(db: &Db, id: &str) -> AppResult<crate::domain::Edital> {
    let conn = db.conn()?;
    let e = edital_repo::get(&conn, id)?;
    if e.status != EditalStatus::Importando {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "edital já importado",
        ));
    }
    edital_repo::limpar_paginas(&conn, id)?;
    Ok(e)
}

/// Desfaz uma importação que o pdf.js não conseguiu abrir. Só vale para editais
/// ainda em `importando`, e só com os códigos de falha de abertura.
pub fn abortar(db: &Db, paths: &AppPaths, id: &str, code: ErrorCode) -> AppResult<()> {
    if !matches!(
        code,
        ErrorCode::ImportEncrypted | ErrorCode::ImportCorrupted | ErrorCode::ImportCancelado
    ) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            format!("código de abortar inválido: {code:?}"),
        ));
    }
    let conn = db.conn()?;
    let e = edital_repo::get(&conn, id)?;
    if e.status != EditalStatus::Importando {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "edital já importado",
        ));
    }
    edital_repo::remover(&conn, id)?;
    remover_dir(&paths.edital_dir(id));
    tracing::info!(edital = %id, ?code, "importação abortada");
    Ok(())
}

/// Apaga registros, a pasta do edital e os workspaces de jobs ligados a ele.
pub fn remover_edital(db: &Db, paths: &AppPaths, id: &str) -> AppResult<()> {
    let conn = db.conn()?;
    edital_repo::get(&conn, id)?;
    let jobs = job_repo::ids_do_edital(&conn, id)?;
    edital_repo::remover(&conn, id)?;
    drop(conn);
    remover_dir(&paths.edital_dir(id));
    for job in jobs {
        remover_dir(&paths.job_dir(&job));
    }
    tracing::info!(edital = %id, "edital removido");
    Ok(())
}

fn gravar_copia(paths: &AppPaths, id: &str, bytes: &[u8]) -> AppResult<()> {
    let dir = paths.edital_dir(id);
    fs::create_dir_all(&dir)?;
    let tmp = dir.join("original.pdf.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, paths.edital_pdf(id))?;
    Ok(())
}

fn remover_dir(dir: &Path) {
    if let Err(e) = fs::remove_dir_all(dir) {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(dir = %dir.display(), erro = %e, "não consegui apagar pasta");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ItemTexto, PaginaInput};

    const AGORA: &str = "2026-10-06T00:00:00Z";

    struct Ctx {
        _dir: tempfile::TempDir,
        db: Db,
        paths: AppPaths,
        src: std::path::PathBuf,
    }

    fn ctx() -> Ctx {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path().join("data")).unwrap();
        let src = dir.path().join("Edital TCE.pdf");
        fs::write(&src, b"%PDF-1.7\n...conteudo...").unwrap();
        Ctx {
            db: Db::open_in_memory().unwrap(),
            paths,
            src,
            _dir: dir,
        }
    }

    #[test]
    fn cabecalho() {
        assert!(tem_cabecalho_pdf(b"%PDF-1.4"));
        assert!(tem_cabecalho_pdf(b"\xEF\xBB\xBF  %PDF-1.4"));
        assert!(!tem_cabecalho_pdf(b"PK\x03\x04"));
        assert!(!tem_cabecalho_pdf(b""));
        let mut tarde = vec![b' '; 2000];
        tarde.extend_from_slice(b"%PDF-1.4");
        assert!(!tem_cabecalho_pdf(&tarde));
    }

    #[test]
    fn recusa_nao_pdf_sem_copiar() {
        let c = ctx();
        fs::write(&c.src, b"nao sou pdf").unwrap();
        let err = iniciar(&c.db, &c.paths, &c.src, AGORA).unwrap_err();
        assert_eq!(err.code, ErrorCode::ImportNotPdf);
        assert_eq!(fs::read_dir(c.paths.editais_dir()).unwrap().count(), 0);
    }

    #[test]
    fn copia_e_registra() {
        let c = ctx();
        let r = iniciar(&c.db, &c.paths, &c.src, AGORA).unwrap();
        assert!(!r.duplicado);
        assert_eq!(r.edital.nome, "Edital TCE");
        assert_eq!(r.edital.arquivo_original, "Edital TCE.pdf");
        assert_eq!(r.edital.status, EditalStatus::Importando);
        assert_eq!(
            fs::read(c.paths.edital_pdf(&r.edital.id)).unwrap(),
            fs::read(&c.src).unwrap()
        );
    }

    #[test]
    fn duplicata_devolve_existente() {
        let c = ctx();
        let r1 = iniciar(&c.db, &c.paths, &c.src, AGORA).unwrap();
        {
            let mut conn = c.db.conn().unwrap();
            let p = PaginaInput {
                pagina: 1,
                texto: "x".into(),
                itens: vec![ItemTexto { t: "x".into(), i: 0, b: None }],
                origem: Default::default(),
                confianca: None,
            };
            edital_repo::salvar_paginas(&mut conn, &r1.edital.id, &[p]).unwrap();
            edital_repo::finalizar_importacao(&conn, &r1.edital.id, 1).unwrap();
        }
        let r2 = iniciar(&c.db, &c.paths, &c.src, AGORA).unwrap();
        assert!(r2.duplicado);
        assert_eq!(r2.edital.id, r1.edital.id);
    }

    #[test]
    fn reimportar_incompleto_recomeca() {
        let c = ctx();
        let r1 = iniciar(&c.db, &c.paths, &c.src, AGORA).unwrap();
        let r2 = iniciar(&c.db, &c.paths, &c.src, AGORA).unwrap();
        assert!(!r2.duplicado);
        assert_eq!(r2.edital.id, r1.edital.id);
    }

    #[test]
    fn abortar_apaga_copia_e_registro() {
        let c = ctx();
        let r = iniciar(&c.db, &c.paths, &c.src, AGORA).unwrap();
        abortar(&c.db, &c.paths, &r.edital.id, ErrorCode::ImportCorrupted).unwrap();
        assert!(!c.paths.edital_dir(&r.edital.id).exists());
        let conn = c.db.conn().unwrap();
        assert!(edital_repo::find(&conn, &r.edital.id).unwrap().is_none());
    }

    #[test]
    fn abortar_recusa_codigo_qualquer() {
        let c = ctx();
        let r = iniciar(&c.db, &c.paths, &c.src, AGORA).unwrap();
        let err = abortar(&c.db, &c.paths, &r.edital.id, ErrorCode::Internal).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn remover_apaga_pasta_e_workspaces() {
        let c = ctx();
        let r = iniciar(&c.db, &c.paths, &c.src, AGORA).unwrap();
        let id = r.edital.id;
        c.db.conn()
            .unwrap()
            .execute(
                "INSERT INTO job (id, edital_id, tipo, status, criado_em) VALUES ('j1', ?1, 'x', 'concluido', ?2)",
                [&id, AGORA],
            )
            .unwrap();
        fs::create_dir_all(c.paths.job_dir("j1")).unwrap();
        remover_edital(&c.db, &c.paths, &id).unwrap();
        assert!(!c.paths.edital_dir(&id).exists());
        assert!(!c.paths.job_dir("j1").exists());
    }
}
