//! Qualidade do texto de uma página: separa o PDF que tem camada de texto legível
//! do que tem texto ilegível (fonte sem mapa de caracteres, mojibake) ou nenhum.
//! Os limites são iniciais; calibrar com editais reais (ver a spec, Item 2).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum QualidadeTexto {
    Ok,
    SemTexto,
    /// Tem texto, mas ele parece ilegível.
    Suspeito,
}

impl QualidadeTexto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::SemTexto => "sem_texto",
            Self::Suspeito => "suspeito",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "ok" => Self::Ok,
            "sem_texto" => Self::SemTexto,
            "suspeito" => Self::Suspeito,
            _ => return None,
        })
    }

    /// Para decidir entre o texto nativo e o do OCR: maior é melhor.
    pub fn nota(self) -> u8 {
        match self {
            Self::SemTexto => 0,
            Self::Suspeito => 1,
            Self::Ok => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OrigemTexto {
    /// Camada de texto do PDF, via pdf.js.
    #[default]
    Pdf,
    /// tesseract.js.
    Ocr,
}

impl OrigemTexto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Ocr => "ocr",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pdf" => Some(Self::Pdf),
            "ocr" => Some(Self::Ocr),
            _ => None,
        }
    }
}

/// Abaixo disso a página é `sem_texto`.
const MIN_NAO_BRANCOS: usize = 20;
/// `U+FFFD`, uso privado e controles: acima desta fração, `suspeito`.
const MAX_CARACTERES_RUINS: f64 = 0.01;
/// Pares de mojibake (`Ã§`, `Ã£`, `Â `…): acima desta fração, `suspeito`.
const MAX_MOJIBAKE: f64 = 0.005;
/// Letras e dígitos sobre não-brancos: abaixo disso, `suspeito`.
const MIN_ALFANUMERICOS: f64 = 0.6;
/// Palavras comuns sobre palavras, em páginas com `MIN_PALAVRAS` ou mais.
const MIN_PALAVRAS_COMUNS: f64 = 0.15;
const MIN_PALAVRAS: usize = 50;
/// Confiança média do tesseract (0–100) abaixo da qual a página OCR fica `suspeito`.
pub const CONFIANCA_MINIMA_OCR: f32 = 70.0;
/// Fração de páginas ruins acima da qual processar exige confirmação.
pub const MAX_PAGINAS_RUINS: f64 = 0.2;

/// Palavras mais frequentes do português escrito, sem acento (a comparação ignora acentos).
const COMUNS: &[&str] = &[
    "a", "o", "as", "os", "um", "uma", "uns", "umas", "de", "do", "da", "dos", "das", "em", "no",
    "na", "nos", "nas", "por", "pelo", "pela", "pelos", "pelas", "para", "pra", "com", "sem", "sob",
    "sobre", "entre", "ate", "apos", "ante", "desde", "contra", "e", "ou", "nem", "mas", "que",
    "se", "como", "quando", "onde", "porque", "pois", "porem", "conforme", "caso", "ao", "aos",
    "ser", "sera", "serao", "sao", "foi", "for", "forem", "esta", "estao", "estar", "ter", "tem",
    "terao", "tera", "havera", "deve", "devera", "deverao", "pode", "podera", "poderao", "nao",
    "sim", "mais", "menos", "muito", "ja", "tambem", "so", "apenas", "este", "esta", "estes",
    "estas", "esse", "essa", "esses", "essas", "isso", "isto", "aquele", "aquela", "seu", "sua",
    "seus", "suas", "ele", "ela", "eles", "elas", "qual", "quais", "cada", "todo", "toda", "todos",
    "todas", "outro", "outra", "outros", "outras", "mesmo", "mesma", "dia", "dias", "ano", "anos",
    "item", "itens", "edital", "concurso", "candidato", "candidatos", "prova", "provas", "cargo",
    "cargos", "lei", "art", "artigo", "inciso", "pagina", "numero", "data", "horas", "hora",
    "publico", "publica", "nivel", "superior", "vaga", "vagas", "inscricao", "inscricoes",
    "conhecimentos", "basicos", "especificos", "lingua", "portuguesa", "nocoes", "direito",
    "regime", "anexo", "tabela", "periodo", "prazo", "forma", "termos", "caso", "parte", "geral",
    "seguinte", "seguintes", "acordo", "meio", "fase", "etapa", "resultado", "nota", "pontos",
    "objetiva", "discursiva", "classificacao", "aprovado", "aprovados", "remuneracao", "jornada",
    "semanais", "requisitos", "atribuicoes", "disposicoes", "finais", "banca", "organizadora",
    "fundacao", "tribunal", "estado", "municipio", "federal", "estadual", "municipal", "sistema",
    "sistemas", "gestao", "controle", "administracao", "ii", "iii", "iv", "v", "vi",
];

fn sem_acento(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        c => c,
    }
}

/// Segundo caractere de pares típicos de UTF-8 lido como Latin-1/Windows-1252.
fn e_continuacao_mojibake(c: char) -> bool {
    matches!(c, '\u{80}'..='\u{BF}') || "€‚ƒ„…†‡ˆ‰Š‹ŒŽ‘’“”•–—˜™š›œžŸ".contains(c)
}

