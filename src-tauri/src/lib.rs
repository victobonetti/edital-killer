mod analise;
mod commands;
mod domain;
mod error;
mod extraction;
mod importacao;
mod llm;
mod state;
mod storage;
#[cfg(test)]
mod test_support;

use tauri::{Manager, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tracing_subscriber::EnvFilter;

use crate::state::{rodar_health_check, AppState};
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
            app.manage(AppState::new(db, paths));
            analise::iniciar_worker(app.handle());

            // 3. Health check em segundo plano; a UI abre com o indicador em "verificando".
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                rodar_health_check(&handle).await;
            });
            Ok(())
        })
        .on_window_event(confirmar_fechamento)
        .invoke_handler(tauri::generate_handler![
            commands::analise::analise_iniciar,
            commands::analise::cargo_escolher,
            commands::analise::analise_cancelar,
            commands::analise::edital_analise,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Fechar com análise em andamento pergunta antes; confirmando, o `claude` é morto
/// (cancelamento + `kill_on_drop`) e o app sai. Nenhum processo filho sobrevive.
fn confirmar_fechamento(window: &tauri::Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    let app = window.app_handle().clone();
    if !app.state::<AppState>().fila.executando() {
        return;
    }
    api.prevent_close();
    app.dialog()
        .message("Uma análise de edital está em andamento. Se fechar agora, ela será cancelada e você poderá começar de novo depois.")
        .title("Fechar o Edital Killer?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Cancelar análise e fechar".into(),
            "Continuar".into(),
        ))
        .show(move |fechar| {
            if !fechar {
                return;
            }
            app.state::<AppState>().fila.cancelar_atual(None);
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                // Dá tempo de o worker matar o processo e registrar o cancelamento.
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                app.exit(0);
            });
        });
}
