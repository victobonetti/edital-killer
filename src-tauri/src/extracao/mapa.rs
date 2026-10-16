//! Etapa 1: mapa do edital, sem IA. Acha os títulos de seção ("3. DOS CARGOS",
//! "ANEXO I – CONTEÚDO PROGRAMÁTICO") no texto das páginas e corta o edital em
//! seções com faixa de páginas. Cabeçalhos e rodapés repetidos são ignorados.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

/// Título numerado em caixa alta, numa linha própria: "10. DA REALIZAÇÃO DAS PROVAS".
static NUMERADO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(\d{1,2})\.[ \t]+((?:DA|DO|DAS|DOS)[ \t]+[A-ZÀ-ÖØ-Þ][A-ZÀ-ÖØ-Þ0-9 \t,;/()ºª.–-]{2,})$")
        .expect("regex de título numerado")
});
/// "ANEXO I – CONTEÚDO PROGRAMÁTICO" (o título depois do numeral é opcional).
static ANEXO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(ANEXO[ \t]+([IVXLC]+))\b[ \t]*[–:-]?[ \t]*([^\n]{0,120})$").expect("regex de anexo")
});

/// Menos títulos que isso: o mapa cai para blocos de páginas.
const MIN_TITULOS: usize = 3;
const PAGINAS_POR_BLOCO: u32 = 5;
/// Texto antes do primeiro título vira "Preâmbulo" se tiver ao menos isso.
const MIN_PREAMBULO: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secao {
    /// 0-based, na ordem do edital.
    pub ordem: u32,
    pub titulo: String,
    pub pagina_ini: u32,
    /// Byte no texto original da página inicial onde a seção começa.
    pub byte_ini: u32,
    pub pagina_fim: u32,
    /// Byte (exclusivo) no texto original da página final onde a seção termina.
    pub byte_fim: u32,
}

pub struct Pagina<'a> {
    pub numero: u32,
    pub texto: &'a str,
}

/// Linhas que se repetem em mais da metade das páginas (cabeçalho/rodapé).
pub fn linhas_repetidas(paginas: &[Pagina]) -> HashSet<String> {
    if paginas.len() < 4 {
        return HashSet::new();
    }
    let mut contagem: HashMap<String, usize> = HashMap::new();
    for p in paginas {
        let unicas: HashSet<&str> = p.texto.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        for l in unicas {
            *contagem.entry(l.to_owned()).or_default() += 1;
        }
    }
    contagem
        .into_iter()
        .filter(|(_, n)| *n * 2 > paginas.len())
        .map(|(l, _)| l)
        .collect()
}

/// Texto sem cabeçalhos/rodapés repetidos e sem a linha com o número da própria página.
/// Outras linhas só com número ficam: numa tabela, "30" pode ser o total de vagas.
pub fn limpar(texto: &str, repetidas: &HashSet<String>, numero_pagina: u32) -> String {
    let numero = numero_pagina.to_string();
    texto
        .lines()
        .filter(|l| {
            let t = l.trim();
            !repetidas.contains(t) && t != numero
        })
        .collect::<Vec<_>>()
        .join("\n")
}

struct Titulo {
    pagina_idx: usize,
    byte: usize,
    texto: String,
}

fn achar_titulos(paginas: &[Pagina]) -> Vec<Titulo> {
    let mut titulos = Vec::new();
    let mut proximo_numero = 1u32;
    let mut em_anexos = false;
    let mut anexos_vistos = HashSet::new();
    for (idx, p) in paginas.iter().enumerate() {
        let mut achados: Vec<(usize, String, bool)> = Vec::new();
        for c in NUMERADO.captures_iter(p.texto) {
            let n: u32 = c[1].parse().unwrap_or(0);
            achados.push((c.get(0).unwrap().start(), format!("{n}. {}", c[2].trim()), false));
        }
        for c in ANEXO.captures_iter(p.texto) {
            let resto = c[3].trim();
            let titulo = if resto.is_empty() { c[1].to_owned() } else { format!("{} – {resto}", &c[1]) };
            achados.push((c.get(0).unwrap().start(), titulo, true));
        }
        achados.sort_by_key(|a| a.0);
        for (byte, texto, anexo) in achados {
            if anexo {
                let numeral = texto.split_whitespace().nth(1).unwrap_or_default().to_owned();
                if anexos_vistos.insert(numeral) {
                    em_anexos = true;
                    titulos.push(Titulo { pagina_idx: idx, byte, texto });
                }
                continue;
            }
            // Só a sequência 1, 2, 3…, e só antes dos anexos: listas numeradas dentro
            // de anexos ou citações de itens não viram seção.
            let n: u32 = texto.split('.').next().and_then(|n| n.parse().ok()).unwrap_or(0);
            if !em_anexos && n == proximo_numero {
                proximo_numero += 1;
                titulos.push(Titulo { pagina_idx: idx, byte, texto });
            }
        }
    }
    titulos
}

