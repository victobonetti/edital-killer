use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Settings {
    /// Caminho manual do `claude`. `None` = descoberta automática.
    pub claude_path: Option<String>,
    /// Modelo dos papéis simples (triagem, dados gerais, cargos, cronograma).
    #[serde(default = "padrao_rapido")]
    pub modelo_rapido: String,
    /// Modelo dos papéis pesados (provas, conteúdo programático).
    #[serde(default = "padrao_forte")]
    pub modelo_forte: String,
    /// Tempo limite de cada job, em segundos.
    pub timeout_secs: u32,
}

fn padrao_rapido() -> String {
    "haiku".into()
}

fn padrao_forte() -> String {
    "sonnet".into()
}

/// Qual dos dois modelos das Configurações um papel usa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfilModelo {
    Rapido,
    Forte,
}

impl Settings {
    pub fn modelo(&self, perfil: PerfilModelo) -> &str {
        match perfil {
            PerfilModelo::Rapido => &self.modelo_rapido,
            PerfilModelo::Forte => &self.modelo_forte,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            claude_path: None,
            modelo_rapido: padrao_rapido(),
            modelo_forte: padrao_forte(),
            timeout_secs: 600,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_antigas_ganham_os_modelos_padrao() {
        let s: Settings =
            serde_json::from_str(r#"{"claude_path":null,"model":"opus","timeout_secs":300}"#).unwrap();
        assert_eq!((s.modelo_rapido.as_str(), s.modelo_forte.as_str()), ("haiku", "sonnet"));
        assert_eq!(s.timeout_secs, 300);
    }
}
