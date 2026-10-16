//! Normalização do texto de página usada na busca: sem acentos, minúsculas,
//! espaços e quebras colapsados, hífen de fim de linha removido.
//!
//! Junto com o texto normalizado sai um mapa de volta ao original:
//! `mapa[k]` é o offset, em unidades UTF-16, do caractere do texto original que
//! gerou o k-ésimo `char` do normalizado. O mapa tem um elemento a mais no fim
//! (o tamanho UTF-16 do original), para que um intervalo `[a, b)` do normalizado
//! vire `[mapa[a], mapa[b])` no original.

use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Normalizado {
    pub texto: String,
    pub mapa: Vec<u32>,
}

impl Normalizado {
    /// Converte um intervalo em chars do normalizado para UTF-16 do original.
    #[allow(dead_code)] // usado pela verificação literal (Item 4)
    pub fn intervalo_original(&self, inicio: usize, fim: usize) -> (u32, u32) {
        (self.mapa[inicio], self.mapa[fim])
    }
}

const SOFT_HYPHEN: char = '\u{00AD}';

fn is_hifen(c: char) -> bool {
    matches!(c, '-' | '\u{2010}' | '\u{2011}')
}

pub fn normalizar(original: &str) -> Normalizado {
    let mut chars: Vec<(u32, char)> = Vec::with_capacity(original.len());
    let mut off = 0u32;
    for c in original.chars() {
        chars.push((off, c));
        off += c.len_utf16() as u32;
    }
    let total_utf16 = off;

    let mut texto = String::with_capacity(original.len());
    let mut mapa: Vec<u32> = Vec::with_capacity(original.len() + 1);
    let mut ultimo: Option<char> = None;

    let mut i = 0;
    while i < chars.len() {
        let (off, c) = chars[i];

        if c == SOFT_HYPHEN {
            i += 1;
            continue;
        }

        if is_hifen(c) && ultimo.is_some_and(char::is_alphabetic) {
            if let Some(prox) = fim_de_hifenizacao(&chars, i + 1) {
                i = prox;
                continue;
            }
        }

        if c.is_whitespace() {
            if ultimo.is_some_and(|u| u != ' ') {
                texto.push(' ');
                mapa.push(off);
                ultimo = Some(' ');
            }
            i += 1;
            continue;
        }

        for d in c.nfd().filter(|d| !is_combining_mark(*d)) {
            for l in d.to_lowercase() {
                texto.push(l);
                mapa.push(off);
                ultimo = Some(l);
            }
        }
        i += 1;
    }

    if ultimo == Some(' ') {
        texto.pop();
        mapa.pop();
    }
    mapa.push(total_utf16);

    Normalizado { texto, mapa }
}

/// Se a partir de `inicio` vier espaço opcional + quebra de linha + espaço
/// opcional + letra, devolve o índice da letra (o hífen e o espaço somem).
fn fim_de_hifenizacao(chars: &[(u32, char)], inicio: usize) -> Option<usize> {
    let mut j = inicio;
    let mut quebrou = false;
    while j < chars.len() && chars[j].1.is_whitespace() {
        if matches!(chars[j].1, '\n' | '\r') {
            quebrou = true;
        }
        j += 1;
    }
    (quebrou && j < chars.len() && chars[j].1.is_alphabetic()).then_some(j)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16_slice(s: &str, a: u32, b: u32) -> String {
        let u: Vec<u16> = s.encode_utf16().collect();
        String::from_utf16(&u[a as usize..b as usize]).unwrap()
    }

    #[test]
    fn remove_acentos_e_minusculas() {
        let n = normalizar("Língua PORTUGUESA: Crase");
        assert_eq!(n.texto, "lingua portuguesa: crase");
    }

    #[test]
    fn colapsa_espacos_e_quebras() {
        let n = normalizar("  Direito \n\n  Administrativo\t\t1. ");
        assert_eq!(n.texto, "direito administrativo 1.");
    }

    #[test]
    fn remove_hifen_de_fim_de_linha() {
        let n = normalizar("Adminis-\ntração Pública");
        assert_eq!(n.texto, "administracao publica");
        let n = normalizar("adminis- \r\n  tração");
        assert_eq!(n.texto, "administracao");
    }

    #[test]
    fn mantem_hifen_no_meio_da_linha_e_antes_de_numero() {
        assert_eq!(normalizar("guarda-chuva").texto, "guarda-chuva");
        assert_eq!(normalizar("Lei 8.112-\n90").texto, "lei 8.112- 90");
        assert_eq!(normalizar("itens 1 -\nDireito").texto, "itens 1 - direito");
    }

    #[test]
    fn remove_soft_hyphen() {
        assert_eq!(normalizar("con\u{00AD}trole").texto, "controle");
    }

    #[test]
    fn mapa_volta_ao_original() {
        let original = "1. Noções de\nAdminis-\ntração Pública 🚀 fim";
        let n = normalizar(original);
        assert_eq!(n.mapa.len(), n.texto.chars().count() + 1);

        let alvo = "administracao publica";
        let byte = n.texto.find(alvo).unwrap();
        let ini = n.texto[..byte].chars().count();
        let fim = ini + alvo.chars().count();
        let (a, b) = n.intervalo_original(ini, fim);
        assert_eq!(utf16_slice(original, a, b), "Adminis-\ntração Pública");

        // Depois de um caractere fora do BMP (2 unidades UTF-16).
        let byte = n.texto.find("fim").unwrap();
        let ini = n.texto[..byte].chars().count();
        let (a, b) = n.intervalo_original(ini, ini + 3);
        assert_eq!(utf16_slice(original, a, b), "fim");
    }

    #[test]
    fn texto_vazio() {
        let n = normalizar(" \n ");
        assert_eq!(n.texto, "");
        assert_eq!(n.mapa, vec![3]);
    }
}
