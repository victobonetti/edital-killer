//! Backend que usa o `claude` instalado e logado pelo próprio usuário.
//! O app nunca lê nem armazena credenciais do Claude.

pub mod discovery;
pub mod health;
mod processo;
pub mod runner;

use async_trait::async_trait;

use super::backend::{BackendEstado, BackendStatus, Execucao, FalhaJob, LlmBackend};
use super::cancel::CancelToken;
use crate::error::{AppError, ErrorCode};

pub struct ClaudeCliBackend {
    /// Caminho manual das Configurações (`None` = descoberta automática).
    pub caminho_manual: Option<String>,
}

#[async_trait]
impl LlmBackend for ClaudeCliBackend {
    async fn health_check(&self) -> BackendStatus {
        let mut st = BackendStatus::verificando();
        st.verificado_em = Some(chrono::Utc::now().to_rfc3339());

        let indisponivel = |mut st: BackendStatus, erro: AppError| {
            st.estado = BackendEstado::Indisponivel;
            st.erro = Some(erro);
            st
        };

        let encontrado = match discovery::descobrir(self.caminho_manual.as_deref()) {
            Ok(e) => e,
            Err(e) => return indisponivel(st, e),
        };
        st.caminho = Some(encontrado.caminho.display().to_string());
        st.origem_caminho = Some(encontrado.origem.to_owned());

        match health::versao(&encontrado.caminho).await {
            Ok(v) => st.versao = Some(v),
            Err(e) => return indisponivel(st, e),
        }

        match health::login(&encontrado.caminho).await {
            Ok(Some(login)) => {
                st.logado = Some(login.logado);
                st.metodo_login = login.metodo;
                if !login.logado {
                    let erro = AppError::new(
                        ErrorCode::ClaudeNotLoggedIn,
                        "claude auth status: loggedIn = false",
                    );
                    return indisponivel(st, erro);
                }
            }
            // Formato desconhecido: não bloqueia; o primeiro job revela o problema.
            Ok(None) => tracing::warn!("auth status em formato desconhecido"),
            Err(e) => return indisponivel(st, e),
        }

        st.estado = BackendEstado::Pronto;
        st
    }

    async fn executar(&self, ex: Execucao, cancel: CancelToken) -> Result<serde_json::Value, FalhaJob> {
        let encontrado = discovery::descobrir(self.caminho_manual.as_deref())
            .map_err(|erro| FalhaJob { erro, retorno_em: None })?;
        runner::executar(&encontrado.caminho, ex, cancel).await
    }
}
