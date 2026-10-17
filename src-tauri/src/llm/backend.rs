use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::llm::cancel::CancelToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum BackendEstado {
    Verificando,
    Pronto,
    /// Executando um job.
    Ocupado,
    /// Em limite de uso; `retorno_em` traz o horário quando conhecido.
    LimiteUso,
    Indisponivel,
}

/// Resultado do health check, também usado como estado do indicador da barra de status.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BackendStatus {
    pub estado: BackendEstado,
    /// Executável encontrado (`None` = não encontrado).
    pub caminho: Option<String>,
    /// Como o caminho foi obtido: `manual`, `path` ou `local_conhecido`.
    pub origem_caminho: Option<String>,
    pub versao: Option<String>,
    /// `None` quando não foi possível saber.
    pub logado: Option<bool>,
    /// Método de login informado pelo CLI (ex.: `claude.ai`). Nunca credenciais.
    pub metodo_login: Option<String>,
    pub erro: Option<AppError>,
    /// ISO 8601, quando em limite de uso e o horário é conhecido.
    pub retorno_em: Option<String>,
    /// ISO 8601 da última verificação.
    pub verificado_em: Option<String>,
}

impl BackendStatus {
    pub fn verificando() -> Self {
        Self {
            estado: BackendEstado::Verificando,
            caminho: None,
            origem_caminho: None,
            versao: None,
            logado: None,
            metodo_login: None,
            erro: None,
            retorno_em: None,
            verificado_em: None,
        }
    }
}

/// Uma chamada ao LLM: prompt, schema da saída e onde trabalhar.
#[derive(Debug, Clone)]
pub struct Execucao {
    /// Pasta do job; o LLM só lê arquivos daqui.
    pub workspace: PathBuf,
    pub prompt: String,
    /// JSON Schema (texto) que a saída precisa seguir.
    pub schema: String,
    pub modelo: Option<String>,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct FalhaJob {
    pub erro: AppError,
    /// Fim do limite de uso, quando conhecido (ISO 8601).
    pub retorno_em: Option<String>,
}

/// Backend de LLM usado pelo core. No Passo 1 a única implementação é
/// `ClaudeCliBackend`; o adaptador BYOK (API key) entra depois por esta interface.
#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn health_check(&self) -> BackendStatus;
    /// Executa um job e devolve a saída estruturada (ainda não validada pelo core).
    async fn executar(&self, ex: Execucao, cancel: CancelToken) -> Result<serde_json::Value, FalhaJob>;
}
