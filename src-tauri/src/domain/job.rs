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
}