/// Se o que a seção pega da última página é só cabeçalho (ou nada), o fim recua para
/// o fim da página anterior: "páginas 1–2" vira "página 1".
fn recuar_fim(paginas: &[Pagina], s: &mut Secao, repetidas: &HashSet<String>) {
    while s.pagina_fim > s.pagina_ini {
        let Some(idx) = paginas.iter().position(|p| p.numero == s.pagina_fim) else { return };
        let p = &paginas[idx];
        let sobra = p.texto.get(..s.byte_fim as usize).unwrap_or_default();
        if !limpar(sobra, repetidas, p.numero).trim().is_empty() || idx == 0 {
            return;
        }
        let anterior = &paginas[idx - 1];
        s.pagina_fim = anterior.numero;
        s.byte_fim = anterior.texto.len() as u32;
    }
}

pub fn mapear(paginas: &[Pagina], repetidas: &HashSet<String>) -> Vec<Secao> {
    let Some(ultima) = paginas.last() else {
        return Vec::new();
    };
    let fim_total = (ultima.numero, ultima.texto.len() as u32);
    let titulos = achar_titulos(paginas);

    if titulos.len() < MIN_TITULOS {
        return paginas
            .chunks(PAGINAS_POR_BLOCO as usize)
            .enumerate()
            .map(|(k, bloco)| {
                let (a, b) = (bloco[0].numero, bloco[bloco.len() - 1].numero);
                Secao {
                    ordem: k as u32,
                    titulo: if a == b { format!("Página {a}") } else { format!("Páginas {a}–{b}") },
                    pagina_ini: a,
                    byte_ini: 0,
                    pagina_fim: b,
                    byte_fim: bloco[bloco.len() - 1].texto.len() as u32,
                }
            })
            .collect();
    }

    let mut secoes = Vec::new();
    let primeiro = &titulos[0];
    let preambulo: usize = paginas[..primeiro.pagina_idx].iter().map(|p| p.texto.len()).sum::<usize>() + primeiro.byte;
    if preambulo >= MIN_PREAMBULO {
        secoes.push(Secao {
            ordem: 0,
            titulo: "Preâmbulo".into(),
            pagina_ini: paginas[0].numero,
            byte_ini: 0,
            pagina_fim: paginas[primeiro.pagina_idx].numero,
            byte_fim: primeiro.byte as u32,
        });
    }
    for (k, t) in titulos.iter().enumerate() {
        let (pagina_fim, byte_fim) = match titulos.get(k + 1) {
            Some(prox) => (paginas[prox.pagina_idx].numero, prox.byte as u32),
            None => fim_total,
        };
        secoes.push(Secao {
            ordem: secoes.len() as u32,
            titulo: t.texto.clone(),
            pagina_ini: paginas[t.pagina_idx].numero,
            byte_ini: t.byte as u32,
            pagina_fim,
            byte_fim,
        });
    }
    for s in &mut secoes {
        recuar_fim(paginas, s, repetidas);
    }
    secoes
}

