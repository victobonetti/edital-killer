use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::ErrorCode;

/// Categoria de seção dada pela triagem. Uma seção pode ter várias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Categoria {
    DadosGerais,
    Inscricao,
    CargosVagas,
    RequisitosAtribuicoes,
    Cronograma,
    ProvasAvaliacao,
    ConteudoProgramatico,
    Outros,
}

/// Bloco de extração (papel). `Triagem` também passa pela tabela `extracao`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Bloco {
    Triagem,
    DadosGerais,
    Cargos,
    Cronograma,
    Provas,
    Conteudo,
}

impl Bloco {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Triagem => "triagem",
            Self::DadosGerais => "dados_gerais",
            Self::Cargos => "cargos",
            Self::Cronograma => "cronograma",
            Self::Provas => "provas",
            Self::Conteudo => "conteudo",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::String(s.to_owned())).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum StatusExtracao {
    /// Job na fila ou executando.
    Pendente,
    Concluido,
    Falhou,
}

impl StatusExtracao {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pendente => "pendente",
            Self::Concluido => "concluido",
            Self::Falhou => "falhou",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "pendente" => Self::Pendente,
            "concluido" => Self::Concluido,
            "falhou" => Self::Falhou,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SecaoEdital {
    pub ordem: u32,
    pub titulo: String,
    pub pagina_ini: u32,
    pub pagina_fim: u32,
    /// `None` até a triagem terminar.
    pub categorias: Option<Vec<Categoria>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Extracao {
    pub id: String,
    pub bloco: Bloco,
    pub cargo_id: Option<String>,
    pub job_id: Option<String>,
    pub versao: u32,
    pub status: StatusExtracao,
    pub erro_code: Option<ErrorCode>,
    pub erro_detail: Option<String>,
    pub atualizado_em: String,
}

/// O que a tela do Edital mostra sobre o processamento.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EstadoExtracao {
    pub edital_id: String,
    /// Resultado da triagem (`None` antes dela).
    pub e_edital: Option<bool>,
    pub triagem_motivo: Option<String>,
    pub secoes: Vec<SecaoEdital>,
    /// Mais recente de cada bloco (e cargo) primeiro.
    pub extracoes: Vec<Extracao>,
}
