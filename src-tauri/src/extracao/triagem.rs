//! Etapa 2: triagem. Recebe só o sumário do mapa (título + começo de cada seção) e
//! devolve se o PDF é um edital e as categorias de cada seção.

use std::collections::HashSet;

use rusqlite::Connection;
use serde::Deserialize;

use super::mapa::{self, Pagina, Secao};
use super::{PROMPT_SISTEMA};
use crate::domain::{Categoria, PerfilModelo};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::llm::jobs::NovoJob;
use crate::storage::extracao_repo;

/// Suba quando mudar o prompt ou o schema: extrações antigas ficam marcadas como de versão anterior.
pub const VERSAO: u32 = 1;
const PROMPT: &str = include_str!("../../prompts/triagem.md");
const SCHEMA: &str = include_str!("../../schemas/triagem.schema.json");
/// Caracteres do começo de cada seção que vão no sumário.
const AMOSTRA: usize = 400;

/// Sumário enviado à triagem: `[S<ordem>] <título> (páginas a–b, N caracteres)` + amostra.
pub fn sumario(paginas: &[Pagina], secoes: &[Secao], repetidas: &HashSet<String>) -> String {
    let mut out = String::new();
    for s in secoes {
        let texto = mapa::texto_secao(paginas, s, repetidas);
        let corrido = texto
            .lines()
            .filter(|l| !l.starts_with("[[página "))
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let sem_titulo = corrido.strip_prefix(s.titulo.as_str()).unwrap_or(&corrido).trim();
        let amostra: String = sem_titulo.chars().take(AMOSTRA).collect();
        let reticencias = if sem_titulo.chars().count() > AMOSTRA { "…" } else { "" };
        let paginas_txt = if s.pagina_ini == s.pagina_fim {
            format!("página {}", s.pagina_ini)
        } else {
            format!("páginas {}–{}", s.pagina_ini, s.pagina_fim)
        };
        out.push_str(&format!(
            "[S{}] {} ({paginas_txt}, {} caracteres)\n{amostra}{reticencias}\n\n",
            s.ordem,
            s.titulo,
            corrido.chars().count()
        ));
    }
    out
}

pub fn novo_job(edital_id: &str, arquivo: &str, sumario: &str) -> AppResult<NovoJob> {
    Ok(NovoJob {
        edital_id: Some(edital_id.to_owned()),
        tipo: "triagem".into(),
        system: PROMPT_SISTEMA.into(),
        prompt: PROMPT.replace("{{ARQUIVO}}", arquivo).replace("{{SUMARIO}}", sumario),
        schema: serde_json::from_str(SCHEMA)?,
        perfil: PerfilModelo::Rapido,
    })
}

#[derive(Debug, Deserialize)]
struct Resposta {
    e_edital: bool,
    motivo: String,
    secoes: Vec<RespostaSecao>,
}

#[derive(Debug, Deserialize)]
struct RespostaSecao {
    id: String,
    categorias: Vec<Categoria>,
}

/// Grava o resultado da triagem. Seção que a triagem pulou vira `outros` (e vai para o log).
/// Devolve `Err(EDITAL_NAO_E_EDITAL)` quando não é edital.
pub fn aplicar(conn: &Connection, edital_id: &str, resultado: &str) -> AppResult<()> {
    let r: Resposta = serde_json::from_str(resultado)
        .map_err(|e| AppError::new(ErrorCode::ClaudeSaidaInvalida, format!("triagem: {e}")))?;
    let secoes = extracao_repo::secoes(conn, edital_id)?;
    for s in &secoes {
        let id = format!("S{}", s.ordem);
        let categorias = match r.secoes.iter().find(|x| x.id == id) {
            Some(x) => x.categorias.clone(),
            None => {
                tracing::warn!(edital = %edital_id, secao = %id, "triagem pulou a seção; fica como outros");
                vec![Categoria::Outros]
            }
        };
        extracao_repo::salvar_categorias(conn, edital_id, s.ordem, &categorias)?;
    }
    extracao_repo::salvar_triagem(conn, edital_id, r.e_edital, &r.motivo)?;
    if !r.e_edital {
        return Err(AppError::new(ErrorCode::EditalNaoEEdital, r.motivo));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sumario_traz_id_titulo_paginas_e_amostra_sem_titulo() {
        let textos = [
            (1, "1. DAS DISPOSIÇÕES PRELIMINARES\n1.1 O concurso será executado pela Fundação VUNESP.".to_owned()),
            (2, "2. DO CONCURSO\n2.1 Prova objetiva e prova escrita.".to_owned()),
            (3, "3. DOS CARGOS\n3.1 Auditor de Controle Externo: 30 vagas.".to_owned()),
        ];
        let paginas: Vec<Pagina> = textos.iter().map(|(n, t)| Pagina { numero: *n, texto: t }).collect();
        let secoes = mapa::mapear(&paginas, &HashSet::new());
        let s = sumario(&paginas, &secoes, &HashSet::new());
        assert!(s.contains("[S0] 1. DAS DISPOSIÇÕES PRELIMINARES (página 1, "), "{s}");
        assert!(s.contains("\n1.1 O concurso será executado pela Fundação VUNESP.\n"), "{s}");
        assert!(s.contains("[S1] 2. DO CONCURSO (página 2, "), "{s}");
        assert!(s.contains("[S2] 3. DOS CARGOS (página 3, "));
        let job = novo_job("e1", "edital.pdf", &s).unwrap();
        assert!(job.prompt.contains("arquivo: edital.pdf"));
        assert!(job.prompt.contains("[S1] 2. DO CONCURSO"));
        assert_eq!(job.perfil, PerfilModelo::Rapido);
    }
}
