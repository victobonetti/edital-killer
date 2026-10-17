//! Verificação literal: confirma que um trecho devolvido pelo LLM existe no texto
//! do edital. A busca é no texto normalizado (sem acento, minúsculo, espaços
//! colapsados, hifenização desfeita), e o achado volta como offsets do original.

use crate::domain::normalize::normalizar;
use crate::storage::analise_repo::PaginaBusca;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Achado {
    pub pagina: u32,
    /// Offsets UTF-16 no texto original da página.
    pub inicio: u32,
    pub fim: u32,
}

/// Procura `trecho` primeiro na página indicada, depois nas vizinhas e por fim no
/// edital inteiro. Prefixos de numeração ("1.2", "a)") que o LLM às vezes omite
/// ou acrescenta não atrapalham: o trecho é procurado sem eles.
pub fn localizar(paginas: &[PaginaBusca], pagina: Option<u32>, trecho: &str) -> Option<Achado> {
    let alvo = normalizar(sem_numeracao(trecho)).texto;
    let alvo = alvo.trim_matches(|c: char| c.is_whitespace() || ".;:,".contains(c));
    // Muito curto casa em qualquer lugar e não prova nada.
    if alvo.chars().count() < 3 {
        return None;
    }
    let ordem = pagina.into_iter().flat_map(|p| [p, p.saturating_sub(1), p + 1]);
    let preferidas = ordem.filter_map(|n| paginas.iter().find(|p| p.numero == n));
    preferidas
        .chain(paginas.iter())
        .find_map(|p| buscar(p, alvo))
}

fn buscar(p: &PaginaBusca, alvo: &str) -> Option<Achado> {
    let byte = p.normalizado.find(alvo)?;
    let ini = p.normalizado[..byte].chars().count();
    let fim = ini + alvo.chars().count();
    Some(Achado {
        pagina: p.numero,
        inicio: *p.mapa.get(ini)?,
        fim: *p.mapa.get(fim)?,
    })
}

/// Remove numeração de item no início: "1.", "1.2.3", "a)", "IV -", "•".
fn sem_numeracao(s: &str) -> &str {
    let s = s.trim_start();
    let token_fim = s.find(char::is_whitespace).unwrap_or(s.len());
    let token = &s[..token_fim];
    let numerico = !token.is_empty()
        && token.chars().all(|c| c.is_ascii_digit() || ".)-–".contains(c))
        && token.chars().any(|c| c.is_ascii_digit());
    let letra = token.len() <= 3
        && (token.ends_with(')') || token.ends_with('.'))
        && token[..token.len() - 1].chars().all(|c| c.is_ascii_alphabetic());
    let marcador = matches!(token, "•" | "-" | "–" | "—" | "*");
    if (numerico || letra || marcador) && token_fim < s.len() {
        s[token_fim..].trim_start().trim_start_matches(['-', '–', '—']).trim_start()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pagina(numero: u32, texto: &str) -> PaginaBusca {
        let n = normalizar(texto);
        PaginaBusca {
            numero,
            normalizado: n.texto,
            mapa: n.mapa,
        }
    }

    fn edital() -> Vec<PaginaBusca> {
        vec![
            pagina(1, "EDITAL DE ABERTURA Nº 01/2026\nTribunal de Contas"),
            pagina(2, "LÍNGUA PORTUGUESA: 1. Leitura e interpre-\ntação de textos. 2. Crase."),
            pagina(3, "TECNOLOGIA DA INFORMAÇÃO: Gerenciamento de\n   serviços com ITIL 4."),
        ]
    }

    #[test]
    fn acha_com_acento_caixa_e_hifenizacao() {
        let ps = edital();
        let a = localizar(&ps, Some(2), "Leitura e interpretação de textos").unwrap();
        assert_eq!(a.pagina, 2);
        let original = "LÍNGUA PORTUGUESA: 1. Leitura e interpre-\ntação de textos. 2. Crase.";
        let utf16: Vec<u16> = original.encode_utf16().collect();
        let achado = String::from_utf16(&utf16[a.inicio as usize..a.fim as usize]).unwrap();
        assert_eq!(achado, "Leitura e interpre-\ntação de textos");
    }

    #[test]
    fn ignora_numeracao_e_quebras() {
        let ps = edital();
        assert!(localizar(&ps, Some(3), "1.1 Gerenciamento de serviços com ITIL 4").is_some());
        assert!(localizar(&ps, Some(2), "b) Crase.").is_some());
    }

    #[test]
    fn corrige_pagina_errada() {
        let ps = edital();
        assert_eq!(localizar(&ps, Some(1), "Crase").unwrap().pagina, 2);
        assert_eq!(localizar(&ps, None, "tribunal de contas").unwrap().pagina, 1);
    }

    #[test]
    fn trecho_inventado_ou_curto_nao_verifica() {
        let ps = edital();
        assert_eq!(localizar(&ps, Some(2), "Física quântica aplicada"), None);
        assert_eq!(localizar(&ps, Some(2), "a"), None);
    }
}
