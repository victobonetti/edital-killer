//! Resultado da análise de um edital: cargos, conteúdo programático e o andamento
//! do processamento, como a UI precisa exibir.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;

use super::JobStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Cargo {
    pub id: String,
    pub nome: String,
    pub vagas: Option<u32>,
    pub escolaridade: Option<String>,
    /// Página (1-based) onde o cargo aparece no edital.
    pub pagina: Option<u32>,
    /// O nome foi encontrado literalmente no texto do edital.
    pub verificado: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Topico {
    pub id: String,
    pub texto: String,
    pub pagina: Option<u32>,
    /// Encontrado literalmente no texto; `false` = conferir no PDF.
    pub verificado: bool,
    /// Offsets UTF-16 no texto original da página (destaque no PDF).
    pub inicio: Option<u32>,
    pub fim: Option<u32>,
    pub filhos: Vec<Topico>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Materia {
    pub id: String,
    pub nome: String,
    pub pagina: Option<u32>,
    /// Bloco da prova (ex.: "Conhecimentos Gerais"), do quadro de provas.
    pub bloco: Option<String>,
    /// Questões da matéria; `None` quando o edital só informa por bloco.
    pub questoes: Option<u32>,
    pub peso: Option<f64>,
    /// Bloco, questões e peso foram confirmados no texto do edital.
    pub quadro_verificado: bool,
    pub topicos: Vec<Topico>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BlocoProva {
    pub nome: String,
    /// Questões do bloco, quando o edital informa por bloco.
    pub questoes: Option<u32>,
    /// Percentual (0–100) dos pontos do bloco exigido para não ser eliminado.
    pub minimo: Option<f64>,
    pub verificado: bool,
}

/// Quadro de provas do cargo escolhido.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EstruturaProva {
    pub duracao_minutos: Option<u32>,
    pub duracao_verificada: bool,
    /// Percentual (0–100) do total de pontos exigido.
    pub minimo_geral: Option<f64>,
    pub minimo_verificado: bool,
    pub blocos: Vec<BlocoProva>,
}

/// Etapa da análise que um job executa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EtapaAnalise {
    /// Lê metadados e localiza os cargos.
    Localizar,
    /// Extrai o conteúdo programático do cargo escolhido.
    Extrair,
}

impl EtapaAnalise {
    /// Valor da coluna `job.tipo`.
    pub fn tipo_job(self) -> &'static str {
        match self {
            Self::Localizar => "localizador",
            Self::Extrair => "extrator",
        }
    }

    pub fn do_tipo_job(tipo: &str) -> Option<Self> {
        match tipo {
            "localizador" => Some(Self::Localizar),
            "extrator" => Some(Self::Extrair),
            _ => None,
        }
    }
}

/// Último processamento do edital, em termos que a UI mostra sem expor jobs.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Andamento {
    pub etapa: EtapaAnalise,
    pub status: JobStatus,
    /// ISO 8601; início da execução (ou criação, se ainda na fila).
    pub desde: String,
    pub erro: Option<AppError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Analise {
    /// Vazio = cargos ainda não localizados.
    pub cargos: Vec<Cargo>,
    pub cargo_escolhido: Option<String>,
    /// `None` = conteúdo do cargo escolhido ainda não extraído.
    pub materias: Option<Vec<Materia>>,
    /// Quadro de provas do cargo escolhido, extraído junto com o conteúdo.
    pub estrutura: Option<EstruturaProva>,
    pub andamento: Option<Andamento>,
}
