use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::qualidade::{OrigemTexto, QualidadeTexto};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EditalStatus {
    /// PDF copiado, texto das páginas ainda chegando da UI.
    Importando,
    /// Texto extraído e salvo; pronto para processamento.
    Importado,
    /// Nenhuma página com texto legível, nem depois do OCR: processamento bloqueado.
    SemTexto,
}

impl EditalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Importando => "importando",
            Self::Importado => "importado",
            Self::SemTexto => "sem_texto",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "importando" => Some(Self::Importando),
            "importado" => Some(Self::Importado),
            "sem_texto" => Some(Self::SemTexto),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Edital {
    pub id: String,
    pub nome: String,
    pub orgao: Option<String>,
    pub banca: Option<String>,
    /// Nome do arquivo que o usuário importou (só informativo).
    pub arquivo_original: String,
    pub sha256: String,
    pub total_paginas: Option<u32>,
    /// Páginas sem texto ou com texto ilegível, depois do OCR.
    pub paginas_problema: Vec<PaginaProblema>,
    /// Números (1-based) das páginas cujo texto veio do OCR.
    pub paginas_ocr: Vec<u32>,
    /// Mais de 20% das páginas com problema: processar exige confirmação.
    pub exige_confirmacao: bool,
    pub status: EditalStatus,
    /// ISO 8601 (UTC).
    pub importado_em: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PaginaProblema {
    pub pagina: u32,
    pub qualidade: QualidadeTexto,
}

/// Trecho de texto (item do pdf.js ou palavra do OCR), com o índice onde começa no
/// texto da página. Índices são em unidades UTF-16 (o que `String.length` mede no JS).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ItemTexto {
    pub t: String,
    pub i: u32,
    /// Só no OCR: caixa `[x, y, largura, altura]` em pontos, a partir do canto superior
    /// esquerdo da página (viewport do pdf.js em escala 1). Permite destacar sem camada de texto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub b: Option<[f32; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PaginaInput {
    /// 1-based.
    pub pagina: u32,
    pub texto: String,
    pub itens: Vec<ItemTexto>,
    #[serde(default)]
    pub origem: OrigemTexto,
    /// Confiança média do tesseract (0–100), só no OCR.
    #[serde(default)]
    #[ts(optional)]
    pub confianca: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImportStart {
    pub edital: Edital,
    /// `true` quando o mesmo PDF (mesmo SHA-256) já estava importado.
    pub duplicado: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EditalUpdate {
    pub nome: String,
    pub orgao: Option<String>,
    pub banca: Option<String>,
}
