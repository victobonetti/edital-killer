//! Fila serial de análises. Um job por vez: o `claude` da máquina é um recurso
//! único e caro. A UI só vê o andamento por edital (evento `analise-atualizada`);
//! jobs, workspaces e logs ficam para quem abrir Ajustes › Avançado.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

use crate::domain::{Andamento, Analise, EditalStatus, EtapaAnalise, JobStatus};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::extraction;
use crate::llm::backend::{BackendEstado, FalhaJob, LlmBackend};
use crate::llm::cancel::CancelToken;
use crate::llm::claude_cli::ClaudeCliBackend;
use crate::state::{rodar_health_check, AppState};
use crate::storage::{analise_repo, config_repo, edital_repo, job_repo, AppPaths, Db};

/// Evento emitido quando a análise de um edital muda. Payload: id do edital.
pub const EVENTO_ANALISE: &str = "analise-atualizada";

pub struct Pedido {
    pub job_id: String,
    pub edital_id: String,
    pub etapa: EtapaAnalise,
}

struct Atual {
    job_id: String,
    edital_id: String,
    cancel: CancelToken,
}

pub struct Fila {
    tx: mpsc::UnboundedSender<Pedido>,
    rx: Mutex<Option<mpsc::UnboundedReceiver<Pedido>>>,
    atual: Mutex<Option<Atual>>,
}

impl Default for Fila {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: Mutex::new(Some(rx)),
            atual: Mutex::new(None),
        }
    }
}

impl Fila {
    pub fn executando(&self) -> bool {
        self.atual.lock().map(|a| a.is_some()).unwrap_or(false)
    }

    /// Cancela o job em execução (de um edital, ou qualquer um). Devolve se havia algum.
    pub fn cancelar_atual(&self, edital_id: Option<&str>) -> bool {
        let Ok(a) = self.atual.lock() else { return false };
        match a.as_ref() {
            Some(a) if edital_id.is_none_or(|e| e == a.edital_id) => {
                a.cancel.cancelar();
                tracing::info!(job = %a.job_id, "cancelamento pedido");
                true
            }
            _ => false,
        }
    }
}

/// Cria o job e põe na fila. Recusa se o edital não pode ser analisado ou já está em análise.
pub fn enfileirar(state: &AppState, edital_id: &str, etapa: EtapaAnalise) -> AppResult<()> {
    let conn = state.db.conn()?;
    let edital = edital_repo::get(&conn, edital_id)?;
    match edital.status {
        EditalStatus::Importado => {}
        EditalStatus::SemTexto => return Err(AppError::new(ErrorCode::EditalSemTexto, edital_id)),
        EditalStatus::Importando => {
            return Err(AppError::new(ErrorCode::InvalidInput, "importação incompleta"))
        }
    }
    if job_repo::ativo_do_edital(&conn, edital_id)?.is_some() {
        return Err(AppError::new(ErrorCode::AnaliseEmAndamento, edital_id));
    }
    let job_id = uuid::Uuid::new_v4().to_string();
    job_repo::criar(&conn, &job_id, edital_id, etapa.tipo_job(), &agora())?;
    tracing::info!(job = %job_id, edital = edital_id, tipo = etapa.tipo_job(), "job na fila");
    state
        .fila
        .tx
        .send(Pedido {
            job_id,
            edital_id: edital_id.to_owned(),
            etapa,
        })
        .map_err(|_| AppError::new(ErrorCode::Internal, "fila de análises parada"))
}

/// Cargos, conteúdo e andamento do edital, como a tela mostra.
pub fn carregar(db: &Db, edital_id: &str) -> AppResult<Analise> {
    let conn = db.conn()?;
    edital_repo::get(&conn, edital_id)?;
    let mut a = analise_repo::carregar(&conn, edital_id)?;
    a.andamento = job_repo::ultimo_do_edital(&conn, edital_id)?.and_then(|j| {
        Some(Andamento {
            etapa: EtapaAnalise::do_tipo_job(&j.tipo)?,
            status: j.status,
            desde: j.iniciado_em.unwrap_or(j.criado_em),
            erro: j.erro_code.map(|code| AppError::new(code, j.erro_detail.unwrap_or_default())),
        })
    });
    Ok(a)
}

/// Inicia o worker da fila. Chamado uma vez, no setup do app.
pub fn iniciar_worker(app: &AppHandle) {
    let Some(mut rx) = app.state::<AppState>().fila.rx.lock().ok().and_then(|mut r| r.take()) else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(p) = rx.recv().await {
            processar(&app, p).await;
        }
    });
}

