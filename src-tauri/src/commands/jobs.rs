use tauri::{AppHandle, State};

use crate::domain::{EstadoFila, Job, PerfilModelo, UsoClaude};
use crate::error::AppResult;
use crate::llm::jobs::NovoJob;
use crate::state::{AmbienteApp, AppState};
use crate::storage::{config_repo, job_repo};

const HISTORICO: u32 = 30;

#[tauri::command]
pub async fn jobs_listar(state: State<'_, AppState>) -> AppResult<Vec<Job>> {
    job_repo::recentes(&*state.db.conn()?, HISTORICO)
}

#[tauri::command]
pub async fn fila_estado(state: State<'_, AppState>) -> AppResult<EstadoFila> {
    Ok(state.fila.estado(&state.db))
}

#[tauri::command]
pub async fn job_cancelar(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.fila.cancelar(&AmbienteApp(app), &id)
}

/// Encerra a pausa por limite de uso antes do horário de renovação.
#[tauri::command]
pub async fn fila_retomar(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.fila.retomar_agora(&AmbienteApp(app));
    Ok(())
}

/// Último uso do plano visto (`None` antes do primeiro job).
#[tauri::command]
pub async fn claude_uso(state: State<'_, AppState>) -> AppResult<Option<UsoClaude>> {
    config_repo::uso_claude(&*state.db.conn()?)
}

/// Job mínimo que exercita o caminho real (runner, fila, consumo, limite).
#[tauri::command]
pub async fn claude_testar_execucao(
    app: AppHandle,
    state: State<'_, AppState>,
    confirmado: bool,
) -> AppResult<Job> {
    let novo = NovoJob {
        edital_id: None,
        tipo: "diagnostico".into(),
        system: "Você é o teste de diagnóstico do Edital Killer. Responda só no formato pedido.".into(),
        prompt: "Responda com ok = true.".into(),
        schema: serde_json::json!({
            "type": "object",
            "properties": {"ok": {"type": "boolean", "const": true}},
            "required": ["ok"],
            "additionalProperties": false
        }),
        perfil: PerfilModelo::Rapido,
    };
    state.fila.enfileirar(&AmbienteApp(app), novo, confirmado)
}

/// Resposta ao `fechar-pedido`: cancela a fila e o job em execução e sai.
#[tauri::command]
pub async fn app_sair(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.fila.cancelar_tudo(&AmbienteApp(app.clone())).await;
    tracing::info!("saindo com a fila cancelada");
    app.exit(0);
    Ok(())
}
