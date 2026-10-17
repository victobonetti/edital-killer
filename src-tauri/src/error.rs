//! `AppError` e o catálogo de códigos. Todo erro que cruza a fronteira IPC
//! é um `AppError { code, detail }`; a UI traduz `code` em tela explicativa.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum ErrorCode {
    /// Falha inesperada no core.
    Internal,
    /// Falha no SQLite.
    Database,
    /// Falha de leitura/escrita em disco.
    Io,
    /// Parâmetro inválido vindo da UI.
    InvalidInput,
    /// Arquivo não começa com cabeçalho `%PDF`.
    ImportNotPdf,
    /// PDF exige senha para abrir.
    ImportEncrypted,
    /// pdf.js não conseguiu abrir o PDF.
    ImportCorrupted,
    /// Edital não encontrado.
    EditalNotFound,
    /// Edital sem texto extraível: processamento bloqueado até existir OCR.
    EditalSemTexto,
    /// Executável `claude` não encontrado.
    ClaudeNotFound,
    /// `claude` encontrado, mas não executou (`--version` falhou).
    ClaudeNotRunnable,
    /// `claude` sem login.
    ClaudeNotLoggedIn,
    /// `claude` não respondeu dentro do tempo limite.
    ClaudeTimeout,
    /// `claude` executou, mas terminou com erro (API, rede, processo).
    ClaudeExecFailed,
    /// Conta em limite de uso; o status do backend traz `retorno_em` quando conhecido.
    ClaudeUsageLimit,
    /// O modelo configurado não está disponível para a conta (ex.: exige créditos).
    ClaudeModelUnavailable,
    /// Saída do `claude` não é JSON válido ou não segue o schema.
    ClaudeOutputInvalid,
    /// Job cancelado pelo usuário.
    JobCancelled,
    /// Já existe uma análise na fila ou executando para este edital.
    AnaliseEmAndamento,
    /// O localizador não encontrou nenhum cargo verificável no edital.
    CargosNaoEncontrados,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, thiserror::Error)]
#[error("{code:?}: {detail}")]
#[ts(export)]
pub struct AppError {
    pub code: ErrorCode,
    /// Detalhe técnico, mostrado como informação secundária (nunca como mensagem principal).
    pub detail: String,
}

impl AppError {
    pub fn new(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::new(ErrorCode::Database, e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::new(ErrorCode::Io, e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(ErrorCode::Internal, e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
