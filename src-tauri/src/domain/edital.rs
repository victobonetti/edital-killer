use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EditalStatus {
    /// PDF copiado, texto das páginas ainda chegando da UI.
    Importando,
    /// Texto extraído e salvo; pronto para processamento.
    Importado,
    /// Nenhuma página com texto: processamento bloqueado (sem OCR no Passo 1).
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
    /// Números (1-based) das páginas sem texto extraível.
    pub paginas_sem_texto: Vec<u32>,
    pub status: EditalStatus,
    /// ISO 8601 (UTC).
    pub importado_em: String,
}

/// Trecho de texto da camada do pdf.js, com o índice onde começa no texto da página.
/// Índices são em unidades UTF-16 (o que `String.length` mede no JS).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ItemTexto {
    pub t: String,
    pub i: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PaginaInput {
    /// 1-based.
    pub pagina: u32,
    pub texto: String,
    pub itens: Vec<ItemTexto>,
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
