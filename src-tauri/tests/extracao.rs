//! Extração de ponta a ponta (mapa, fila, triagem) contra o `fake-claude`, que responde
//! com as fixtures de `tests/respostas/`.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use edital_killer_lib::domain::{
    Bloco, Categoria, EstadoExtracao, ItemTexto, OrigemTexto, PaginaInput, Settings, StatusExtracao,
};
use edital_killer_lib::error::ErrorCode;
use edital_killer_lib::extracao;
use edital_killer_lib::llm::jobs::{Ambiente, Esperas, Fila, Notificacao};
use edital_killer_lib::storage::edital_repo::{self, NovoEdital};
use edital_killer_lib::storage::{config_repo, extracao_repo, AppPaths, Db};

struct Teste {
    db: Db,
    paths: AppPaths,
    fila: OnceLock<Arc<Fila>>,
    extracoes: Mutex<Vec<String>>,
    _dir: tempfile::TempDir,
}

impl Ambiente for Teste {
    fn db(&self) -> &Db {
        &self.db
    }
    fn paths(&self) -> &AppPaths {
        &self.paths
    }
    fn notificar(&self, n: Notificacao) {
        match n {
            Notificacao::JobTerminou(job) => {
                extracao::ao_terminar(self, self.fila.get().unwrap(), &job).unwrap();
            }
            Notificacao::Extracao(id) => self.extracoes.lock().unwrap().push(id),
            _ => {}
        }
    }
}

const CAB: &str = "TRIBUNAL DE CONTAS DO ESTADO DE SÃO PAULO";

fn paginas() -> Vec<String> {
    let corpo = [
        "CONCURSO PÚBLICO PARA PROVIMENTO DE VAGAS NO CARGO DE AUDITOR DE CONTROLE EXTERNO\nEDITAL Nº 001/2026\nO Presidente do Tribunal de Contas do Estado torna pública a abertura de inscrições para o concurso público, que será regido pelas instruções a seguir.\n1. DAS DISPOSIÇÕES PRELIMINARES\n1.1. O concurso será executado pela Fundação para o Vestibular da Universidade Estadual Paulista.",
        "2. DO CONCURSO\n2.1. O concurso será realizado em duas etapas: prova objetiva e prova escrita, de caráter eliminatório e classificatório.",
        "3. DOS CARGOS\n3.1. Auditor de Controle Externo: 30 vagas, com vencimento de R$ 15.000,00 e jornada de 40 horas semanais.",
        "4. DAS INSCRIÇÕES\n4.1. As inscrições serão realizadas exclusivamente pela internet, no período previsto no cronograma do concurso.",
        "ANEXO I – CONTEÚDO PROGRAMÁTICO\nLíngua Portuguesa: Interpretação de texto. Ortografia oficial. Acentuação gráfica. Pontuação. Crase.",
        "ANEXO II – CRONOGRAMA PREVISTO\nInício das inscrições: 13 de julho de 2026. Aplicação da prova objetiva: 27 de setembro de 2026.",
    ];
    corpo.iter().enumerate().map(|(k, c)| format!("{}\n{CAB}\n{c}", k + 1)).collect()
}

fn ambiente() -> Arc<Teste> {
    // Mesma pasta para todos os testes: sem corrida no ambiente do processo.
    std::env::set_var(
        "FAKE_CLAUDE_RESPOSTAS",
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/respostas"),
    );
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(dir.path()).unwrap();
    let db = Db::open(&paths.db_file()).unwrap();
    let settings = Settings {
        claude_path: Some(env!("CARGO_BIN_EXE_fake-claude").to_owned()),
        timeout_secs: 60,
        ..Settings::default()
    };
    config_repo::save_settings(&db.conn().unwrap(), &settings).unwrap();
    let amb = Arc::new(Teste {
        db,
        paths,
        fila: OnceLock::new(),
        extracoes: Mutex::new(Vec::new()),
        _dir: dir,
    });
    let fila = Fila::nova(&amb.db, Esperas::default());
    amb.fila.set(Arc::clone(&fila)).ok().unwrap();
    tokio::spawn(fila.rodar(Arc::clone(&amb)));
    amb
}

fn importar(amb: &Teste, id: &str, arquivo: &str) {
    let mut conn = amb.db.conn().unwrap();
    edital_repo::inserir(
        &conn,
        &NovoEdital {
            id,
            nome: "Edital",
            arquivo_original: arquivo,
            sha256: id,
            importado_em: "2026-10-16T00:00:00Z",
        },
    )
    .unwrap();
    let paginas: Vec<PaginaInput> = paginas()
        .into_iter()
        .enumerate()
        .map(|(k, texto)| PaginaInput {
            pagina: k as u32 + 1,
            itens: vec![ItemTexto { t: texto.clone(), i: 0, b: None }],
            texto,
            origem: OrigemTexto::Pdf,
            confianca: None,
        })
        .collect();
    edital_repo::salvar_paginas(&mut conn, id, &paginas).unwrap();
    edital_repo::finalizar_importacao(&conn, id, paginas.len() as u32).unwrap();
}

