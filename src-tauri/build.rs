fn main() {
    // tauri-build only embeds the manifest in the app executable; test binaries
    // without Common Controls v6 die on startup with STATUS_ENTRYPOINT_NOT_FOUND.
    // So we turn it off here and embed the same manifest in every binary.
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("tauri-build failed");
    embed_manifest();
}

fn embed_manifest() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        return;
    }
    println!("cargo:rerun-if-changed=windows/app.manifest");
    println!("cargo:rerun-if-changed=windows/app.rc");
    let dir = std::env::current_dir().unwrap().join("windows");
    if target.ends_with("msvc") {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
            dir.join("app.manifest").display()
        );
        return;
    }
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("app_manifest.o");
    let status = std::process::Command::new("windres")
        .current_dir(&dir)
        .args(["app.rc", "-O", "coff", "-o"])
        .arg(&out)
        .status()
        .expect("windres not found in PATH");
    assert!(status.success(), "windres failed to compile windows/app.rc");
    println!("cargo:rustc-link-arg={}", out.display());
}
