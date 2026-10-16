mod commands;
// Públicos para os testes de integração (tests/), que usam o fake-claude.
pub mod domain;
pub mod error;
pub mod extracao;
mod importacao;
pub mod llm;
mod state;
pub mod storage;

use std::sync::Arc;

use tauri::{Emitter, Manager};
use tracing_subscriber::EnvFilter;

use crate::llm::jobs::{Esperas, Fila};
use crate::state::{rodar_health_check, AmbienteApp, AppState, EVENTO_FECHAR_PEDIDO};
use crate::storage::{job_repo, AppPaths, Db};

/// Mantém o writer de log vivo enquanto o app roda.
struct LogGuard(#[allow(dead_code)] tracing_appender::non_blocking::WorkerGuard);

fn iniciar_log(paths: &AppPaths) -> Result<LogGuard, Box<dyn std::error::Error>> {
    let arquivo = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("core")
        .filename_suffix("log")
        .max_log_files(14)
        .build(paths.logs_dir())?;
    let (writer, guard) = tracing_appender::non_blocking(arquivo);
    tracing_subscriber::fmt()
        .json()
        .with_writer(writer)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    Ok(LogGuard(guard))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let paths = AppPaths::new(app.path().app_data_dir()?)?;
            app.manage(iniciar_log(&paths)?);
            tracing::info!(versao = %app.package_info().version, dados = %paths.root().display(), "iniciando");

            // 1. Migrações. 2. Jobs órfãos viram `interrompido`.
            let db = Db::open(&paths.db_file())?;
            let orfaos =
                job_repo::interromper_orfaos(&*db.conn()?, &chrono::Utc::now().to_rfc3339())?;
            if orfaos > 0 {
                tracing::warn!(orfaos, "jobs interrompidos pelo fechamento anterior");
            }
            let soltas = storage::extracao_repo::reconciliar(&*db.conn()?, &chrono::Utc::now().to_rfc3339())?;
            if soltas > 0 {
                tracing::warn!(soltas, "extrações marcadas como interrompidas");
            }
            let fila = Fila::nova(&db, Esperas::default());
            app.manage(AppState::new(db, paths, Arc::clone(&fila)));
            tauri::async_runtime::spawn(fila.rodar(Arc::new(AmbienteApp(app.handle().clone()))));

            // 3. Health check em segundo plano; a UI abre com o indicador em "verificando".
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                rodar_health_check(&handle).await;
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Com job em execução, a UI pergunta antes; `app_sair` cancela tudo e sai.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                let estado = state.fila.estado(&state.db);
                if estado.executando.is_some() {
                    api.prevent_close();
                    if let Err(e) = window.emit(EVENTO_FECHAR_PEDIDO, estado) {
                        tracing::warn!(erro = %e, "falha ao pedir confirmação de fechamento");
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_bootstrap,
            commands::app::settings_obter,
            commands::app::settings_salvar,
            commands::app::abrir_pasta_dados,
            commands::app::abrir_pasta_logs,
            commands::app::limpar_workspaces_jobs,
            commands::claude::claude_status,
            commands::claude::claude_verificar,
            commands::editais::escolher_pdfs,
            commands::editais::editais_listar,
            commands::editais::edital_obter,
            commands::editais::edital_atualizar,
            commands::editais::edital_remover,
            commands::editais::edital_pdf,
            commands::editais::importar_iniciar,
            commands::editais::importar_retomar,
            commands::editais::importar_paginas,
            commands::editais::importar_finalizar,
            commands::editais::importar_abortar,
            commands::jobs::jobs_listar,
            commands::jobs::fila_estado,
            commands::jobs::job_cancelar,
            commands::jobs::fila_retomar,
            commands::jobs::claude_uso,
            commands::jobs::claude_testar_execucao,
            commands::jobs::app_sair,
            commands::extracao::edital_processar,
            commands::extracao::extracao_estado,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
