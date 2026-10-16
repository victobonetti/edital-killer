use tauri::State;

use crate::domain::Job;
use crate::error::AppResult;
use crate::state::AppState;
use crate::storage::job_repo;

const HISTORICO: u32 = 30;

#[tauri::command]
pub async fn jobs_listar(state: State<'_, AppState>) -> AppResult<Vec<Job>> {
    job_repo::recentes(&*state.db.conn()?, HISTORICO)
}
