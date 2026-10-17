use std::sync::RwLock;

use tauri::{AppHandle, Emitter, Manager};

use crate::analise::Fila;
use crate::llm::backend::{BackendEstado, BackendStatus, LlmBackend};
use crate::llm::claude_cli::ClaudeCliBackend;
use crate::storage::{config_repo, AppPaths, Db};

/// Evento emitido sempre que o status do Claude CLI muda. Payload: `BackendStatus`.
pub const EVENTO_CLAUDE_STATUS: &str = "claude-status";

pub struct AppState {
    pub db: Db,
    pub paths: AppPaths,
    pub claude: RwLock<BackendStatus>,
    /// Serializa health checks concorrentes.
    pub health_lock: tokio::sync::Mutex<()>,
    pub fila: Fila,
}

impl AppState {
    pub fn new(db: Db, paths: AppPaths) -> Self {
        Self {
            db,
            paths,
            claude: RwLock::new(BackendStatus::verificando()),
            health_lock: tokio::sync::Mutex::new(()),
            fila: Fila::default(),
        }
    }

    pub fn claude_status(&self) -> BackendStatus {
        self.claude
            .read()
            .map(|s| s.clone())
            .unwrap_or_else(|_| BackendStatus::verificando())
    }

    pub fn set_claude_status(&self, app: &AppHandle, st: BackendStatus) {
        if let Ok(mut g) = self.claude.write() {
            *g = st.clone();
        }
        if let Err(e) = app.emit(EVENTO_CLAUDE_STATUS, st) {
            tracing::warn!(erro = %e, "falha ao emitir status do claude");
        }
    }
}

/// Roda o health check do Claude CLI, guarda e emite o resultado.
pub async fn rodar_health_check(app: &AppHandle) -> BackendStatus {
    let state = app.state::<AppState>();
    let _vez = state.health_lock.lock().await;

    let manual = state
        .db
        .conn()
        .and_then(|c| config_repo::settings(&c))
        .map(|s| s.claude_path)
        .unwrap_or_default();

    let mut anterior = state.claude_status();
    anterior.estado = BackendEstado::Verificando;
    state.set_claude_status(app, anterior);

    let st = ClaudeCliBackend {
        caminho_manual: manual,
    }
    .health_check()
    .await;

    tracing::info!(estado = ?st.estado, versao = ?st.versao, erro = ?st.erro, "health check do claude");
    let mut st = st;
    if st.estado == BackendEstado::Pronto {
        if let Err(e) = state.db.conn().and_then(|c| config_repo::marcar_health_passou(&c)) {
            tracing::warn!(erro = %e, "falha ao registrar health check aprovado");
        }
        // Um job pode ter começado durante a verificação.
        if state.fila.executando() {
            st.estado = BackendEstado::Ocupado;
        }
    }
    state.set_claude_status(app, st.clone());
    st
}