pub fn classificar(texto: &str) -> QualidadeTexto {
    let nao_brancos: Vec<char> = texto.chars().filter(|c| !c.is_whitespace()).collect();
    let n = nao_brancos.len();
    if n < MIN_NAO_BRANCOS {
        return QualidadeTexto::SemTexto;
    }
    let fracao = |k: usize| k as f64 / n as f64;

    let ruins = nao_brancos
        .iter()
        .filter(|&&c| c == '\u{FFFD}' || ('\u{E000}'..='\u{F8FF}').contains(&c) || c.is_control())
        .count();
    if fracao(ruins) > MAX_CARACTERES_RUINS {
        return QualidadeTexto::Suspeito;
    }

    let mojibake = nao_brancos
        .windows(2)
        .filter(|w| matches!(w[0], 'Ã' | 'Â') && e_continuacao_mojibake(w[1]))
        .count();
    if fracao(mojibake) > MAX_MOJIBAKE {
        return QualidadeTexto::Suspeito;
    }

    let alfanumericos = nao_brancos.iter().filter(|c| c.is_alphanumeric()).count();
    if fracao(alfanumericos) < MIN_ALFANUMERICOS {
        return QualidadeTexto::Suspeito;
    }

    let palavras: Vec<String> = texto
        .split(|c: char| !c.is_alphabetic())
        .filter(|p| !p.is_empty())
        .map(|p| p.to_lowercase().chars().map(sem_acento).collect())
        .collect();
    if palavras.len() >= MIN_PALAVRAS {
        let comuns = palavras.iter().filter(|p| COMUNS.contains(&p.as_str())).count();
        if (comuns as f64 / palavras.len() as f64) < MIN_PALAVRAS_COMUNS {
            return QualidadeTexto::Suspeito;
        }
    }
    QualidadeTexto::Ok
}

/// Qualidade final da página: OCR com confiança baixa nunca passa de `suspeito`.
pub fn classificar_pagina(texto: &str, origem: OrigemTexto, confianca: Option<f32>) -> QualidadeTexto {
    let q = classificar(texto);
    if origem == OrigemTexto::Ocr
        && q == QualidadeTexto::Ok
        && confianca.unwrap_or(0.0) < CONFIANCA_MINIMA_OCR
    {
        return QualidadeTexto::Suspeito;
    }
    q
}

#[cfg(test)]
mod tests {
    use super::*;
    use QualidadeTexto::*;

    const EDITAL: &str = "CONCURSO PÚBLICO PARA PROVIMENTO DE CARGOS DO QUADRO DE PESSOAL. \
        O Presidente do Tribunal de Contas do Estado de São Paulo, no uso de suas atribuições, \
        torna pública a abertura de inscrições para o concurso público destinado ao provimento \
        de vagas do cargo de Auditor de Controle Externo, sob o regime estatutário, conforme as \
        disposições deste edital. 1. DAS DISPOSIÇÕES PRELIMINARES 1.1 O concurso será regido \
        por este edital e executado pela Fundação para o Vestibular da Universidade Estadual \
        Paulista, que será a banca organizadora. 1.2 A prova objetiva terá caráter eliminatório.";

    #[test]
    fn edital_normal_e_ok() {
        assert_eq!(classificar(EDITAL), Ok);
    }

    #[test]
    fn vazio_ou_curto_e_sem_texto() {
        assert_eq!(classificar(""), SemTexto);
        assert_eq!(classificar("  \n 12 \n"), SemTexto);
    }

    #[test]
    fn mojibake_e_suspeito() {
        let ruim = EDITAL.replace('ç', "Ã§").replace('ã', "Ã£").replace('ú', "Ãº");
        assert_eq!(classificar(&ruim), Suspeito);
    }

    #[test]
    fn substituicao_e_uso_privado_sao_suspeitos() {
        let ruim: String = EDITAL.chars().map(|c| if c == 'e' { '\u{FFFD}' } else { c }).collect();
        assert_eq!(classificar(&ruim), Suspeito);
        let pua: String = EDITAL.chars().map(|c| if c == 'a' { '\u{E01A}' } else { c }).collect();
        assert_eq!(classificar(&pua), Suspeito);
    }

    #[test]
    fn fonte_sem_mapa_vira_letras_trocadas_e_e_suspeita() {
        // Cifra de deslocamento: letras válidas, mas nenhuma palavra existe.
        let cifrado: String = EDITAL
            .chars()
            .map(|c| match c {
                'a'..='y' | 'A'..='Y' => char::from(c as u8 + 1),
                c => c,
            })
            .collect();
        assert_eq!(classificar(&cifrado), Suspeito);
    }

    #[test]
    fn simbolos_soltos_sao_suspeitos() {
        assert_eq!(classificar("!@#$ %&*() ;;:: ][}{ <<>> ~~^^ //\\ ++-- == ** !!"), Suspeito);
    }

    #[test]
    fn tabela_numerica_passa() {
        let quadro = "Cargo Vagas Salário\nAuditor 30 R$ 15.000,00\nAgente 12 R$ 8.000,00\n\
                      Técnico 45 R$ 6.500,00\n01/02/2026 15/03/2026 20/04/2026";
        assert_eq!(classificar(quadro), Ok);
    }

    #[test]
    fn ocr_com_confianca_baixa_fica_suspeito() {
        assert_eq!(classificar_pagina(EDITAL, OrigemTexto::Ocr, Some(55.0)), Suspeito);
        assert_eq!(classificar_pagina(EDITAL, OrigemTexto::Ocr, Some(91.0)), Ok);
        assert_eq!(classificar_pagina(EDITAL, OrigemTexto::Pdf, None), Ok);
    }
}
