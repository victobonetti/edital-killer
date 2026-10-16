use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use ts_rs::TS;

use crate::domain::{JobStatus, Settings};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::{rodar_health_check, AppState};
use crate::storage::config_repo;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Bootstrap {
    pub versao_app: String,
    pub pasta_dados: String,
    /// Se o health check do Claude já passou alguma vez nesta máquina.
    pub health_ja_passou: bool,
}

#[tauri::command]
pub async fn app_bootstrap(app: AppHandle, state: State<'_, AppState>) -> AppResult<Bootstrap> {
    let conn = state.db.conn()?;
    Ok(Bootstrap {
        versao_app: app.package_info().version.to_string(),
        pasta_dados: state.paths.root().display().to_string(),
        health_ja_passou: config_repo::health_ja_passou(&conn)?,
    })
}

#[tauri::command]
pub async fn settings_obter(state: State<'_, AppState>) -> AppResult<Settings> {
    config_repo::settings(&*state.db.conn()?)
}

#[tauri::command]
pub async fn settings_salvar(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<Settings> {
    if !(30..=7200).contains(&settings.timeout_secs) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "timeout deve ficar entre 30 e 7200 segundos",
        ));
    }
    let limpa = |s: Option<String>| s.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    let settings = Settings {
        claude_path: limpa(settings.claude_path),
        model: limpa(settings.model),
        timeout_secs: settings.timeout_secs,
    };
    let caminho_mudou = {
        let conn = state.db.conn()?;
        let antes = config_repo::settings(&conn)?;
        config_repo::save_settings(&conn, &settings)?;
        antes.claude_path != settings.claude_path
    };
    if caminho_mudou {
        tauri::async_runtime::spawn(async move {
            rodar_health_check(&app).await;
        });
    }
    Ok(settings)
}

#[tauri::command]
pub async fn abrir_pasta_dados(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    abrir(&app, state.paths.root())
}

#[tauri::command]
pub async fn abrir_pasta_logs(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    abrir(&app, &state.paths.logs_dir())
}

/// Apaga os workspaces de jobs que não estão em execução. Devolve quantos apagou.
#[tauri::command]
pub async fn limpar_workspaces_jobs(state: State<'_, AppState>) -> AppResult<u32> {
    let executando: Vec<String> = {
        let conn = state.db.conn()?;
        let mut stmt = conn.prepare("SELECT id FROM job WHERE status = ?1")?;
        let ids = stmt
            .query_map([JobStatus::Executando.as_str()], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids
    };
    let mut n = 0;
    for entry in std::fs::read_dir(state.paths.jobs_dir())? {
        let entry = entry?;
        let nome = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_dir() && !executando.contains(&nome) {
            std::fs::remove_dir_all(entry.path())?;
            n += 1;
        }
    }
    tracing::info!(n, "workspaces de jobs apagados");
    Ok(n)
}

fn abrir(app: &AppHandle, path: &std::path::Path) -> AppResult<()> {
    app.opener()
        .open_path(path.display().to_string(), None::<&str>)
        .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))
}