async fn processar(app: &AppHandle, p: Pedido) {
    let state = app.state::<AppState>();
    let cancel = CancelToken::default();
    match state.db.conn().and_then(|c| job_repo::iniciar(&c, &p.job_id, &agora())) {
        Ok(true) => {}
        Ok(false) => return, // cancelado enquanto esperava
        Err(e) => {
            tracing::error!(erro = %e, "falha ao iniciar job");
            return;
        }
    }
    if let Ok(mut a) = state.fila.atual.lock() {
        *a = Some(Atual {
            job_id: p.job_id.clone(),
            edital_id: p.edital_id.clone(),
            cancel: cancel.clone(),
        });
    }
    let mut st = state.claude_status();
    st.estado = BackendEstado::Ocupado;
    state.set_claude_status(app, st);
    emitir(app, &p.edital_id);

    let settings = state
        .db
        .conn()
        .and_then(|c| config_repo::settings(&c))
        .unwrap_or_default();
    let backend = ClaudeCliBackend {
        caminho_manual: settings.claude_path.clone(),
    };
    let r = executar(&state.db, &state.paths, &backend, &p, &settings, cancel).await;

    if let Ok(mut a) = state.fila.atual.lock() {
        *a = None;
    }
    let mut st = state.claude_status();
    st.estado = BackendEstado::Pronto;
    st.retorno_em = None;
    match &r {
        Err(f) if f.erro.code == ErrorCode::ClaudeUsageLimit => {
            st.estado = BackendEstado::LimiteUso;
            st.retorno_em = f.retorno_em.clone();
        }
        Err(f) if matches!(f.erro.code, ErrorCode::ClaudeNotFound | ErrorCode::ClaudeNotLoggedIn) => {
            // O ambiente mudou desde o último health check: refaz.
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                rodar_health_check(&app).await;
            });
        }
        _ => {}
    }
    state.set_claude_status(app, st);
    emitir(app, &p.edital_id);
}

/// Executa um job do começo ao fim e registra o resultado. Sem Tauri, para testar.
pub async fn executar(
    db: &Db,
    paths: &AppPaths,
    backend: &dyn LlmBackend,
    p: &Pedido,
    settings: &crate::domain::Settings,
    cancel: CancelToken,
) -> Result<(), FalhaJob> {
    let sem_retorno = |erro: AppError| FalhaJob {
        erro,
        retorno_em: None,
    };
    let r = async {
        let ex = extraction::preparar(db, &paths.job_dir(&p.job_id), &p.edital_id, p.etapa, settings)
            .map_err(sem_retorno)?;
        let saida = backend.executar(ex, cancel).await?;
        extraction::aplicar(db, &p.edital_id, p.etapa, saida).map_err(sem_retorno)
    }
    .await;

    let (status, erro) = match &r {
        Ok(()) => (JobStatus::Concluido, None),
        Err(f) if f.erro.code == ErrorCode::JobCancelled => (JobStatus::Cancelado, None),
        Err(f) => (JobStatus::Falhou, Some(&f.erro)),
    };
    match &r {
        Ok(()) => tracing::info!(job = %p.job_id, "job concluído"),
        Err(f) => tracing::warn!(job = %p.job_id, code = ?f.erro.code, detail = %f.erro.detail, "job não concluído"),
    }
    if let Err(e) = db
        .conn()
        .and_then(|c| job_repo::finalizar(&c, &p.job_id, status, &agora(), erro))
    {
        tracing::error!(erro = %e, "falha ao registrar fim do job");
    }
    r
}

fn emitir(app: &AppHandle, edital_id: &str) {
    if let Err(e) = app.emit(EVENTO_ANALISE, edital_id) {
        tracing::warn!(erro = %e, "falha ao emitir andamento da análise");
    }
}