async fn esperar_triagem(amb: &Teste, id: &str) -> EstadoExtracao {
    let inicio = Instant::now();
    loop {
        let e = extracao_repo::estado(&amb.db.conn().unwrap(), id).unwrap();
        if e.extracoes.first().is_some_and(|x| x.status != StatusExtracao::Pendente) {
            return e;
        }
        assert!(inicio.elapsed() < Duration::from_secs(20), "triagem presa: {e:?}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn mapa_e_triagem_classificam_as_secoes() {
    let amb = ambiente();
    importar(&amb, "e1", "edital.pdf");
    let fila = amb.fila.get().unwrap();
    let antes = extracao::processar(&*amb, fila, "e1", false, true).unwrap();
    let titulos: Vec<&str> = antes.secoes.iter().map(|s| s.titulo.as_str()).collect();
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
    assert!(antes.secoes.iter().all(|s| s.categorias.is_none()));

    // Processar de novo com a triagem na fila é recusado.
    let err = extracao::processar(&*amb, fila, "e1", false, true).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);

    let e = esperar_triagem(&amb, "e1").await;
    assert_eq!(e.extracoes[0].bloco, Bloco::Triagem);
    assert_eq!(e.extracoes[0].status, StatusExtracao::Concluido, "{e:?}");
    assert_eq!(e.e_edital, Some(true));
    let cat = |k: usize| e.secoes[k].categorias.clone().unwrap();
    assert_eq!(cat(2), [Categoria::DadosGerais, Categoria::ProvasAvaliacao]);
    assert_eq!(cat(5), [Categoria::ConteudoProgramatico]);
    // A fixture não classifica S6: vira `outros`.
    assert_eq!(cat(6), [Categoria::Outros]);

    // O prompt enviado traz o sumário, não o edital inteiro.
    let job = e.extracoes[0].job_id.clone().unwrap();
    let prompt = std::fs::read_to_string(amb.paths.job_dir(&job).join("prompt.md")).unwrap();
    assert!(prompt.contains("[S3] 3. DOS CARGOS (página 3, "));
    assert!(!prompt.contains(CAB), "cabeçalho repetido não vai no sumário");
    assert_eq!(std::fs::read_to_string(amb.paths.job_dir(&job).join("modelo.txt")).unwrap(), "haiku");
    assert!(amb.extracoes.lock().unwrap().iter().any(|x| x == "e1"));
}

#[tokio::test]
async fn nao_edital_falha_com_motivo() {
    let amb = ambiente();
    importar(&amb, "e2", "FAKE_VARIANTE=nao_edital.pdf");
    extracao::processar(&*amb, amb.fila.get().unwrap(), "e2", false, true).unwrap();
    let e = esperar_triagem(&amb, "e2").await;
    assert_eq!(e.extracoes[0].status, StatusExtracao::Falhou);
    assert_eq!(e.extracoes[0].erro_code, Some(ErrorCode::EditalNaoEEdital));
    assert_eq!(e.e_edital, Some(false));
    assert!(e.triagem_motivo.unwrap().contains("Gabarito"));
}

#[tokio::test]
async fn edital_sem_texto_nao_processa() {
    let amb = ambiente();
    let conn = amb.db.conn().unwrap();
    edital_repo::inserir(
        &conn,
        &NovoEdital { id: "e3", nome: "x", arquivo_original: "x.pdf", sha256: "e3", importado_em: "agora" },
    )
    .unwrap();
    conn.execute("UPDATE edital SET status = 'sem_texto' WHERE id = 'e3'", []).unwrap();
    drop(conn);
    let err = extracao::processar(&*amb, amb.fila.get().unwrap(), "e3", true, true).unwrap_err();
    assert_eq!(err.code, ErrorCode::EditalSemTexto);
}

/// Triagem real (haiku) sobre uma CÓPIA do banco do app. Não roda por padrão:
/// `EK_DB=<cópia> cargo test --test extracao triagem_real -- --ignored --nocapture`
#[tokio::test]
#[ignore]
async fn triagem_real_no_banco() {
    let caminho = std::env::var("EK_DB").expect("defina EK_DB com uma CÓPIA do banco");
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(dir.path()).unwrap();
    let db = Db::open(std::path::Path::new(&caminho)).unwrap();
    config_repo::save_settings(&db.conn().unwrap(), &Settings::default()).unwrap();
    let edital_id: String = db
        .conn()
        .unwrap()
        .query_row("SELECT id FROM edital WHERE status = 'importado' LIMIT 1", [], |r| r.get(0))
        .unwrap();
    let amb = Arc::new(Teste { db, paths, fila: OnceLock::new(), extracoes: Mutex::new(Vec::new()), _dir: dir });
    let fila = Fila::nova(&amb.db, Esperas::default());
    amb.fila.set(Arc::clone(&fila)).ok().unwrap();
    tokio::spawn(Arc::clone(&fila).rodar(Arc::clone(&amb)));

    extracao::processar(&*amb, &fila, &edital_id, true, true).unwrap();
    let inicio = Instant::now();
    let e = loop {
        let e = extracao_repo::estado(&amb.db.conn().unwrap(), &edital_id).unwrap();
        if e.extracoes.first().is_some_and(|x| x.status != StatusExtracao::Pendente) {
            break e;
        }
        assert!(inicio.elapsed() < Duration::from_secs(180), "triagem não terminou");
        tokio::time::sleep(Duration::from_millis(200)).await;
    };
    println!("e_edital = {:?} ({:?})", e.e_edital, e.triagem_motivo);
    for s in &e.secoes {
        println!("S{:<2} p{:>2}-{:<2} {:<60} {:?}", s.ordem, s.pagina_ini, s.pagina_fim, s.titulo, s.categorias.as_deref().unwrap_or_default());
    }
    let job = edital_killer_lib::storage::job_repo::get(&amb.db.conn().unwrap(), e.extracoes[0].job_id.as_deref().unwrap()).unwrap();
    println!("status {:?} {:?} | consumo {:?}", e.extracoes[0].status, e.extracoes[0].erro_detail, job.consumo);
    assert_eq!(e.extracoes[0].status, StatusExtracao::Concluido);
}
