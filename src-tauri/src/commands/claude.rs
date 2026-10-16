use tauri::{AppHandle, State};

use crate::llm::backend::BackendStatus;
use crate::state::{rodar_health_check, AppState};

#[tauri::command]
pub async fn claude_status(state: State<'_, AppState>) -> Result<BackendStatus, ()> {
    Ok(state.claude_status())
}

#[tauri::command]
pub async fn claude_verificar(app: AppHandle) -> Result<BackendStatus, ()> {
    Ok(rodar_health_check(&app).await)
}
