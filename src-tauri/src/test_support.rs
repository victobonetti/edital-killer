//! Apoio aos testes: compila o `tools/fake-claude` uma vez e devolve o caminho.

use std::path::PathBuf;
use std::sync::OnceLock;

pub fn fake_claude() -> PathBuf {
    static CAMINHO: OnceLock<PathBuf> = OnceLock::new();
    CAMINHO
        .get_or_init(|| {
            let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let target = raiz.join("target").join("fake-claude");
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
            let ok = std::process::Command::new(cargo)
                .args(["build", "--quiet", "--manifest-path"])
                .arg(raiz.join("../tools/fake-claude/Cargo.toml"))
                .arg("--target-dir")
                .arg(&target)
                .status()
                .expect("rodar cargo build do fake-claude")
                .success();
            assert!(ok, "falha ao compilar o fake-claude");
            target
                .join("debug")
                .join(format!("fake-claude{}", std::env::consts::EXE_SUFFIX))
        })
        .clone()
}
