use tauri::{AppHandle, Emitter, State};

use crate::analise::{self, EVENTO_ANALISE};
use crate::domain::{Analise, EtapaAnalise};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::storage::{analise_repo, job_repo};

/// Começa a análise: metadados e cargos do edital.
#[tauri::command]
pub async fn analise_iniciar(app: AppHandle, state: State<'_, AppState>, edital_id: String) -> AppResult<()> {
    analise::enfileirar(&state, &edital_id, EtapaAnalise::Localizar)?;
    let _ = app.emit(EVENTO_ANALISE, &edital_id);
    Ok(())
}

/// Grava o cargo do usuário e já extrai o conteúdo programático dele.
#[tauri::command]
pub async fn cargo_escolher(
    app: AppHandle,
    state: State<'_, AppState>,
    edital_id: String,
    cargo_id: String,
) -> AppResult<()> {
    {
        let conn = state.db.conn()?;
        if job_repo::ativo_do_edital(&conn, &edital_id)?.is_some() {
            return Err(AppError::new(ErrorCode::AnaliseEmAndamento, edital_id));
        }
        analise_repo::escolher_cargo(&conn, &edital_id, &cargo_id)?;
    }
    analise::enfileirar(&state, &edital_id, EtapaAnalise::Extrair)?;
    let _ = app.emit(EVENTO_ANALISE, &edital_id);
    Ok(())
}

#[tauri::command]
pub async fn analise_cancelar(app: AppHandle, state: State<'_, AppState>, edital_id: String) -> AppResult<()> {
    let na_fila = job_repo::cancelar_na_fila(&*state.db.conn()?, &edital_id, &chrono::Utc::now().to_rfc3339())?;
    let executando = state.fila.cancelar_atual(Some(&edital_id));
    if na_fila > 0 && !executando {
        let _ = app.emit(EVENTO_ANALISE, &edital_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn edital_analise(state: State<'_, AppState>, edital_id: String) -> AppResult<Analise> {
    analise::carregar(&state.db, &edital_id)
}
