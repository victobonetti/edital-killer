use std::sync::{Arc, RwLock};

use tauri::{AppHandle, Emitter, Manager};

use crate::domain::EstadoFila;
use crate::llm::backend::{BackendEstado, BackendStatus, LlmBackend};
use crate::llm::claude_cli::ClaudeCliBackend;
use crate::llm::jobs::{Ambiente, Fila, Notificacao};
use crate::storage::{config_repo, AppPaths, Db};

/// Evento emitido sempre que o status do Claude CLI muda. Payload: `BackendStatus`.
pub const EVENTO_CLAUDE_STATUS: &str = "claude-status";
/// Payload: `EstadoFila`.
pub const EVENTO_JOBS: &str = "jobs-atualizados";
/// Payload: `UsoClaude`.
pub const EVENTO_USO: &str = "claude-uso";
/// Payload: `Job` em estado final.
pub const EVENTO_JOB_TERMINOU: &str = "job-terminou";
/// Janela pediu para fechar com job em execução. Payload: `EstadoFila`.
pub const EVENTO_FECHAR_PEDIDO: &str = "fechar-pedido";

pub struct AppState {
    pub db: Db,
    pub paths: AppPaths,
    pub fila: Arc<Fila>,
    /// Resultado do último health check.
    claude: RwLock<BackendStatus>,
    fila_estado: RwLock<EstadoFila>,
    /// Serializa health checks concorrentes.
    pub health_lock: tokio::sync::Mutex<()>,
}

impl AppState {
    pub fn new(db: Db, paths: AppPaths, fila: Arc<Fila>) -> Self {
        let fila_estado = fila.estado(&db);
        Self {
            db,
            paths,
            fila,
            claude: RwLock::new(BackendStatus::verificando()),
            fila_estado: RwLock::new(fila_estado),
            health_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Status do indicador: o health check, ajustado pela fila (ocupado, em limite).
    pub fn claude_status(&self) -> BackendStatus {
        let mut st = self
            .claude
            .read()
            .map(|s| s.clone())
            .unwrap_or_else(|_| BackendStatus::verificando());
        if st.estado == BackendEstado::Pronto {
            let fila = self.fila_estado.read().map(|f| f.clone()).unwrap_or_default();
            if let Some(ate) = fila.pausada_ate {
                st.estado = BackendEstado::LimiteUso;
                st.retorno_em = Some(ate);
            } else if fila.executando.is_some() {
                st.estado = BackendEstado::Ocupado;
            }
        }
        st
    }

    fn emitir_status(&self, app: &AppHandle) {
        if let Err(e) = app.emit(EVENTO_CLAUDE_STATUS, self.claude_status()) {
            tracing::warn!(erro = %e, "falha ao emitir status do claude");
        }
    }

    fn set_claude_status(&self, app: &AppHandle, st: BackendStatus) {
        if let Ok(mut g) = self.claude.write() {
            *g = st;
        }
        self.emitir_status(app);
    }
}

/// A fila conversa com o app por aqui.
pub struct AmbienteApp(pub AppHandle);

impl Ambiente for AmbienteApp {
    fn db(&self) -> &Db {
        &self.0.state::<AppState>().inner().db
    }

    fn paths(&self) -> &AppPaths {
        &self.0.state::<AppState>().inner().paths
    }

    fn notificar(&self, n: Notificacao) {
        let state = self.0.state::<AppState>();
        let emitido = match n {
            Notificacao::Fila(estado) => {
                if let Ok(mut g) = state.fila_estado.write() {
                    *g = estado.clone();
                }
                state.emitir_status(&self.0);
                self.0.emit(EVENTO_JOBS, estado)
            }
            Notificacao::Uso(uso) => self.0.emit(EVENTO_USO, uso),
            Notificacao::JobTerminou(job) => self.0.emit(EVENTO_JOB_TERMINOU, job),
            Notificacao::LoginPerdido => {
                let app = self.0.clone();
                tauri::async_runtime::spawn(async move {
                    rodar_health_check(&app).await;
                });
                Ok(())
            }
        };
        if let Err(e) = emitido {
            tracing::warn!(erro = %e, "falha ao emitir evento da fila");
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

    let mut anterior = state.claude.read().map(|s| s.clone()).unwrap_or_else(|_| BackendStatus::verificando());
    anterior.estado = BackendEstado::Verificando;
    state.set_claude_status(app, anterior);

    let st = ClaudeCliBackend {
        caminho_manual: manual,
    }
    .health_check()
    .await;

    tracing::info!(estado = ?st.estado, versao = ?st.versao, erro = ?st.erro, "health check do claude");
    if st.estado == BackendEstado::Pronto {
        if let Err(e) = state.db.conn().and_then(|c| config_repo::marcar_health_passou(&c)) {
            tracing::warn!(erro = %e, "falha ao registrar health check aprovado");
        }
    }
    state.set_claude_status(app, st);
    state.claude_status()
}
