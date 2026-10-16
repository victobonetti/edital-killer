//! Fila + runner de ponta a ponta contra o `fake-claude` (src/bin/fake-claude.rs).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use edital_killer_lib::domain::{
    JanelaUso, Job, JobStatus, NivelUso, PerfilModelo, Settings, UsoClaude,
};
use edital_killer_lib::error::ErrorCode;
use edital_killer_lib::llm::jobs::{Ambiente, Esperas, Fila, NovoJob, Notificacao};
use edital_killer_lib::storage::{config_repo, job_repo, AppPaths, Db};

struct Teste {
    db: Db,
    paths: AppPaths,
    notificacoes: Mutex<Vec<Notificacao>>,
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
        self.notificacoes.lock().unwrap().push(n);
    }
}

fn ambiente(timeout_secs: u32) -> Arc<Teste> {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(dir.path()).unwrap();
    let db = Db::open(&paths.db_file()).unwrap();
    let settings = Settings {
        claude_path: Some(env!("CARGO_BIN_EXE_fake-claude").to_owned()),
        modelo_rapido: "fake".into(),
        modelo_forte: "fake-forte".into(),
        timeout_secs,
    };
    config_repo::save_settings(&db.conn().unwrap(), &settings).unwrap();
    Arc::new(Teste {
        db,
        paths,
        notificacoes: Mutex::new(Vec::new()),
        _dir: dir,
    })
}

fn esperas_curtas() -> Esperas {
    Esperas {
        erro_api: vec![Duration::from_millis(10), Duration::from_millis(10)],
        folga_reset: Duration::from_secs(60),
        sem_reset: Duration::from_secs(60),
        saida_invalida: 1,
    }
}

fn iniciar(amb: &Arc<Teste>) -> Arc<Fila> {
    let fila = Fila::nova(&amb.db, esperas_curtas());
    tokio::spawn(Arc::clone(&fila).rodar(Arc::clone(amb)));
    fila
}

fn novo(cenario: &str) -> NovoJob {
    NovoJob {
        edital_id: None,
        tipo: "teste".into(),
        system: "sistema".into(),
        prompt: format!("CENARIO: {cenario}\nRESPOSTA: {{\"ok\": true}}\nUTILIZACAO: 0.42\n"),
        schema: serde_json::json!({
            "type": "object",
            "properties": {"ok": {"type": "boolean"}},
            "required": ["ok"]
        }),
        perfil: PerfilModelo::Rapido,
    }
}

