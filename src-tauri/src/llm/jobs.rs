//! Fila serial de jobs: um `claude` por vez, em ordem de criação, persistida no banco.
//!
//! A fila não conhece o Tauri: fala com o app por `Ambiente`, o que permite testá-la
//! de ponta a ponta com o `fake-claude`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::Value;
use tokio::sync::{watch, Notify};

use super::claude_cli::runner::{self, Execucao, Falha};
use super::claude_cli::stream::{self, Desfecho};
use super::claude_cli::discovery;
use crate::domain::{EstadoFila, Job, JobStatus, NivelUso, PerfilModelo, UsoClaude};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::storage::{config_repo, job_repo, AppPaths, Db};

pub const RESULTADO: &str = "resultado.json";
/// Modelo pedido, resolvido das Configurações na hora de enfileirar.
pub const MODELO: &str = "modelo.txt";

pub trait Ambiente: Send + Sync + 'static {
    fn db(&self) -> &Db;
    fn paths(&self) -> &AppPaths;
    fn notificar(&self, n: Notificacao);
}

#[derive(Debug, Clone)]
pub enum Notificacao {
    /// Algum job mudou de status, ou a fila pausou/retomou.
    Fila(EstadoFila),
    Uso(UsoClaude),
    /// Job chegou a um estado final (concluído, falhou, cancelado).
    JobTerminou(Job),
    /// O `claude` perdeu o login no meio de um job.
    LoginPerdido,
    /// O estado da extração de um edital mudou (id do edital).
    Extracao(String),
}

/// O que um papel (localizador, extrator…) entrega para virar job.
pub struct NovoJob {
    pub edital_id: Option<String>,
    pub tipo: String,
    pub system: String,
    pub prompt: String,
    pub schema: Value,
    pub perfil: PerfilModelo,
}

/// Esperas da fila. Os testes encurtam.
#[derive(Debug, Clone)]
pub struct Esperas {
    /// Antes de cada nova tentativa após `CLAUDE_ERRO_API`; o tamanho é o número de tentativas extras.
    pub erro_api: Vec<Duration>,
    /// Somada ao `resetsAt` antes de retomar.
    pub folga_reset: Duration,
    /// Pausa quando o limite chega sem `resetsAt`.
    pub sem_reset: Duration,
    /// Novas tentativas automáticas para saída fora do schema.
    pub saida_invalida: u32,
}

impl Default for Esperas {
    fn default() -> Self {
        Self {
            erro_api: vec![Duration::from_secs(30), Duration::from_secs(120)],
            folga_reset: Duration::from_secs(60),
            sem_reset: Duration::from_secs(30 * 60),
            saida_invalida: 1,
        }
    }
}

struct Atual {
    job_id: String,
    cancelar: watch::Sender<bool>,
}

pub struct Fila {
    acordar: Notify,
    atual: Mutex<Option<Atual>>,
    pausada_ate: Mutex<Option<DateTime<Utc>>>,
    esperas: Esperas,
}

enum Fim {
    Concluido(Value),
    Limite(Option<i64>),
    Cancelado,
    Falhou(AppError),
    SemLogin(String),
}

fn agora() -> String {
    Utc::now().to_rfc3339()
}

