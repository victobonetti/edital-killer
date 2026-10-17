//! Papéis do LLM na análise do edital: preparar o workspace do job e, com a
//! resposta, validar contra o schema, verificar literalmente contra o texto e gravar.

pub mod verificacao;

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use crate::domain::normalize::normalizar;
use crate::domain::{Cargo, EtapaAnalise, Materia, Settings, Topico};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::llm::backend::Execucao;
use crate::storage::analise_repo::{self, PaginaBusca};
use crate::storage::edital_repo::{self, MetadadosLidos};
use crate::storage::Db;

use verificacao::localizar;

const PROMPT_LOCALIZADOR: &str = include_str!("../../prompts/localizador.prompt");
const PROMPT_EXTRATOR: &str = include_str!("../../prompts/extrator.prompt");
const SCHEMA_LOCALIZADOR: &str = include_str!("../../schemas/localizador.schema.json");
const SCHEMA_EXTRATOR: &str = include_str!("../../schemas/extrator.schema.json");

/// Modelo usado quando o usuário não escolheu outro em Ajustes.
pub const MODELO_PADRAO: &str = "sonnet";

fn schema(etapa: EtapaAnalise) -> &'static str {
    match etapa {
        EtapaAnalise::Localizar => SCHEMA_LOCALIZADOR,
        EtapaAnalise::Extrair => SCHEMA_EXTRATOR,
    }
}

/// Cria o workspace do job (`edital.txt`, `prompt.md`, `schema.json`) e a execução.
pub fn preparar(
    db: &Db,
    workspace: &Path,
    edital_id: &str,
    etapa: EtapaAnalise,
    settings: &Settings,
) -> AppResult<Execucao> {
    let (paginas, prompt) = {
        let conn = db.conn()?;
        let paginas = analise_repo::paginas_texto(&conn, edital_id)?;
        let prompt = match etapa {
            EtapaAnalise::Localizar => PROMPT_LOCALIZADOR.to_owned(),
            EtapaAnalise::Extrair => {
                let cargo = analise_repo::cargo_escolhido(&conn, edital_id)?
                    .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "nenhum cargo escolhido"))?;
                PROMPT_EXTRATOR.replace("{{CARGO}}", &cargo.nome)
            }
        };
        (paginas, prompt)
    };

    std::fs::create_dir_all(workspace)?;
    let mut texto = String::new();
    for p in &paginas {
        let _ = writeln!(texto, "=== PÁGINA {} ===", p.numero);
        if p.sem_texto {
            texto.push_str("[sem texto]\n");
        } else {
            texto.push_str(p.texto.trim_end());
            texto.push('\n');
        }
        texto.push('\n');
    }
    // Schema compacto: vai na linha de comando.
    let schema: Value = serde_json::from_str(schema(etapa))?;
    let schema = serde_json::to_string(&schema)?;
    std::fs::write(workspace.join("edital.txt"), texto)?;
    std::fs::write(workspace.join("prompt.md"), &prompt)?;
    std::fs::write(workspace.join("schema.json"), &schema)?;

    Ok(Execucao {
        workspace: workspace.to_path_buf(),
        prompt,
        schema,
        modelo: Some(settings.model.clone().unwrap_or_else(|| MODELO_PADRAO.to_owned())),
        timeout: Duration::from_secs(settings.timeout_secs.into()),
    })
}

/// Valida a saída do LLM, confere contra o texto e grava o resultado da etapa.
pub fn aplicar(db: &Db, edital_id: &str, etapa: EtapaAnalise, saida: Value) -> AppResult<()> {
    validar(etapa, &saida)?;
    let mut conn = db.conn()?;
    let paginas = analise_repo::paginas_busca(&conn, edital_id)?;
    match etapa {
        EtapaAnalise::Localizar => {
            let lido: Localizado = serde_json::from_value(saida).map_err(invalida)?;
            let cargos = cargos_verificados(&paginas, lido.cargos);
            if cargos.is_empty() {
                return Err(AppError::new(
                    ErrorCode::CargosNaoEncontrados,
                    "a resposta não trouxe nenhum cargo",
                ));
            }
            let metadados = metadados_verificados(&paginas, lido.metadados);
            if edital_repo::aplicar_metadados(&conn, edital_id, &metadados)? {
                tracing::info!(edital = edital_id, ?metadados, "metadados preenchidos pela análise");
            }
            analise_repo::substituir_cargos(&mut conn, edital_id, &cargos)
        }
        EtapaAnalise::Extrair => {
            let cargo = analise_repo::cargo_escolhido(&conn, edital_id)?
                .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "nenhum cargo escolhido"))?;
            let lido: Extraido = serde_json::from_value(saida).map_err(invalida)?;
            let materias = materias_verificadas(&paginas, lido.materias);
            if materias.is_empty() {
                return Err(AppError::new(
                    ErrorCode::ClaudeOutputInvalid,
                    "a resposta não trouxe nenhuma matéria",
                ));
            }
            analise_repo::substituir_conteudo(&mut conn, &cargo.id, &materias)
        }
    }
}

