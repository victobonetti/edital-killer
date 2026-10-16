use std::path::PathBuf;

use tauri::ipc::Response;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::domain::{Edital, EditalStatus, EditalUpdate, ImportStart, PaginaInput};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::importacao;
use crate::state::{AmbienteApp, AppState};
use crate::storage::edital_repo;

/// Abre o dialog nativo (filtro `.pdf`, seleção múltipla). Lista vazia = cancelado.
#[tauri::command]
pub async fn escolher_pdfs(app: AppHandle) -> AppResult<Vec<String>> {
    let escolhidos = app
        .dialog()
        .file()
        .set_title("Importar edital")
        .add_filter("PDF", &["pdf"])
        .blocking_pick_files()
        .unwrap_or_default();
    escolhidos
        .into_iter()
        .map(|f| {
            f.into_path()
                .map(|p| p.display().to_string())
                .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))
        })
        .collect()
}

#[tauri::command]
pub async fn editais_listar(state: State<'_, AppState>) -> AppResult<Vec<Edital>> {
    edital_repo::listar(&*state.db.conn()?)
}

#[tauri::command]
pub async fn edital_obter(state: State<'_, AppState>, id: String) -> AppResult<Edital> {
    edital_repo::get(&*state.db.conn()?, &id)
}

#[tauri::command]
pub async fn edital_atualizar(
    state: State<'_, AppState>,
    id: String,
    dados: EditalUpdate,
) -> AppResult<Edital> {
    edital_repo::atualizar(&*state.db.conn()?, &id, &dados)
}

#[tauri::command]
pub async fn edital_remover(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    // Nenhum claude pode continuar trabalhando num edital que vai sumir.
    state.fila.cancelar_do_edital(&AmbienteApp(app), &id).await?;
    importacao::remover_edital(&state.db, &state.paths, &id)
}

/// Bytes do PDF copiado, para o pdf.js renderizar. A UI nunca lê o workspace direto.
#[tauri::command]
pub async fn edital_pdf(state: State<'_, AppState>, id: String) -> AppResult<Response> {
    edital_repo::get(&*state.db.conn()?, &id)?;
    let bytes = std::fs::read(state.paths.edital_pdf(&id))?;
    Ok(Response::new(bytes))
}

#[tauri::command]
pub async fn importar_iniciar(state: State<'_, AppState>, caminho: String) -> AppResult<ImportStart> {
    let agora = chrono::Utc::now().to_rfc3339();
    importacao::iniciar(&state.db, &state.paths, &PathBuf::from(caminho), &agora)
}

#[tauri::command]
pub async fn importar_retomar(state: State<'_, AppState>, id: String) -> AppResult<Edital> {
    importacao::retomar(&state.db, &id)
}

#[tauri::command]
pub async fn importar_paginas(
    state: State<'_, AppState>,
    id: String,
    paginas: Vec<PaginaInput>,
) -> AppResult<Vec<u32>> {
    // Devolve as páginas do lote que precisam de OCR.
    let mut conn = state.db.conn()?;
    exigir_importando(&edital_repo::get(&conn, &id)?)?;
    edital_repo::salvar_paginas(&mut conn, &id, &paginas)
}

#[tauri::command]
pub async fn importar_finalizar(
    state: State<'_, AppState>,
    id: String,
    total_paginas: u32,
) -> AppResult<Edital> {
    let conn = state.db.conn()?;
    exigir_importando(&edital_repo::get(&conn, &id)?)?;
    let e = edital_repo::finalizar_importacao(&conn, &id, total_paginas)?;
    tracing::info!(edital = %id, total_paginas, problemas = e.paginas_problema.len(), ocr = e.paginas_ocr.len(), status = ?e.status, "importação concluída");
    Ok(e)
}

/// A UI chama quando o pdf.js não abre o PDF (`IMPORT_ENCRYPTED` ou `IMPORT_CORRUPTED`).
#[tauri::command]
pub async fn importar_abortar(
    state: State<'_, AppState>,
    id: String,
    code: ErrorCode,
) -> AppResult<()> {
    importacao::abortar(&state.db, &state.paths, &id, code)
}

fn exigir_importando(e: &Edital) -> AppResult<()> {
    if e.status == EditalStatus::Importando {
        Ok(())
    } else {
        Err(AppError::new(ErrorCode::InvalidInput, "edital já importado"))
    }
}
