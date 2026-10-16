use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Utilização ≥ este valor pede confirmação antes de enfileirar um job.
pub const UTILIZACAO_ALTA: f64 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NivelUso {
    /// `status = allowed`.
    Normal,
    /// O CLI sinalizou aviso (qualquer status que não seja `allowed` nem recusa).
    Aviso,
    /// Limite atingido: jobs são recusados até a renovação.
    Limite,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JanelaUso {
    /// 0.0–1.0.
    pub utilizacao: f64,
    /// ISO 8601.
    pub renova_em: Option<String>,
}

/// Último estado de uso do plano visto num `rate_limit_event`. Persistido em `config`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UsoClaude {
    pub nivel: NivelUso,
    /// `status` cru do CLI, para diagnóstico.
    pub status: String,
    pub janela_5h: Option<JanelaUso>,
    pub janela_7d: Option<JanelaUso>,
    /// ISO 8601: quando o limite que está valendo renova.
    pub renova_em: Option<String>,
    /// ISO 8601.
    pub atualizado_em: String,
}

impl UsoClaude {
    /// Janelas com utilização alta que ainda não renovaram em `agora`.
    pub fn janelas_altas(&self, agora: DateTime<Utc>) -> Vec<(&'static str, &JanelaUso)> {
        let renovou = |j: &JanelaUso| {
            j.renova_em
                .as_deref()
                .and_then(|r| DateTime::parse_from_rfc3339(r).ok())
                .is_some_and(|r| r <= agora)
        };
        [("5 h", &self.janela_5h), ("7 dias", &self.janela_7d)]
            .into_iter()
            .filter_map(|(nome, j)| j.as_ref().map(|j| (nome, j)))
            .filter(|(_, j)| j.utilizacao >= UTILIZACAO_ALTA && !renovou(j))
            .collect()
    }
}
