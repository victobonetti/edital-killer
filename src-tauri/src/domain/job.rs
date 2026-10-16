use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum JobStatus {
    NaFila,
    Executando,
    Concluido,
    Falhou,
    Cancelado,
    /// Estava executando quando o app fechou.
    Interrompido,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NaFila => "na_fila",
            Self::Executando => "executando",
            Self::Concluido => "concluido",
            Self::Falhou => "falhou",
            Self::Cancelado => "cancelado",
            Self::Interrompido => "interrompido",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "na_fila" => Self::NaFila,
            "executando" => Self::Executando,
            "concluido" => Self::Concluido,
            "falhou" => Self::Falhou,
            "cancelado" => Self::Cancelado,
            "interrompido" => Self::Interrompido,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Job {
    pub id: String,
    pub edital_id: Option<String>,
    /// Papel do job (ex.: `localizador`, `extrator`).
    pub tipo: String,
    pub status: JobStatus,
    pub criado_em: String,
    pub iniciado_em: Option<String>,
    pub finalizado_em: Option<String>,
    pub erro_code: Option<ErrorCode>,
    pub erro_detail: Option<String>,
    /// Execuções do `claude` feitas para este job (recusas por limite de uso não contam).
    pub tentativas: u32,
    /// Somado entre as tentativas. `None` até a primeira execução terminar.
    pub consumo: Option<ConsumoJob>,
    /// Utilização das janelas vista no último `rate_limit_event` do job (0.0–1.0).
    pub utilizacao_5h: Option<f64>,
    pub utilizacao_7d: Option<f64>,
}

/// Consumo de uma ou mais execuções, lido do evento `result` do CLI.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ConsumoJob {
    pub modelo: Option<String>,
    pub tokens_entrada: u32,
    pub tokens_saida: u32,
    pub tokens_cache_escrita: u32,
    pub tokens_cache_leitura: u32,
    pub tokens_raciocinio: u32,
    /// Preço de tabela da API. Com login do claude.ai não é cobrado.
    pub custo_estimado_usd: f64,
    pub duracao_ms: u32,
}

impl ConsumoJob {
    pub fn somar(&mut self, outro: &ConsumoJob) {
        if outro.modelo.is_some() {
            self.modelo.clone_from(&outro.modelo);
        }
        self.tokens_entrada += outro.tokens_entrada;
        self.tokens_saida += outro.tokens_saida;
        self.tokens_cache_escrita += outro.tokens_cache_escrita;
        self.tokens_cache_leitura += outro.tokens_cache_leitura;
        self.tokens_raciocinio += outro.tokens_raciocinio;
        self.custo_estimado_usd += outro.custo_estimado_usd;
        self.duracao_ms += outro.duracao_ms;
    }
}

/// Estado da fila, emitido no evento `jobs-atualizados`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EstadoFila {
    /// Job em execução.
    pub executando: Option<String>,
    pub na_fila: u32,
    /// ISO 8601: fila pausada por limite de uso até este horário.
    pub pausada_ate: Option<String>,
}