fn validar(etapa: EtapaAnalise, saida: &Value) -> AppResult<()> {
    let schema: Value = serde_json::from_str(schema(etapa))?;
    let validador = jsonschema::validator_for(&schema)
        .map_err(|e| AppError::new(ErrorCode::Internal, format!("schema embutido inválido: {e}")))?;
    let erros: Vec<String> = validador
        .iter_errors(saida)
        .take(3)
        .map(|e| format!("{} em {}", e, e.instance_path()))
        .collect();
    if erros.is_empty() {
        Ok(())
    } else {
        Err(AppError::new(ErrorCode::ClaudeOutputInvalid, erros.join("; ")))
    }
}

fn invalida(e: serde_json::Error) -> AppError {
    AppError::new(ErrorCode::ClaudeOutputInvalid, e.to_string())
}

fn novo_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ---- Saída do localizador ----

#[derive(Deserialize)]
struct Localizado {
    metadados: MetadadosSaida,
    cargos: Vec<CargoSaida>,
}

#[derive(Deserialize)]
struct MetadadosSaida {
    titulo: Option<Campo>,
    orgao: Option<Campo>,
    banca: Option<Campo>,
    numero_edital: Option<Campo>,
    data_prova: Option<Campo>,
    inscricoes_inicio: Option<Campo>,
    inscricoes_fim: Option<Campo>,
}

#[derive(Deserialize)]
struct Campo {
    valor: String,
    trecho: String,
    pagina: u32,
}

#[derive(Deserialize)]
struct CargoSaida {
    nome: String,
    vagas: Option<u32>,
    escolaridade: Option<String>,
    pagina: u32,
}

/// Só sobrevive o campo cujo trecho existe no edital (e, nas datas, com data válida).
fn metadados_verificados(paginas: &[PaginaBusca], m: MetadadosSaida) -> MetadadosLidos {
    let texto = |c: Option<Campo>| {
        c.filter(|c| localizar(paginas, Some(c.pagina), &c.trecho).is_some())
            .map(|c| c.valor.trim().to_owned())
            .filter(|v| !v.is_empty())
    };
    let data = |c: Option<Campo>| texto(c).filter(|d| edital_repo::data_valida(d));
    MetadadosLidos {
        nome: texto(m.titulo),
        orgao: texto(m.orgao),
        banca: texto(m.banca),
        numero_edital: texto(m.numero_edital),
        data_prova: data(m.data_prova),
        inscricoes_inicio: data(m.inscricoes_inicio),
        inscricoes_fim: data(m.inscricoes_fim),
    }
}

fn cargos_verificados(paginas: &[PaginaBusca], cargos: Vec<CargoSaida>) -> Vec<Cargo> {
    let mut vistos = HashSet::new();
    cargos
        .into_iter()
        .filter(|c| vistos.insert(normalizar(c.nome.trim()).texto))
        .map(|c| {
            let achado = localizar(paginas, Some(c.pagina), &c.nome);
            Cargo {
                id: novo_id(),
                nome: c.nome.trim().to_owned(),
                vagas: c.vagas,
                escolaridade: c.escolaridade.map(|e| e.trim().to_owned()).filter(|e| !e.is_empty()),
                pagina: Some(achado.map_or(c.pagina, |a| a.pagina)),
                verificado: achado.is_some(),
            }
        })
        .collect()
}

// ---- Saída do extrator ----

#[derive(Deserialize)]
struct Extraido {
    materias: Vec<MateriaSaida>,
}