impl Fila {
    /// Recupera a pausa por limite persistida (o app pode ter fechado durante ela).
    pub fn nova(db: &Db, esperas: Esperas) -> Arc<Self> {
        let pausada_ate = db
            .conn()
            .and_then(|c| config_repo::fila_pausada_ate(&c))
            .ok()
            .flatten()
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&Utc));
        Arc::new(Self {
            acordar: Notify::new(),
            atual: Mutex::new(None),
            pausada_ate: Mutex::new(pausada_ate),
            esperas,
        })
    }

    pub fn estado(&self, db: &Db) -> EstadoFila {
        EstadoFila {
            executando: self.atual.lock().ok().and_then(|a| a.as_ref().map(|a| a.job_id.clone())),
            na_fila: db.conn().and_then(|c| job_repo::contar_na_fila(&c)).unwrap_or(0),
            pausada_ate: self.pausa().map(|d| d.to_rfc3339()),
        }
    }

    fn pausa(&self) -> Option<DateTime<Utc>> {
        self.pausada_ate.lock().ok().and_then(|p| *p)
    }

    fn avisar<A: Ambiente>(&self, amb: &A) {
        amb.notificar(Notificacao::Fila(self.estado(amb.db())));
    }

    fn pausar<A: Ambiente>(&self, amb: &A, ate: Option<DateTime<Utc>>) {
        if let Ok(mut p) = self.pausada_ate.lock() {
            *p = ate;
        }
        let iso = ate.map(|d| d.to_rfc3339());
        if let Err(e) = amb
            .db()
            .conn()
            .and_then(|c| config_repo::salvar_fila_pausada_ate(&c, iso.as_deref()))
        {
            tracing::warn!(erro = %e, "falha ao persistir a pausa da fila");
        }
        self.avisar(amb);
    }

    /// Cria o workspace e põe o job na fila. Com utilização alta numa janela e sem
    /// `confirmado`, recusa com `CLAUDE_USO_ALTO` para a UI pedir confirmação.
    pub fn enfileirar<A: Ambiente>(&self, amb: &A, novo: NovoJob, confirmado: bool) -> AppResult<Job> {
        jsonschema::validator_for(&novo.schema).map_err(|e| {
            AppError::new(ErrorCode::Internal, format!("schema do papel {} inválido: {e}", novo.tipo))
        })?;
        let conn = amb.db().conn()?;
        if !confirmado {
            if let Some(uso) = config_repo::uso_claude(&conn)? {
                let altas = uso.janelas_altas(Utc::now());
                if !altas.is_empty() {
                    let detalhe = altas
                        .iter()
                        .map(|(nome, j)| format!("janela de {nome} está em {:.0}%", j.utilizacao * 100.0))
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(AppError::new(ErrorCode::ClaudeUsoAlto, detalhe));
                }
            }
        }

        let id = uuid::Uuid::new_v4().to_string();
        let ws = amb.paths().job_dir(&id);
        std::fs::create_dir_all(&ws)?;
        std::fs::write(ws.join(runner::SYSTEM), &novo.system)?;
        std::fs::write(ws.join(runner::PROMPT), &novo.prompt)?;
        std::fs::write(ws.join(runner::SCHEMA), serde_json::to_string(&novo.schema)?)?;
        let modelo = config_repo::settings(&conn)?.modelo(novo.perfil).to_owned();
        std::fs::write(ws.join(MODELO), &modelo)?;
        job_repo::inserir(&conn, &id, novo.edital_id.as_deref(), &novo.tipo, &agora())?;
        let job = job_repo::get(&conn, &id)?;
        drop(conn);
        tracing::info!(job = %id, tipo = %novo.tipo, "job na fila");
        self.avisar(amb);
        self.acordar.notify_one();
        Ok(job)
    }

    /// Cancela um job na fila ou em execução.
    pub fn cancelar<A: Ambiente>(&self, amb: &A, id: &str) -> AppResult<()> {
        if let Ok(atual) = self.atual.lock() {
            if let Some(a) = atual.as_ref().filter(|a| a.job_id == id) {
                a.cancelar.send_replace(true);
                return Ok(());
            }
        }
        let n = job_repo::cancelar_na_fila(&*amb.db().conn()?, Some(id), &agora())?;
        if n == 0 {
            return Err(AppError::new(
                ErrorCode::JobNotFound,
                format!("job {id} não está na fila nem em execução"),
            ));
        }
        self.avisar(amb);
        Ok(())
    }

    /// Cancela a fila inteira e o job em execução, e espera o processo morrer (até 5 s).
    pub async fn cancelar_tudo<A: Ambiente>(&self, amb: &A) {
        if let Ok(c) = amb.db().conn() {
            let _ = job_repo::cancelar_na_fila(&c, None, &agora());
        }
        if let Ok(atual) = self.atual.lock() {
            if let Some(a) = atual.as_ref() {
                a.cancelar.send_replace(true);
            }
        }
        for _ in 0..100 {
            if self.atual.lock().map(|a| a.is_none()).unwrap_or(true) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        self.avisar(amb);
    }

    /// Cancela os jobs de um edital (antes de removê-lo).
    pub async fn cancelar_do_edital<A: Ambiente>(&self, amb: &A, edital_id: &str) -> AppResult<()> {
        let ids = job_repo::ids_do_edital(&*amb.db().conn()?, edital_id)?;
        for id in &ids {
            let _ = self.cancelar(amb, id);
        }
        for _ in 0..100 {
            let rodando = self
                .atual
                .lock()
                .map(|a| a.as_ref().is_some_and(|a| ids.contains(&a.job_id)))
                .unwrap_or(false);
            if !rodando {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Ok(())
    }

    pub fn retomar_agora<A: Ambiente>(&self, amb: &A) {
        self.pausar(amb, None);
        self.acordar.notify_one();
    }

    /// Laço da fila. Roda enquanto o app estiver aberto.
    pub async fn rodar<A: Ambiente>(self: Arc<Self>, amb: Arc<A>) {
        loop {
            if let Some(ate) = self.pausa() {
                match (ate - Utc::now()).to_std() {
                    Ok(falta) if !falta.is_zero() => {
                        tokio::select! {
                            _ = tokio::time::sleep(falta) => {}
                            _ = self.acordar.notified() => {}
                        }
                        continue;
                    }
                    _ => {
                        tracing::info!("pausa por limite de uso terminou; fila retomada");
                        self.pausar(&*amb, None);
                    }
                }
            }
            match amb.db().conn().and_then(|c| job_repo::proximo_na_fila(&c)) {
                Ok(Some(job)) => self.executar_job(&*amb, job).await,
                Ok(None) => self.acordar.notified().await,
                Err(e) => {
                    tracing::error!(erro = %e, "falha ao ler a fila");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn executar_job<A: Ambiente>(&self, amb: &A, job: Job) {
        let (tx, rx) = watch::channel(false);
        if let Ok(mut a) = self.atual.lock() {
            *a = Some(Atual {
                job_id: job.id.clone(),
                cancelar: tx,
            });
        }
        let fim = match amb.db().conn().and_then(|c| job_repo::marcar_executando(&c, &job.id, &agora())) {
            Ok(true) => {
                self.avisar(amb);
                tracing::info!(job = %job.id, tipo = %job.tipo, "job iniciado");
                self.tentar(amb, &job, rx).await
            }
            Ok(false) => {
                if let Ok(mut a) = self.atual.lock() {
                    *a = None;
                }
                return;
            }
            Err(e) => Fim::Falhou(e),
        };
        let pausar_ate = self.aplicar(amb, &job.id, fim);
        if let Ok(mut a) = self.atual.lock() {
            *a = None;
        }
        match pausar_ate {
            Some(ate) => self.pausar(amb, Some(ate)),
            None => self.avisar(amb),
        }
    }

    /// Executa o job, com as novas tentativas automáticas do catálogo de erros.
    async fn tentar<A: Ambiente>(&self, amb: &A, job: &Job, rx: watch::Receiver<bool>) -> Fim {
        let settings = match amb.db().conn().and_then(|c| config_repo::settings(&c)) {
            Ok(s) => s,
            Err(e) => return Fim::Falhou(e),
        };
        let claude = match discovery::descobrir(settings.claude_path.as_deref()) {
            Ok(c) => c.caminho,
            Err(e) => return Fim::Falhou(e),
        };
        let ws = amb.paths().job_dir(&job.id);
        let schema: Value = match std::fs::read_to_string(ws.join(runner::SCHEMA))
            .map_err(AppError::from)
            .and_then(|s| Ok(serde_json::from_str(&s)?))
        {
            Ok(s) => s,
            Err(e) => return Fim::Falhou(e),
        };
        let modelo = std::fs::read_to_string(ws.join(MODELO))
            .ok()
            .map(|m| m.trim().to_owned())
            .filter(|m| !m.is_empty());
        let ex = Execucao {
            claude: &claude,
            workspace: &ws,
            modelo: modelo.as_deref(),
            timeout: Duration::from_secs(settings.timeout_secs.into()),
        };
        let mut consumo = job.consumo.clone().unwrap_or_default();
        let mut invalidas = 0;
        let mut erros_api = 0;

        loop {
            let saida = runner::executar(&ex, rx.clone(), |info| {
                let uso = info.para_uso(&agora());
                if let Ok(c) = amb.db().conn() {
                    let _ = config_repo::salvar_uso_claude(&c, &uso);
                }
                amb.notificar(Notificacao::Uso(uso));
            })
            .await;

            let s = match saida {
                Err(Falha::Cancelado) => return Fim::Cancelado,
                Err(Falha::Timeout(d)) => {
                    let _ = amb.db().conn().and_then(|c| job_repo::contar_tentativa(&c, &job.id));
                    return Fim::Falhou(AppError::new(
                        ErrorCode::ClaudeTimeout,
                        format!("o job passou de {}s e foi encerrado", d.as_secs()),
                    ));
                }
                Err(Falha::Processo(m)) => {
                    return Fim::Falhou(AppError::new(ErrorCode::ClaudeProcessoFalhou, m))
                }
                Ok(s) => s,
            };

            let rate = s.ultimo_rate_limit.as_ref();
            let recusado = rate.is_some_and(|i| stream::nivel(&i.status) == NivelUso::Limite);
            let reset = rate.and_then(|i| i.resets_at);
            if let Some(info) = rate {
                if !matches!(info.status.as_str(), "allowed" | "rejected") {
                    tracing::warn!(status = %info.status, "status de rate limit não visto antes");
                }
            }
            let uso = rate.map(|i| i.para_uso(&agora()));
            if let Some(r) = &s.resultado {
                consumo.somar(&r.consumo());
                let _ = amb.db().conn().and_then(|c| {
                    job_repo::registrar_consumo(
                        &c,
                        &job.id,
                        &consumo,
                        uso.as_ref().and_then(|u| u.janela_5h.as_ref()).map(|j| j.utilizacao),
                        uso.as_ref().and_then(|u| u.janela_7d.as_ref()).map(|j| j.utilizacao),
                    )
                });
            }

            let Some(r) = s.resultado else {
                if recusado {
                    return Fim::Limite(reset);
                }
                return Fim::Falhou(AppError::new(
                    ErrorCode::ClaudeProcessoFalhou,
                    format!(
                        "o claude saiu (código {:?}) sem resultado. stderr:\n{}",
                        s.codigo_saida, s.stderr_fim
                    ),
                ));
            };

            let desfecho = match r.desfecho() {
                Desfecho::Sucesso(v) => match validar(&schema, &v) {
                    Ok(()) => Desfecho::Sucesso(v),
                    Err(m) => Desfecho::SaidaInvalida(m),
                },
                _ if recusado => Desfecho::Limite,
                d => d,
            };
            if desfecho != Desfecho::Limite {
                let _ = amb.db().conn().and_then(|c| job_repo::contar_tentativa(&c, &job.id));
            }

            match desfecho {
                Desfecho::Sucesso(v) => return Fim::Concluido(v),
                Desfecho::Limite => return Fim::Limite(reset),
                Desfecho::SemLogin(m) => return Fim::SemLogin(m),
                Desfecho::SaidaInvalida(m) => {
                    if invalidas >= self.esperas.saida_invalida {
                        return Fim::Falhou(AppError::new(ErrorCode::ClaudeSaidaInvalida, m));
                    }
                    invalidas += 1;
                    tracing::warn!(job = %job.id, motivo = %m, "saída inválida; nova tentativa");
                }
                Desfecho::ErroApi(m) => {
                    let Some(espera) = self.esperas.erro_api.get(erros_api).copied() else {
                        return Fim::Falhou(AppError::new(ErrorCode::ClaudeErroApi, m));
                    };
                    erros_api += 1;
                    tracing::warn!(job = %job.id, motivo = %m, espera_s = espera.as_secs(), "erro de API; nova tentativa");
                    let mut rx = rx.clone();
                    tokio::select! {
                        _ = tokio::time::sleep(espera) => {}
                        r = rx.wait_for(|c| *c) => if r.is_ok() { return Fim::Cancelado },
                    }
                }
            }
        }
    }

    /// Grava o desfecho. Devolve até quando pausar a fila, se for o caso.
    fn aplicar<A: Ambiente>(&self, amb: &A, id: &str, fim: Fim) -> Option<DateTime<Utc>> {
        let ws = amb.paths().job_dir(id);
        let mut pausar_ate = None;
        let gravado = amb.db().conn().and_then(|c| match fim {
            Fim::Concluido(v) => {
                let json = serde_json::to_string(&v)?;
                std::fs::write(ws.join(RESULTADO), serde_json::to_string_pretty(&v)?)?;
                job_repo::concluir(&c, id, &json, &agora())
            }
            Fim::Limite(reset) => {
                let ate = reset
                    .and_then(|s| DateTime::from_timestamp(s, 0))
                    .map(|d| d + self.esperas.folga_reset)
                    .unwrap_or_else(|| Utc::now() + self.esperas.sem_reset);
                tracing::warn!(job = %id, ate = %ate, "limite de uso; job volta para a fila e a fila pausa");
                pausar_ate = Some(ate);
                job_repo::voltar_para_fila(&c, id)
            }
            Fim::Cancelado => job_repo::finalizar(&c, id, JobStatus::Cancelado, None, &agora()),
            Fim::Falhou(e) => {
                tracing::warn!(job = %id, code = ?e.code, detail = %e.detail, "job falhou");
                job_repo::finalizar(&c, id, JobStatus::Falhou, Some(&e), &agora())
            }
            Fim::SemLogin(m) => {
                amb.notificar(Notificacao::LoginPerdido);
                let e = AppError::new(ErrorCode::ClaudeNotLoggedIn, m);
                job_repo::finalizar(&c, id, JobStatus::Falhou, Some(&e), &agora())
            }
        });
        if let Err(e) = gravado {
            tracing::error!(job = %id, erro = %e, "falha ao gravar o desfecho do job");
        }
        if let Ok(job) = amb.db().conn().and_then(|c| job_repo::get(&c, id)) {
            if job.status != JobStatus::NaFila {
                tracing::info!(job = %id, status = ?job.status, "job terminou");
                amb.notificar(Notificacao::JobTerminou(job));
            }
        }
        pausar_ate
    }
}

/// Validação no core, independente do `--json-schema` do CLI.
pub fn validar(schema: &Value, v: &Value) -> Result<(), String> {
    let validador = jsonschema::validator_for(schema).map_err(|e| e.to_string())?;
    let erros: Vec<String> = validador
        .iter_errors(v)
        .take(5)
        .map(|e| format!("{}: {e}", e.instance_path()))
        .collect();
    if erros.is_empty() {
        Ok(())
    } else {
        Err(format!("fora do schema: {}", erros.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valida_contra_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {"ok": {"type": "boolean"}},
            "required": ["ok"]
        });
        assert!(validar(&schema, &serde_json::json!({"ok": true})).is_ok());
        let e = validar(&schema, &serde_json::json!({"x": 1})).unwrap_err();
        assert!(e.contains("ok"), "{e}");
    }
}