/// Texto de uma seção, com marcadores `[[página N]]` e sem cabeçalhos repetidos.
pub fn texto_secao(paginas: &[Pagina], s: &Secao, repetidas: &HashSet<String>) -> String {
    let mut out = String::new();
    for p in paginas.iter().filter(|p| (s.pagina_ini..=s.pagina_fim).contains(&p.numero)) {
        let ini = if p.numero == s.pagina_ini { s.byte_ini as usize } else { 0 };
        let fim = if p.numero == s.pagina_fim { s.byte_fim as usize } else { p.texto.len() };
        let trecho = p.texto.get(ini..fim.max(ini)).unwrap_or_default();
        let limpo = limpar(trecho, repetidas, p.numero);
        if limpo.trim().is_empty() {
            continue;
        }
        out.push_str(&format!("[[página {}]]\n{}\n", p.numero, limpo.trim()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAB: &str = "TRIBUNAL DE CONTAS DO ESTADO DE SÃO PAULO";

    fn pag(numero: u32, corpo: &str) -> (u32, String) {
        (numero, format!("{numero}\n{CAB}\n{corpo}"))
    }

    fn edital() -> Vec<(u32, String)> {
        vec![
            pag(1, "CONCURSO PÚBLICO PARA PROVIMENTO DE VAGAS NO CARGO DE AUDITOR DE CONTROLE EXTERNO\nEDITAL Nº 001/2026, DE 3 DE JULHO DE 2026\nO Presidente do Tribunal de Contas torna pública a realização de concurso, regido pelas instruções a seguir.\n1. DAS DISPOSIÇÕES PRELIMINARES\n1.1. O concurso será executado pela Fundação VUNESP."),
            pag(2, "1.2. Itens.\n2. DO CONCURSO\n2.1. O Concurso será realizado em 2 (duas) etapas.\n3. DOS CARGOS\n3.1. Auditor de Controle Externo, 30 vagas."),
            pag(3, "Conforme o item 3. DOS CARGOS acima.\n4. DAS INSCRIÇÕES\n4.1. As inscrições serão feitas pela internet."),
            pag(4, "ANEXO I – CONTEÚDO PROGRAMÁTICO\nLíngua Portuguesa: Interpretação de texto.\n1. DOS PRINCÍPIOS FUNDAMENTAIS\nDireito Constitucional."),
            pag(5, "ANEXO II – CRONOGRAMA PREVISTO\nInício das inscrições 13/07/2026"),
        ]
    }

    fn paginas(v: &[(u32, String)]) -> Vec<Pagina<'_>> {
        v.iter().map(|(n, t)| Pagina { numero: *n, texto: t }).collect()
    }

    #[test]
    fn acha_secoes_em_sequencia_e_anexos() {
        let v = edital();
        let p = paginas(&v);
        let s = mapear(&p, &linhas_repetidas(&p));
        let titulos: Vec<&str> = s.iter().map(|s| s.titulo.as_str()).collect();
        assert_eq!(
            titulos,
            [
                "Preâmbulo",
                "1. DAS DISPOSIÇÕES PRELIMINARES",
                "2. DO CONCURSO",
                "3. DOS CARGOS",
                "4. DAS INSCRIÇÕES",
                "ANEXO I – CONTEÚDO PROGRAMÁTICO",
                "ANEXO II – CRONOGRAMA PREVISTO",
            ]
        );
        // "3. DOS CARGOS" citado no meio da linha e "1. DOS PRINCÍPIOS" dentro do anexo não viram seção.
        let cargos = &s[3];
        assert_eq!((cargos.pagina_ini, cargos.pagina_fim), (2, 3));
        // "2. DO CONCURSO" termina onde começa "3. DOS CARGOS", na mesma página.
        assert_eq!((s[2].pagina_ini, s[2].pagina_fim), (2, 2));
        // O preâmbulo e "1. DAS DISPOSIÇÕES…" não invadem a página 2 só por causa do cabeçalho.
        assert_eq!(s[1].pagina_fim, 2, "1.2. Itens. está na página 2");
        assert_eq!(s[4].pagina_fim, 3, "4. DAS INSCRIÇÕES não pega o cabeçalho da página 4");
        assert_eq!(s.last().unwrap().pagina_fim, 5);
    }

    #[test]
    fn texto_da_secao_tem_marcadores_e_sem_cabecalho() {
        let v = edital();
        let p = paginas(&v);
        let repetidas = linhas_repetidas(&p);
        assert!(repetidas.contains(CAB));
        let s = mapear(&p, &repetidas);
        let t = texto_secao(&p, &s[3], &repetidas);
        assert!(t.starts_with("[[página 2]]\n3. DOS CARGOS"), "{t}");
        assert!(t.contains("[[página 3]]\nConforme o item"));
        assert!(!t.contains("4. DAS INSCRIÇÕES"));
        assert!(!t.contains(CAB));
    }

    #[test]
    fn sem_titulos_cai_para_blocos_de_paginas() {
        let v: Vec<(u32, String)> = (1..=12).map(|n| (n, format!("texto corrido da página {n}"))).collect();
        let s = mapear(&paginas(&v), &HashSet::new());
        let titulos: Vec<&str> = s.iter().map(|s| s.titulo.as_str()).collect();
        assert_eq!(titulos, ["Páginas 1–5", "Páginas 6–10", "Páginas 11–12"]);
    }
}