#[derive(Deserialize)]
struct MateriaSaida {
    nome: String,
    pagina: u32,
    topicos: Vec<TopicoSaida>,
}

#[derive(Deserialize)]
struct TopicoSaida {
    texto: String,
    pagina: u32,
    #[serde(default)]
    subtopicos: Vec<TopicoSaida>,
}

fn materias_verificadas(paginas: &[PaginaBusca], materias: Vec<MateriaSaida>) -> Vec<Materia> {
    materias
        .into_iter()
        .filter(|m| !m.nome.trim().is_empty())
        .map(|m| Materia {
            id: novo_id(),
            nome: m.nome.trim().to_owned(),
            pagina: Some(localizar(paginas, Some(m.pagina), &m.nome).map_or(m.pagina, |a| a.pagina)),
            topicos: m.topicos.into_iter().map(|t| topico(paginas, t)).collect(),
        })
        .collect()
}

fn topico(paginas: &[PaginaBusca], t: TopicoSaida) -> Topico {
    let achado = localizar(paginas, Some(t.pagina), &t.texto);
    Topico {
        id: novo_id(),
        texto: t.texto.trim().to_owned(),
        pagina: Some(achado.map_or(t.pagina, |a| a.pagina)),
        verificado: achado.is_some(),
        inicio: achado.map(|a| a.inicio),
        fim: achado.map(|a| a.fim),
        // Subtópicos mais profundos já chegam achatados pelo schema (dois níveis).
        filhos: t.subtopicos.into_iter().map(|s| topico(paginas, s)).collect(),
    }
}

#[cfg(test)]
pub(crate) mod tests_apoio {
    use crate::domain::PaginaInput;
    use crate::storage::edital_repo::{finalizar_importacao, inserir, salvar_paginas, NovoEdital};
    use crate::storage::Db;