fn agora() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Settings;
    use crate::llm::backend::{BackendStatus, Execucao};
    use async_trait::async_trait;
    use serde_json::{json, Value};

    struct Fake(Result<Value, ErrorCode>);

    #[async_trait]
    impl LlmBackend for Fake {
        async fn health_check(&self) -> BackendStatus {
            BackendStatus::verificando()
        }
        async fn executar(&self, ex: Execucao, _c: CancelToken) -> Result<Value, FalhaJob> {
            assert!(ex.workspace.join("edital.txt").is_file());
            self.0.clone().map_err(|code| FalhaJob {
                erro: AppError::new(code, "fake"),
                retorno_em: None,
            })
        }
    }

    fn ambiente() -> (Db, AppPaths, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path()).unwrap();
        let db = Db::open_in_memory().unwrap();
        crate::extraction::tests_apoio::edital_de_teste(&db);
        (db, paths, dir)
    }

    fn pedido(db: &Db, etapa: EtapaAnalise) -> Pedido {
        let id = uuid::Uuid::new_v4().to_string();
        job_repo::criar(&db.conn().unwrap(), &id, "e1", etapa.tipo_job(), &agora()).unwrap();
        job_repo::iniciar(&db.conn().unwrap(), &id, &agora()).unwrap();
        Pedido {
            job_id: id,
            edital_id: "e1".into(),
            etapa,
        }
    }

    fn cargos() -> Value {
        json!({
            "metadados": {"titulo": null, "orgao": null, "banca": null, "numero_edital": null,
                          "data_prova": null, "inscricoes_inicio": null, "inscricoes_fim": null},
            "cargos": [{"nome": "Auditor de Controle Externo – TI", "vagas": 12, "escolaridade": null, "pagina": 2}]
        })
    }

    #[tokio::test]
    async fn job_concluido_grava_e_aparece_no_andamento() {
        let (db, paths, _d) = ambiente();
        let p = pedido(&db, EtapaAnalise::Localizar);
        executar(&db, &paths, &Fake(Ok(cargos())), &p, &Settings::default(), CancelToken::default())
            .await
            .unwrap();
        let a = carregar(&db, "e1").unwrap();
        assert_eq!(a.cargos.len(), 1);
        let and = a.andamento.unwrap();
        assert_eq!((and.etapa, and.status), (EtapaAnalise::Localizar, JobStatus::Concluido));
        assert!(paths.job_dir(&p.job_id).join("prompt.md").is_file());
    }

    #[tokio::test]
    async fn falha_e_cancelamento_ficam_registrados() {
        let (db, paths, _d) = ambiente();
        let p = pedido(&db, EtapaAnalise::Localizar);
        let f = executar(&db, &paths, &Fake(Err(ErrorCode::ClaudeTimeout)), &p, &Settings::default(), CancelToken::default())
            .await
            .unwrap_err();
        assert_eq!(f.erro.code, ErrorCode::ClaudeTimeout);
        let and = carregar(&db, "e1").unwrap().andamento.unwrap();
        assert_eq!(and.status, JobStatus::Falhou);
        assert_eq!(and.erro.unwrap().code, ErrorCode::ClaudeTimeout);

        let p = pedido(&db, EtapaAnalise::Localizar);
        executar(&db, &paths, &Fake(Err(ErrorCode::JobCancelled)), &p, &Settings::default(), CancelToken::default())
            .await
            .unwrap_err();
        let and = carregar(&db, "e1").unwrap().andamento.unwrap();
        assert_eq!(and.status, JobStatus::Cancelado);
        assert!(and.erro.is_none());
    }

    /// Usa o `claude` instalado e logado (gasta uso da conta). Rodar com:
    /// `cargo test ponta_a_ponta -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn ponta_a_ponta_com_claude_real() {
        let (db, paths, _d) = ambiente();
        let backend = ClaudeCliBackend { caminho_manual: None };
        let settings = Settings::default();

        let p = pedido(&db, EtapaAnalise::Localizar);
        executar(&db, &paths, &backend, &p, &settings, CancelToken::default())
            .await
            .unwrap_or_else(|f| panic!("localizador: {:?}", f.erro));
        let a = carregar(&db, "e1").unwrap();
        let e = edital_repo::get(&db.conn().unwrap(), "e1").unwrap();
        println!("edital: {} | {:?} | prova {:?}", e.nome, e.orgao, e.data_prova);
        println!("cargos: {:#?}", a.cargos);
        assert!(!a.cargos.is_empty());

        analise_repo::escolher_cargo(&db.conn().unwrap(), "e1", &a.cargos[0].id).unwrap();
        let p = pedido(&db, EtapaAnalise::Extrair);
        executar(&db, &paths, &backend, &p, &settings, CancelToken::default())
            .await
            .unwrap_or_else(|f| panic!("extrator: {:?}", f.erro));
        let materias = carregar(&db, "e1").unwrap().materias.unwrap();
        for m in &materias {
            println!("{} (p. {:?})", m.nome, m.pagina);
            for t in &m.topicos {
                println!("  {} {} p.{:?}", if t.verificado { "✓" } else { "?" }, t.texto, t.pagina);
            }
        }
        assert!(!materias.is_empty());
    }

    #[tokio::test]
    async fn saida_invalida_nao_apaga_resultado_anterior() {
        let (db, paths, _d) = ambiente();
        let p = pedido(&db, EtapaAnalise::Localizar);
        executar(&db, &paths, &Fake(Ok(cargos())), &p, &Settings::default(), CancelToken::default())
            .await
            .unwrap();
        let p = pedido(&db, EtapaAnalise::Localizar);
        let f = executar(&db, &paths, &Fake(Ok(json!({"x": 1}))), &p, &Settings::default(), CancelToken::default())
            .await
            .unwrap_err();
        assert_eq!(f.erro.code, ErrorCode::ClaudeOutputInvalid);
        assert_eq!(carregar(&db, "e1").unwrap().cargos.len(), 1);
    }
}
