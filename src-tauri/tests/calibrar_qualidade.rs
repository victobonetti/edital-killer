//! Calibração do classificador contra um banco real. Não roda por padrão.
//! `EK_DB=<cópia do banco> cargo test --test calibrar_qualidade -- --ignored --nocapture`

use edital_killer_lib::domain::qualidade::classificar;
use edital_killer_lib::storage::Db;

#[test]
#[ignore]
fn classificar_paginas_do_banco() {
    let caminho = std::env::var("EK_DB").expect("defina EK_DB com uma CÓPIA do banco");
    let db = Db::open(std::path::Path::new(&caminho)).unwrap();
    let conn = db.conn().unwrap();
    let mut stmt = conn
        .prepare("SELECT e.nome, p.numero, p.texto FROM pagina p JOIN edital e ON e.id = p.edital_id ORDER BY 1, 2")
        .unwrap();
    let linhas = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?, r.get::<_, String>(2)?)))
        .unwrap();
    for l in linhas {
        let (edital, n, texto) = l.unwrap();
        let amostra: String = texto.chars().filter(|c| !c.is_control()).take(70).collect();
        println!("{edital} p{n:>3} {:?} {} chars | {amostra}", classificar(&texto), texto.chars().count());
    }
}