    /// Edital "e1" com 3 páginas: metadados, quadro de cargos e um anexo de conteúdo.
    pub fn edital_de_teste(db: &Db) {
        let mut conn = db.conn().unwrap();
        inserir(
            &conn,
            &NovoEdital {
                id: "e1",
                nome: "edital_abertura",
                arquivo_original: "edital_abertura.pdf",
                sha256: "h",
                importado_em: "x",
            },
        )
        .unwrap();
        let p = |n: u32, t: &str| PaginaInput {
            pagina: n,
            texto: t.into(),
            itens: vec![],
        };
        salvar_paginas(
            &mut conn,
            "e1",
            &[
                p(1, "EDITAL Nº 01/2026 — CONCURSO PÚBLICO\nTRIBUNAL DE CONTAS DO ESTADO\nA prova será realizada em 14 de março de 2027."),
                p(2, "QUADRO DE CARGOS\nAuditor de Controle Externo – TI   12 vagas"),
                p(3, "ANEXO I\nLÍNGUA PORTUGUESA: 1. Crase. 2. Regência verbal e nominal.\n2.1 Regência de verbos de movimento."),
            ],
        )
        .unwrap();
        finalizar_importacao(&conn, "e1", 3).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::tests_apoio::edital_de_teste;
    use super::*;
    use serde_json::json;


    fn localizado() -> Value {
        json!({
            "metadados": {
                "titulo": {"valor": "Concurso Público nº 01/2026", "trecho": "EDITAL Nº 01/2026 — CONCURSO PÚBLICO", "pagina": 1},
                "orgao": {"valor": "Tribunal de Contas do Estado", "trecho": "TRIBUNAL DE CONTAS DO ESTADO", "pagina": 1},
                "banca": {"valor": "VUNESP", "trecho": "Fundação VUNESP organiza", "pagina": 1},
                "numero_edital": null,
                "data_prova": {"valor": "2027-03-14", "trecho": "realizada em 14 de março de 2027", "pagina": 1},
                "inscricoes_inicio": null,
                "inscricoes_fim": null
            },
            "cargos": [
                {"nome": "Auditor de Controle Externo – TI", "vagas": 12, "escolaridade": "Superior", "pagina": 1},
                {"nome": "Auditor de Controle Externo – TI", "vagas": 12, "escolaridade": "Superior", "pagina": 2},
                {"nome": "Cargo inventado", "vagas": null, "escolaridade": null, "pagina": 2}
            ]
        })
    }

    #[test]
    fn localizador_grava_cargos_e_metadados_verificados() {
        let db = Db::open_in_memory().unwrap();
        edital_de_teste(&db);
        aplicar(&db, "e1", EtapaAnalise::Localizar, localizado()).unwrap();
        let conn = db.conn().unwrap();
        let e = edital_repo::get(&conn, "e1").unwrap();
        assert_eq!(e.nome, "Concurso Público nº 01/2026");
        assert_eq!(e.orgao.as_deref(), Some("Tribunal de Contas do Estado"));
        // Trecho da banca não existe no texto: descartado.
        assert_eq!(e.banca, None);
        assert_eq!(e.data_prova.as_deref(), Some("2027-03-14"));

        let cargos = analise_repo::cargos(&conn, "e1").unwrap();
        assert_eq!(cargos.len(), 2, "duplicata removida");
        assert!(cargos[0].verificado);
        assert_eq!(cargos[0].pagina, Some(2), "página corrigida");
        assert!(!cargos[1].verificado);
    }

    #[test]
    fn extrator_grava_conteudo_com_verificacao() {
        let db = Db::open_in_memory().unwrap();
        edital_de_teste(&db);
        aplicar(&db, "e1", EtapaAnalise::Localizar, localizado()).unwrap();
        let cargo = analise_repo::cargos(&db.conn().unwrap(), "e1").unwrap()[0].clone();
        analise_repo::escolher_cargo(&db.conn().unwrap(), "e1", &cargo.id).unwrap();

        let saida = json!({"materias": [{
            "nome": "Língua Portuguesa", "pagina": 3,
            "topicos": [
                {"texto": "Crase.", "pagina": 3, "subtopicos": []},
                {"texto": "Regência verbal e nominal.", "pagina": 3, "subtopicos": [
                    {"texto": "Regência de verbos de movimento.", "pagina": 3}
                ]},
                {"texto": "Física quântica", "pagina": 3, "subtopicos": []}
            ]
        }]});
        aplicar(&db, "e1", EtapaAnalise::Extrair, saida).unwrap();

        let a = analise_repo::carregar(&db.conn().unwrap(), "e1").unwrap();
        let m = &a.materias.unwrap()[0];
        assert_eq!(m.topicos.len(), 3);
        assert!(m.topicos[0].verificado && m.topicos[0].inicio.is_some());
        assert!(m.topicos[1].filhos[0].verificado);
        assert!(!m.topicos[2].verificado);
    }

    #[test]
    fn saida_fora_do_schema_e_rejeitada() {
        let db = Db::open_in_memory().unwrap();
        edital_de_teste(&db);
        let err = aplicar(&db, "e1", EtapaAnalise::Localizar, json!({"cargos": "não"})).unwrap_err();
        assert_eq!(err.code, ErrorCode::ClaudeOutputInvalid);
    }

    #[test]
    fn sem_cargos_vira_erro_explicado() {
        let db = Db::open_in_memory().unwrap();
        edital_de_teste(&db);
        let mut v = localizado();
        v["cargos"] = json!([]);
        let err = aplicar(&db, "e1", EtapaAnalise::Localizar, v).unwrap_err();
        assert_eq!(err.code, ErrorCode::CargosNaoEncontrados);
    }

    #[test]
    fn workspace_tem_texto_por_pagina_e_modelo_padrao() {
        let db = Db::open_in_memory().unwrap();
        edital_de_teste(&db);
        let dir = tempfile::tempdir().unwrap();
        let ex = preparar(&db, dir.path(), "e1", EtapaAnalise::Localizar, &Settings::default()).unwrap();
        let texto = std::fs::read_to_string(dir.path().join("edital.txt")).unwrap();
        assert!(texto.starts_with("=== PÁGINA 1 ===\nEDITAL"));
        assert!(texto.contains("=== PÁGINA 3 ==="));
        assert_eq!(ex.modelo.as_deref(), Some(MODELO_PADRAO));
        assert!(serde_json::from_str::<Value>(&ex.schema).is_ok());
        // Extrator exige cargo escolhido.
        let err = preparar(&db, dir.path(), "e1", EtapaAnalise::Extrair, &Settings::default()).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidInput);
    }
}
