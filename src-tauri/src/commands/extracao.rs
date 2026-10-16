use tauri::{AppHandle, State};

use crate::domain::EstadoExtracao;
use crate::error::AppResult;
use crate::extracao;
use crate::state::{AmbienteApp, AppState};
use crate::storage::extracao_repo;

/// Etapas 1 e 2: mapa e triagem. Ver `extracao::processar` para as confirmações.
#[tauri::command]
pub async fn edital_processar(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    confirmar_paginas: bool,
    confirmar_uso: bool,
) -> AppResult<EstadoExtracao> {
    extracao::processar(&AmbienteApp(app), &state.fila, &id, confirmar_paginas, confirmar_uso)
}

#[tauri::command]
pub async fn extracao_estado(state: State<'_, AppState>, id: String) -> AppResult<EstadoExtracao> {
    extracao_repo::estado(&*state.db.conn()?, &id)
}
