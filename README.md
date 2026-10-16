# Edital Killer

App desktop (Tauri 2 + React) que importa editais em PDF e extrai o conteúdo programático com o Claude Code CLI instalado na máquina. Especificação: `docs/Edital Killer Passo 1.md`.

## Pré-requisitos

- Rust (toolchain `stable-x86_64-pc-windows-msvc` no Windows) e Node 20+.
- Windows: Visual Studio Build Tools com a carga "Desenvolvimento para desktop com C++" (linker MSVC) e WebView2.
- Sem admin (sem Build Tools): toolchain `stable-x86_64-pc-windows-gnu` (`rustup override set` em `src-tauri`) + MinGW portátil ([WinLibs](https://winlibs.com), zip) com `mingw64/bin` no início do `PATH`. O aviso `.rsrc merge failure: multiple non-default manifests` vem do manifesto padrão do MinGW e é inofensivo.
- Claude Code instalado e logado (`claude auth status`), para a parte de IA.

## Comandos

```sh
npm install
npm run tauri dev      # app em modo desenvolvimento
npm test               # testes da UI (Vitest)
cd src-tauri && cargo test   # testes do core; também regenera src/bindings (ts-rs)
```

## Estado

| Item | Situação |
| --- | --- |
| 1. App desktop | Telas Biblioteca, Edital, Setup e Configurações; barra de status do Claude; painel de jobs (somente leitura); SQLite com migrações; logs rotacionados |
| 2. Upload do edital | Completo: dialog e arrastar-soltar, validação `%PDF`, SHA-256, duplicatas, extração por página com pdf.js, normalização com mapa de índices, remoção |
| 3. Claude CLI | Descoberta e health check (`--version`, `auth status`). Falta: runner, fila de jobs, catálogo de erros de execução, cancelamento ao fechar |
| 4. Extração | Não iniciado |
