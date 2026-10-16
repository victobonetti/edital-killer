use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Settings {
    /// Caminho manual do `claude`. `None` = descoberta automática.
    pub claude_path: Option<String>,
    /// Modelo passado ao `claude --model`. `None` = padrão do CLI.
    pub model: Option<String>,
    /// Tempo limite de cada job, em segundos.
    pub timeout_secs: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            claude_path: None,
            model: None,
            timeout_secs: 600,
        }
    }
}
