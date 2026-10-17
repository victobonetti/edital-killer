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
npm run dev            # só a UI no navegador, com o core simulado (src/dev/mockIpc.ts)
npm test               # testes da UI (Vitest)
cd src-tauri && cargo test   # testes do core (usa tools/fake-claude); também regenera src/bindings (ts-rs)
cd src-tauri && cargo test ponta_a_ponta -- --ignored --nocapture   # análise real com o claude logado (gasta uso)
```

## Estado

| Item | Situação |
| --- | --- |
| 1. App desktop | Telas Biblioteca, Edital (visão consolidada + documento original), Setup e Ajustes; indicador do Claude no cabeçalho; caminho, modelo, timeout, histórico de jobs e logs recolhidos em Ajustes › Avançado; SQLite com migrações; logs rotacionados |
| 2. Upload do edital | Completo: dialog e arrastar-soltar, validação `%PDF`, SHA-256, duplicatas, extração por página com pdf.js, normalização com mapa de índices, remoção |
| 3. Claude CLI | Descoberta, health check, runner `claude -p` com saída estruturada, fila serial, cancelamento (inclusive ao fechar), catálogo de erros (limite de uso, modelo indisponível, saída inválida…) |
| 4. Extração | Metadados do edital, cargos e conteúdo programático do cargo escolhido, verificados literalmente contra o texto. Falta: destaque do trecho no PDF |
