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
    /// O OCR de uma página falhou (worker, dado de idioma). A página fica como estava.
    ImportOcrFailed,
    /// O usuário cancelou a importação (ex.: durante o OCR).
    ImportCancelado,
    /// Edital não encontrado.
    EditalNotFound,
    /// Edital sem texto legível, nem depois do OCR: processamento bloqueado.
    EditalSemTexto,
    /// Executável `claude` não encontrado.
    ClaudeNotFound,
    /// `claude` encontrado, mas não executou (`--version` falhou).
    ClaudeNotRunnable,
    /// `claude` sem login.
    ClaudeNotLoggedIn,
    /// `claude` não respondeu dentro do tempo limite.
    ClaudeTimeout,
    /// Limite de uso do plano atingido; o job volta para a fila.
    ClaudeLimiteUso,
    /// Utilização da janela acima de 80%: enfileirar exige confirmação.
    ClaudeUsoAlto,
    /// Resposta sem `structured_output` ou fora do schema.
    ClaudeSaidaInvalida,
    /// Erro da API (5xx, sobrecarga) informado no `result`.
    ClaudeErroApi,
    /// Processo saiu sem evento `result`.
    ClaudeProcessoFalhou,
    /// Job não existe ou não está num estado que permita a operação.
    JobNotFound,
    /// O usuário cancelou o job.
    JobCancelado,
    /// O app fechou com o job em execução.
    JobInterrompido,
    /// Mais de 20% das páginas com problema: processar exige confirmação.
    EditalConfirmarPaginas,
    /// A triagem diz que o PDF não é um edital de abertura (motivo no `detail`).
    EditalNaoEEdital,
    /// A triagem não achou seção para o bloco.
    ExtracaoSemSecoes,
    /// A verificação literal reprovou mais de 30% dos itens do bloco.
    ExtracaoIncompleta,
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
