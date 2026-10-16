//! Leitura do `--output-format stream-json`: uma linha JSON por evento.
//! Só dois eventos importam ao core: `rate_limit_event` e `result`. O resto é ignorado.

use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::domain::{ConsumoJob, JanelaUso, NivelUso, UsoClaude};

#[derive(Debug, Clone, PartialEq)]
pub enum Evento {
    RateLimit(RateLimitInfo),
    Resultado(Resultado),
    Outro,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitInfo {
    pub status: String,
    /// Epoch em segundos.
    pub resets_at: Option<i64>,
    #[serde(default)]
    pub unified_windows: HashMap<String, JanelaBruta>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JanelaBruta {
    pub utilization: Option<f64>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Resultado {
    #[serde(default)]
    pub subtype: String,
    #[serde(default)]
    pub is_error: bool,
    pub result: Option<String>,
    pub structured_output: Option<Value>,
    pub total_cost_usd: Option<f64>,
    pub duration_ms: Option<u64>,
    /// Número ou `null`; algumas versões podem mandar string.
    pub api_error_status: Option<Value>,
    pub usage: Option<Usage>,
    #[serde(rename = "modelUsage", default)]
    pub model_usage: HashMap<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    pub output_tokens_details: Option<OutputDetails>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct OutputDetails {
    #[serde(default)]
    pub thinking_tokens: u64,
}

/// Linha que não é JSON ou não tem `type` conhecido vira `Outro`.
pub fn parse_linha(linha: &str) -> Evento {
    let Ok(v) = serde_json::from_str::<Value>(linha) else {
        return Evento::Outro;
    };
    match v.get("type").and_then(Value::as_str) {
        Some("rate_limit_event") => v
            .get("rate_limit_info")
            .and_then(|i| serde_json::from_value(i.clone()).ok())
            .map_or(Evento::Outro, Evento::RateLimit),
        Some("result") => serde_json::from_value(v).map_or(Evento::Outro, Evento::Resultado),
        _ => Evento::Outro,
    }
}

pub fn nivel(status: &str) -> NivelUso {
    match status {
        "allowed" => NivelUso::Normal,
        "rejected" => NivelUso::Limite,
        _ => NivelUso::Aviso,
    }
}

fn iso(epoch: Option<i64>) -> Option<String> {
    epoch
        .and_then(|s| Utc.timestamp_opt(s, 0).single())
        .map(|d: DateTime<Utc>| d.to_rfc3339())
}

impl RateLimitInfo {
    pub fn para_uso(&self, agora: &str) -> UsoClaude {
        let janela = |nome: &str| {
            self.unified_windows.get(nome).and_then(|j| {
                j.utilization.map(|u| JanelaUso {
                    utilizacao: u,
                    renova_em: iso(j.resets_at),
                })
            })
        };
        UsoClaude {
            nivel: nivel(&self.status),
            status: self.status.clone(),
            janela_5h: janela("five_hour"),
            janela_7d: janela("seven_day"),
            renova_em: iso(self.resets_at),
            atualizado_em: agora.to_owned(),
        }
    }
}

/// Como o job terminou, do ponto de vista do `result`.
#[derive(Debug, Clone, PartialEq)]
pub enum Desfecho {
    Sucesso(Value),
    Limite,
    SemLogin(String),
    ErroApi(String),
    SaidaInvalida(String),
}

impl Resultado {
    pub fn status_http(&self) -> Option<u16> {
        match self.api_error_status.as_ref()? {
            Value::Number(n) => n.as_u64().and_then(|n| u16::try_from(n).ok()),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn desfecho(&self) -> Desfecho {
        let texto = self.result.clone().unwrap_or_default();
        if self.is_error {
            let minusculo = texto.to_lowercase();
            return match self.status_http() {
                Some(429) => Desfecho::Limite,
                Some(401 | 403) => Desfecho::SemLogin(texto),
                _ if minusculo.contains("/login") || minusculo.contains("invalid api key") => {
                    Desfecho::SemLogin(texto)
                }
                _ if minusculo.contains("usage limit") || minusculo.contains("hit your limit") => {
                    Desfecho::Limite
                }
                Some(s) => Desfecho::ErroApi(format!("HTTP {s}: {texto}")),
                None => Desfecho::ErroApi(format!("{}: {texto}", self.subtype)),
            };
        }
        match &self.structured_output {
            Some(v) if self.subtype == "success" => Desfecho::Sucesso(v.clone()),
            Some(_) => Desfecho::SaidaInvalida(format!("subtype {}", self.subtype)),
            None => Desfecho::SaidaInvalida(format!(
                "sem structured_output (subtype {}): {}",
                self.subtype,
                texto.chars().take(300).collect::<String>()
            )),
        }
    }

    pub fn consumo(&self) -> ConsumoJob {
        let u = self.usage.clone().unwrap_or_default();
        let n = |v: u64| u32::try_from(v).unwrap_or(u32::MAX);
        ConsumoJob {
            // Com mais de um modelo (ex.: fallback), fica o de maior custo.
            modelo: self
                .model_usage
                .iter()
                .max_by(|a, b| {
                    let custo = |v: &Value| v.get("costUSD").and_then(Value::as_f64).unwrap_or(0.0);
                    custo(a.1).total_cmp(&custo(b.1))
                })
                .map(|(m, _)| m.clone()),
            tokens_entrada: n(u.input_tokens),
            tokens_saida: n(u.output_tokens),
            tokens_cache_escrita: n(u.cache_creation_input_tokens),
            tokens_cache_leitura: n(u.cache_read_input_tokens),
            tokens_raciocinio: n(u.output_tokens_details.map_or(0, |d| d.thinking_tokens)),
            custo_estimado_usd: self.total_cost_usd.unwrap_or(0.0),
            duracao_ms: n(self.duration_ms.unwrap_or(0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Linhas reais do CLI 2.1.301 (2026-10-16), encurtadas.
    const RATE: &str = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1792186800,"rateLimitType":"five_hour","overageStatus":"rejected","isUsingOverage":false,"unifiedWindows":{"five_hour":{"utilization":0.08,"resetsAt":1792186800},"seven_day":{"utilization":0.21,"resetsAt":1792677600}}},"uuid":"u","session_id":"s"}"#;
    const RESULT: &str = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":2304,"total_cost_usd":0.0026983,"usage":{"input_tokens":2,"cache_creation_input_tokens":13203,"cache_read_input_tokens":0,"output_tokens":115,"output_tokens_details":{"thinking_tokens":62}},"modelUsage":{"claude-haiku-5-5":{"costUSD":0.0026983}},"api_error_status":null,"result":"{\"r\":\"ok\"}","structured_output":{"r":"ok"}}"#;

    #[test]
    fn rate_limit_real() {
        let Evento::RateLimit(info) = parse_linha(RATE) else {
            panic!("esperava rate limit")
        };
        let uso = info.para_uso("agora");
        assert_eq!(uso.nivel, NivelUso::Normal);
        assert_eq!(uso.janela_5h.as_ref().unwrap().utilizacao, 0.08);
        assert_eq!(uso.janela_7d.as_ref().unwrap().utilizacao, 0.21);
        assert!(uso.renova_em.as_deref().unwrap().starts_with("2026-10-16T21:40:00"));
        assert!(uso.janelas_altas(chrono::Utc::now()).is_empty());
    }

    #[test]
    fn result_real() {
        let Evento::Resultado(r) = parse_linha(RESULT) else {
            panic!("esperava result")
        };
        assert_eq!(r.desfecho(), Desfecho::Sucesso(serde_json::json!({"r": "ok"})));
        let c = r.consumo();
        assert_eq!(c.modelo.as_deref(), Some("claude-haiku-5-5"));
        assert_eq!(c.tokens_cache_escrita, 13203);
        assert_eq!(c.tokens_raciocinio, 62);
        assert_eq!(c.duracao_ms, 2304);
    }

    #[test]
    fn erros_classificados() {
        let r = |status: &str, texto: &str| {
            let linha = format!(
                r#"{{"type":"result","subtype":"error_during_execution","is_error":true,"api_error_status":{status},"result":"{texto}"}}"#
            );
            match parse_linha(&linha) {
                Evento::Resultado(r) => r.desfecho(),
                e => panic!("{e:?}"),
            }
        };
        assert_eq!(r("429", "x"), Desfecho::Limite);
        assert_eq!(r("null", "You've hit your limit · resets 3pm"), Desfecho::Limite);
        assert!(matches!(r("401", "x"), Desfecho::SemLogin(_)));
        assert!(matches!(r("null", "Please run /login"), Desfecho::SemLogin(_)));
        assert!(matches!(r("529", "Overloaded"), Desfecho::ErroApi(_)));
        assert!(matches!(r("\"500\"", "x"), Desfecho::ErroApi(m) if m.starts_with("HTTP 500")));
    }

    #[test]
    fn sem_structured_output_e_invalido() {
        let linha = r#"{"type":"result","subtype":"success","is_error":false,"result":"texto solto"}"#;
        let Evento::Resultado(r) = parse_linha(linha) else {
            panic!()
        };
        assert!(matches!(r.desfecho(), Desfecho::SaidaInvalida(_)));
    }

    #[test]
    fn status_desconhecido_e_aviso_e_lixo_e_outro() {
        assert_eq!(nivel("allowed_warning"), NivelUso::Aviso);
        assert_eq!(nivel("rejected"), NivelUso::Limite);
        assert_eq!(parse_linha("não é json"), Evento::Outro);
        assert_eq!(parse_linha(r#"{"type":"assistant"}"#), Evento::Outro);
    }
}