async fn esperar(amb: &Teste, id: &str, ate: impl Fn(&Job) -> bool) -> Job {
    let inicio = Instant::now();
    loop {
        let job = job_repo::get(&amb.db.conn().unwrap(), id).unwrap();
        if ate(&job) {
            return job;
        }
        assert!(inicio.elapsed() < Duration::from_secs(20), "job preso: {job:?}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn terminou(j: &Job) -> bool {
    !matches!(j.status, JobStatus::NaFila | JobStatus::Executando)
}

#[tokio::test]
async fn sucesso_grava_resultado_consumo_e_uso() {
    let amb = ambiente(60);
    let fila = iniciar(&amb);
    let job = fila.enfileirar(&*amb, novo("sucesso"), false).unwrap();
    let job = esperar(&amb, &job.id, terminou).await;

    assert_eq!(job.status, JobStatus::Concluido, "{job:?}");
    assert_eq!(job.tentativas, 1);
    let c = job.consumo.unwrap();
    assert_eq!((c.tokens_entrada, c.tokens_cache_escrita, c.tokens_raciocinio), (100, 1000, 5));
    assert_eq!(c.modelo.as_deref(), Some("fake-model"));
    assert_eq!(job.utilizacao_5h, Some(0.42));

    let ws = amb.paths.job_dir(&job.id);
    let resultado: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(ws.join("resultado.json")).unwrap()).unwrap();
    assert_eq!(resultado, serde_json::json!({"ok": true}));
    assert!(ws.join("saida.jsonl").exists());

    let args: Vec<String> =
        serde_json::from_str(&std::fs::read_to_string(ws.join("fake-args.json")).unwrap()).unwrap();
    let par = |f: &str| args[args.iter().position(|a| a == f).unwrap() + 1].clone();
    assert_eq!(par("--tools"), "");
    assert_eq!(par("--model"), "fake");
    assert_eq!(par("--system-prompt-file"), "system.md");

    let uso = config_repo::uso_claude(&amb.db.conn().unwrap()).unwrap().unwrap();
    assert_eq!(uso.nivel, NivelUso::Normal);
    let notifs = amb.notificacoes.lock().unwrap();
    assert!(notifs.iter().any(|n| matches!(n, Notificacao::Uso(_))));
    assert!(notifs.iter().any(|n| matches!(n, Notificacao::JobTerminou(j) if j.id == job.id)));
}

#[tokio::test]
async fn saida_invalida_tenta_de_novo_uma_vez() {
    let amb = ambiente(60);
    let fila = iniciar(&amb);
    let job = fila.enfileirar(&*amb, novo("saida_invalida"), false).unwrap();
    let job = esperar(&amb, &job.id, terminou).await;
    assert_eq!(job.status, JobStatus::Falhou);
    assert_eq!(job.erro_code, Some(ErrorCode::ClaudeSaidaInvalida));
    assert_eq!(job.tentativas, 2);
    // Consumo soma as duas execuções.
    assert_eq!(job.consumo.unwrap().tokens_entrada, 200);
}

#[tokio::test]
async fn erro_api_tenta_mais_duas_vezes() {
    let amb = ambiente(60);
    let fila = iniciar(&amb);
    let job = fila.enfileirar(&*amb, novo("erro_api"), false).unwrap();
    let job = esperar(&amb, &job.id, terminou).await;
    assert_eq!(job.erro_code, Some(ErrorCode::ClaudeErroApi));
    assert_eq!(job.tentativas, 3);
}

#[tokio::test]
async fn limite_devolve_para_a_fila_e_pausa() {
    let amb = ambiente(60);
    let fila = iniciar(&amb);
    let job = fila.enfileirar(&*amb, novo("limite"), false).unwrap();
    let inicio = Instant::now();
    while fila.estado(&amb.db).pausada_ate.is_none() {
        assert!(inicio.elapsed() < Duration::from_secs(20), "fila não pausou");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let job = job_repo::get(&amb.db.conn().unwrap(), &job.id).unwrap();
    assert_eq!(job.status, JobStatus::NaFila);
    assert_eq!(job.tentativas, 0, "recusa por limite não conta tentativa");

    // Pausa persistida: uma fila nova (app reaberto) continua pausada.
    let reaberta = Fila::nova(&amb.db, esperas_curtas());
    assert!(reaberta.estado(&amb.db).pausada_ate.is_some());
    let uso = config_repo::uso_claude(&amb.db.conn().unwrap()).unwrap().unwrap();
    assert_eq!(uso.nivel, NivelUso::Limite);

    // Cancelar um job que está na fila (pausada) funciona.
    fila.cancelar(&*amb, &job.id).unwrap();
    let job = job_repo::get(&amb.db.conn().unwrap(), &job.id).unwrap();
    assert_eq!(job.status, JobStatus::Cancelado);
}

#[tokio::test]
async fn sem_login_falha_e_avisa() {
    let amb = ambiente(60);
    let fila = iniciar(&amb);
    let job = fila.enfileirar(&*amb, novo("sem_login"), false).unwrap();
    let job = esperar(&amb, &job.id, terminou).await;
    assert_eq!(job.erro_code, Some(ErrorCode::ClaudeNotLoggedIn));
    assert!(amb
        .notificacoes
        .lock()
        .unwrap()
        .iter()
        .any(|n| matches!(n, Notificacao::LoginPerdido)));
}

#[tokio::test]
async fn processo_que_morre_traz_o_stderr() {
    let amb = ambiente(60);
    let fila = iniciar(&amb);
    let job = fila.enfileirar(&*amb, novo("crash"), false).unwrap();
    let job = esperar(&amb, &job.id, terminou).await;
    assert_eq!(job.erro_code, Some(ErrorCode::ClaudeProcessoFalhou));
    assert!(job.erro_detail.unwrap().contains("boom"));
}

#[tokio::test]
async fn timeout_mata_e_falha() {
    let amb = ambiente(1);
    let fila = iniciar(&amb);
    let job = fila.enfileirar(&*amb, novo("lento"), false).unwrap();
    let job = esperar(&amb, &job.id, terminou).await;
    assert_eq!(job.erro_code, Some(ErrorCode::ClaudeTimeout));
    let pid = ler_pid_neto(&amb, &job.id).await;
    esperar_morto(pid).await;
}

#[tokio::test]
async fn cancelar_mata_a_arvore_inteira() {
    let amb = ambiente(60);
    let fila = iniciar(&amb);
    let job = fila.enfileirar(&*amb, novo("lento"), false).unwrap();
    let pid = ler_pid_neto(&amb, &job.id).await;
    assert!(vivo(pid));
    fila.cancelar(&*amb, &job.id).unwrap();
    let job = esperar(&amb, &job.id, terminou).await;
    assert_eq!(job.status, JobStatus::Cancelado);
    esperar_morto(pid).await;
}

#[tokio::test]
async fn cancelar_tudo_esvazia_a_fila() {
    let amb = ambiente(60);
    let fila = iniciar(&amb);
    let a = fila.enfileirar(&*amb, novo("lento"), false).unwrap();
    let b = fila.enfileirar(&*amb, novo("sucesso"), false).unwrap();
    esperar(&amb, &a.id, |j| j.status == JobStatus::Executando).await;
    fila.cancelar_tudo(&*amb).await;
    let estado = fila.estado(&amb.db);
    assert_eq!((estado.executando, estado.na_fila), (None, 0));
    for id in [a.id, b.id] {
        assert_eq!(job_repo::get(&amb.db.conn().unwrap(), &id).unwrap().status, JobStatus::Cancelado);
    }
}

#[tokio::test]
async fn uso_alto_pede_confirmacao() {
    let amb = ambiente(60);
    let fila = Fila::nova(&amb.db, esperas_curtas());
    let futuro = (chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339();
    let uso = UsoClaude {
        nivel: NivelUso::Aviso,
        status: "allowed_warning".into(),
        janela_5h: Some(JanelaUso {
            utilizacao: 0.85,
            renova_em: Some(futuro),
        }),
        janela_7d: None,
        renova_em: None,
        atualizado_em: chrono::Utc::now().to_rfc3339(),
    };
    config_repo::salvar_uso_claude(&amb.db.conn().unwrap(), &uso).unwrap();

    let err = fila.enfileirar(&*amb, novo("sucesso"), false).unwrap_err();
    assert_eq!(err.code, ErrorCode::ClaudeUsoAlto);
    assert!(err.detail.contains("85%"), "{}", err.detail);
    assert!(fila.enfileirar(&*amb, novo("sucesso"), true).is_ok());
}

async fn ler_pid_neto(amb: &Teste, job_id: &str) -> u32 {
    let arquivo = amb.paths.job_dir(job_id).join("neto.pid");
    let inicio = Instant::now();
    loop {
        if let Some(pid) = std::fs::read_to_string(&arquivo).ok().and_then(|s| s.trim().parse().ok()) {
            return pid;
        }
        assert!(inicio.elapsed() < Duration::from_secs(20), "neto não nasceu");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn esperar_morto(pid: u32) {
    let inicio = Instant::now();
    while vivo(pid) {
        assert!(inicio.elapsed() < Duration::from_secs(5), "neto {pid} sobreviveu");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[cfg(windows)]
fn vivo(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    // SAFETY: consulta de leitura num handle aberto e fechado aqui.
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let mut codigo = 0u32;
        let ok = GetExitCodeProcess(h, &mut codigo);
        CloseHandle(h);
        ok != 0 && codigo == STILL_ACTIVE as u32
    }
}

#[cfg(unix)]
fn vivo(pid: u32) -> bool {
    // SAFETY: sinal 0 só verifica existência.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// Contra o `claude` real (gasta um pouco de cota). Rodar com:
/// `cargo test --test fila claude_real -- --ignored`
#[tokio::test]
#[ignore]
async fn claude_real_com_haiku() {
    let amb = ambiente(120);
    let settings = Settings {
        claude_path: None,
        modelo_rapido: "haiku".into(),
        modelo_forte: "sonnet".into(),
        timeout_secs: 120,
    };
    config_repo::save_settings(&amb.db.conn().unwrap(), &settings).unwrap();
    let fila = iniciar(&amb);
    let mut pedido = novo("sucesso");
    pedido.prompt = "Responda com ok = true.".into();
    let job = fila.enfileirar(&*amb, pedido, true).unwrap();
    let job = esperar(&amb, &job.id, terminou).await;
    assert_eq!(job.status, JobStatus::Concluido, "{job:?}");
    let c = job.consumo.unwrap();
    assert!(c.modelo.unwrap().contains("haiku"));
    assert!(c.tokens_saida > 0);
    assert!(job.utilizacao_5h.is_some(), "rate_limit_event não chegou");
    eprintln!("consumo real: {:?}", job_repo::get(&amb.db.conn().unwrap(), &job.id).unwrap().consumo);
}
