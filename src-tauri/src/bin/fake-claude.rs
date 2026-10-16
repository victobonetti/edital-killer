//! Imitação do `claude` para os testes do runner e da fila. Não é distribuído.
//!
//! O cenário vem no prompt (stdin), em linhas próprias:
//!   CENARIO: sucesso | saida_invalida | erro_api | limite | sem_login | crash | lento
//!   RESPOSTA: {"ok": true}       (structured_output do sucesso)
//!   UTILIZACAO: 0.5              (janela de 5 h no rate_limit_event)
//! Grava os argumentos recebidos em `fake-args.json` no diretório de trabalho.
//!
//! Sem `RESPOSTA:`, procura a resposta do papel em `$FAKE_CLAUDE_RESPOSTAS/<title do schema>.json`
//! (ou `<title>.<variante>.json`, quando o prompt contém `FAKE_VARIANTE=<variante>`).

use std::io::{Read, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

#[allow(clippy::zombie_processes)] // o neto do cenário `lento`, de propósito
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version") {
        println!("9.9.9 (Fake Claude)");
        return;
    }
    if args.first().map(String::as_str) == Some("auth") {
        println!(r#"{{"loggedIn": true, "authMethod": "claude.ai"}}"#);
        return;
    }
    if args.iter().any(|a| a == "--dormir") {
        std::thread::sleep(Duration::from_secs(600));
        return;
    }
    let _ = std::fs::write("fake-args.json", serde_json::to_string(&args).unwrap());
    let par = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1));
    if !args.iter().any(|a| a == "-p") || par("--output-format").map(String::as_str) != Some("stream-json") {
        eprintln!("fake-claude: faltou -p --output-format stream-json");
        std::process::exit(2);
    }

    let mut prompt = String::new();
    std::io::stdin().read_to_string(&mut prompt).unwrap();
    let campo = |nome: &str| {
        prompt
            .lines()
            .find_map(|l| l.strip_prefix(nome).map(|v| v.trim().to_owned()))
    };
    let cenario = campo("CENARIO:").unwrap_or_else(|| "sucesso".into());
    let utilizacao: f64 = campo("UTILIZACAO:").and_then(|u| u.parse().ok()).unwrap_or(0.1);
    let agora = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    emitir(json!({"type": "system", "subtype": "init", "session_id": "fake"}));
    let status = if cenario == "limite" { "rejected" } else { "allowed" };
    emitir(json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "status": status,
            "resetsAt": agora + 3600,
            "rateLimitType": "five_hour",
            "unifiedWindows": {
                "five_hour": {"utilization": utilizacao, "resetsAt": agora + 3600},
                "seven_day": {"utilization": 0.2, "resetsAt": agora + 86400}
            }
        }
    }));

    match cenario.as_str() {
        "sucesso" => {
            let resposta: Value = campo("RESPOSTA:")
                .and_then(|r| serde_json::from_str(&r).ok())
                .or_else(|| resposta_do_papel(par("--json-schema")?, &prompt))
                .unwrap_or(json!({"ok": true}));
            emitir(resultado(false, None, Some(resposta), "ok"));
        }
        "saida_invalida" => emitir(resultado(false, None, Some(json!({"inesperado": 1})), "x")),
        "erro_api" => emitir(resultado(true, Some(529), None, "Overloaded")),
        "limite" => emitir(resultado(true, Some(429), None, "You've hit your limit")),
        "sem_login" => emitir(resultado(true, Some(401), None, "Please run /login")),
        "crash" => {
            eprintln!("boom: falha simulada");
            std::process::exit(3);
        }
        "lento" => {
            // Um neto, para provar que o cancelamento mata a árvore inteira. Fica sem
            // `wait` de propósito: quem tem que matá-lo é o runner.
            let exe = std::env::current_exe().unwrap();
            let neto = std::process::Command::new(exe).arg("--dormir").spawn().unwrap();
            std::fs::write("neto.pid", neto.id().to_string()).unwrap();
            std::thread::sleep(Duration::from_secs(600));
        }
        outro => {
            eprintln!("fake-claude: cenário desconhecido {outro}");
            std::process::exit(4);
        }
    }
}

fn resposta_do_papel(schema: &str, prompt: &str) -> Option<Value> {
    let dir = std::env::var_os("FAKE_CLAUDE_RESPOSTAS")?;
    let schema: Value = serde_json::from_str(schema).ok()?;
    let titulo = schema.get("title")?.as_str()?;
    let variante = prompt.split("FAKE_VARIANTE=").nth(1).map(|r| {
        r.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<String>()
    });
    let nome = match variante {
        Some(v) => format!("{titulo}.{v}.json"),
        None => format!("{titulo}.json"),
    };
    let texto = std::fs::read_to_string(std::path::Path::new(&dir).join(nome)).ok()?;
    serde_json::from_str(&texto).ok()
}

fn resultado(is_error: bool, http: Option<u16>, saida: Option<Value>, texto: &str) -> Value {
    json!({
        "type": "result",
        "subtype": if is_error { "error_during_execution" } else { "success" },
        "is_error": is_error,
        "api_error_status": http,
        "result": texto,
        "structured_output": saida,
        "duration_ms": 1200,
        "total_cost_usd": 0.01,
        "usage": {
            "input_tokens": 100,
            "output_tokens": 20,
            "cache_creation_input_tokens": 1000,
            "cache_read_input_tokens": 0,
            "output_tokens_details": {"thinking_tokens": 5}
        },
        "modelUsage": {"fake-model": {"costUSD": 0.01}}
    })
}

fn emitir(v: Value) {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{v}").unwrap();
    out.flush().unwrap();
}
