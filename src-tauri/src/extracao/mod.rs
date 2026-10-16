//! Item 4: extração do edital por etapas (mapa, triagem, blocos). Ver a spec.

pub mod mapa;
pub mod triagem;

use crate::domain::{Bloco, EditalStatus, EstadoExtracao, Job, JobStatus, StatusExtracao};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::llm::jobs::{Ambiente, Fila, Notificacao, RESULTADO};
use crate::storage::{edital_repo, extracao_repo};

pub const PROMPT_SISTEMA: &str = include_str!("../../prompts/sistema.md");

fn agora() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Etapas 1 e 2: mapa local e triagem na fila. Refazer apaga a triagem anterior.
///
/// `confirmar_paginas`: o edital tem mais de 20% de páginas ruins e o usuário aceitou.
/// `confirmar_uso`: repassado à fila (aviso de uso alto).
pub fn processar<A: Ambiente>(
    amb: &A,
    fila: &Fila,
    edital_id: &str,
    confirmar_paginas: bool,
    confirmar_uso: bool,
) -> AppResult<EstadoExtracao> {
    let (edital, textos) = {
        let conn = amb.db().conn()?;
        let edital = edital_repo::get(&conn, edital_id)?;
        if extracao_repo::pendente(&conn, edital_id, Bloco::Triagem)? {
            return Err(AppError::new(ErrorCode::InvalidInput, "a triagem deste edital já está na fila"));
        }
        (edital, extracao_repo::paginas(&conn, edital_id)?)
    };
    match edital.status {
        EditalStatus::SemTexto => return Err(AppError::new(ErrorCode::EditalSemTexto, "")),
        EditalStatus::Importando => {
            return Err(AppError::new(ErrorCode::InvalidInput, "a importação deste edital não terminou"))
        }
        EditalStatus::Importado => {}
    }
    if edital.exige_confirmacao && !confirmar_paginas {
        let ruins = edital.paginas_problema.len();
        return Err(AppError::new(
            ErrorCode::EditalConfirmarPaginas,
            format!("{ruins} de {} páginas com problema", edital.total_paginas.unwrap_or(0)),
        ));
    }

    let paginas: Vec<mapa::Pagina> = textos
        .iter()
        .map(|(n, t)| mapa::Pagina { numero: *n, texto: t })
        .collect();
    let repetidas = mapa::linhas_repetidas(&paginas);
    let secoes = mapa::mapear(&paginas, &repetidas);
    let sumario = triagem::sumario(&paginas, &secoes, &repetidas);
    let novo = triagem::novo_job(edital_id, &edital.arquivo_original, &sumario)?;

    // Enfileira antes de gravar o mapa: se o usuário desistir no aviso de uso alto,
    // nada muda no edital.
    let job = fila.enfileirar(amb, novo, confirmar_uso)?;
    let mut conn = amb.db().conn()?;
    extracao_repo::salvar_mapa(&mut conn, edital_id, &secoes)?;
    extracao_repo::inserir(&conn, edital_id, Bloco::Triagem, None, &job.id, triagem::VERSAO, &agora())?;
    tracing::info!(edital = %edital_id, secoes = secoes.len(), sumario_chars = sumario.chars().count(), "mapa feito; triagem na fila");
    let estado = extracao_repo::estado(&conn, edital_id)?;
    drop(conn);
    amb.notificar(Notificacao::Extracao(edital_id.to_owned()));
    Ok(estado)
}

/// Chamado quando um job termina: se for de extração, grava o resultado.
pub fn ao_terminar<A: Ambiente>(amb: &A, _fila: &Fila, job: &Job) -> AppResult<()> {
    let conn = amb.db().conn()?;
    let Some((extracao, edital_id)) = extracao_repo::por_job(&conn, &job.id)? else {
        return Ok(());
    };
    let desfecho = if job.status == JobStatus::Concluido {
        let resultado = std::fs::read_to_string(amb.paths().job_dir(&job.id).join(RESULTADO))?;
        match extracao.bloco {
            Bloco::Triagem => triagem::aplicar(&conn, &edital_id, &resultado),
            outro => Err(AppError::new(ErrorCode::Internal, format!("bloco {outro:?} ainda não implementado"))),
        }
    } else {
        let code = match job.status {
            JobStatus::Cancelado => ErrorCode::JobCancelado,
            JobStatus::Interrompido => ErrorCode::JobInterrompido,
            _ => job.erro_code.unwrap_or(ErrorCode::Internal),
        };
        Err(AppError::new(code, job.erro_detail.clone().unwrap_or_default()))
    };
    match &desfecho {
        Ok(()) => extracao_repo::finalizar(&conn, &extracao.id, StatusExtracao::Concluido, None, &agora())?,
        Err(e) => {
            tracing::warn!(edital = %edital_id, bloco = ?extracao.bloco, code = ?e.code, detail = %e.detail, "extração falhou");
            extracao_repo::finalizar(&conn, &extracao.id, StatusExtracao::Falhou, Some(e), &agora())?
        }
    }
    drop(conn);
    amb.notificar(Notificacao::Extracao(edital_id));
    Ok(())
}
