//! Imita o `claude -p --output-format json` para os testes do core.
//!
//! O comportamento vem de arquivos no diretório de trabalho (o workspace do job),
//! para que testes em paralelo não disputem variáveis de ambiente:
//! - `fake-cenario.txt`: `ok` (padrão), `texto`, `sem_json`, `demora`, `limite_uso`,
//!   `creditos`, `sem_login`, `quebra`.
//! - `fake-saida.json`: a saída estruturada devolvida em `ok` e `texto`.
//!
//! Grava `fake-args.txt` (um argumento por linha) e `fake-stdin.txt` para os testes conferirem.

use std::io::Read;
use std::{env, fs, process, thread, time::Duration};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let _ = fs::write("fake-args.txt", args.join("\n"));
    let mut stdin = String::new();
    let _ = std::io::stdin().read_to_string(&mut stdin);
    let _ = fs::write("fake-stdin.txt", &stdin);

    let cenario = fs::read_to_string("fake-cenario.txt").unwrap_or_else(|_| "ok".into());
    let saida = fs::read_to_string("fake-saida.json").unwrap_or_else(|_| "{}".into());

    match cenario.trim() {
        "ok" => println!(
            r#"{{"type":"result","subtype":"success","is_error":false,"result":"pronto","structured_output":{}}}"#,
            saida.trim()
        ),
        "texto" => println!(
            r#"{{"type":"result","subtype":"success","is_error":false,"result":{}}}"#,
            json_string(&format!("Segue o JSON:\n{}", saida.trim()))
        ),
        "sem_json" => println!(r#"{{"type":"result","is_error":false,"result":"Não encontrei nada."}}"#),
        "demora" => {
            thread::sleep(Duration::from_secs(120));
            println!(r#"{{"type":"result","is_error":false,"structured_output":{{}}}}"#);
        }
        "limite_uso" => {
            println!(r#"{{"type":"result","is_error":true,"api_error_status":429,"result":"Claude AI usage limit reached|1893456000"}}"#);
            process::exit(1);
        }
        "creditos" => {
            println!(r#"{{"type":"result","is_error":true,"api_error_status":429,"api_error_code":"credits_required","result":"Model requires usage credits."}}"#);
            process::exit(1);
        }
        "sem_login" => {
            println!(r#"{{"type":"result","is_error":true,"result":"Invalid API key · Please run /login"}}"#);
            process::exit(1);
        }
        _ => {
            eprintln!("fake-claude: falha simulada");
            process::exit(3);
        }
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
