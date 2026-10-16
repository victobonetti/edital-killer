# Edital Killer

App desktop (Tauri 2 + React) que importa editais em PDF e extrai o conteúdo programático com o Claude Code CLI instalado na máquina. Especificação: `docs/Edital Killer Passo 1.md`.

## Pré-requisitos

- Rust (toolchain `stable-x86_64-pc-windows-msvc` no Windows) e Node 20+.
- Windows: Visual Studio Build Tools com a carga "Desenvolvimento para desktop com C++" (linker MSVC) e WebView2.
- Sem admin (sem Build Tools): toolchain `stable-x86_64-pc-windows-gnu` (`rustup override set` em `src-tauri`) + MinGW portátil ([WinLibs](https://winlibs.com), zip) com `mingw64/bin` no início do `PATH`. O aviso `.rsrc merge failure: multiple non-default manifests` vem do manifesto padrão do MinGW e é inofensivo.
- Claude Code instalado e logado (`claude auth status`), para a parte de IA.

## Comandos

```sh
npm install            # npm run dev/build copiam os arquivos do OCR para public/ocr e baixam o modelo de português (2 MB) na primeira vez
npm run tauri dev      # app em modo desenvolvimento
npm test               # testes da UI (Vitest)
cd src-tauri && cargo test   # testes do core e da fila (contra o fake-claude); regenera src/bindings (ts-rs)
cd src-tauri && cargo test --test fila claude_real -- --ignored   # fila contra o claude real (haiku, gasta pouca cota)
```

## Estado

| Item | Situação |
| --- | --- |
| 1. App desktop | Telas Biblioteca, Edital, Setup e Configurações; barra de status com Claude, uso do plano e fila; painel de jobs com Cancelar, tempo decorrido e consumo; confirmação ao fechar com job rodando; SQLite com migrações; logs rotacionados. Falta: tela de Revisão |
| 2. Upload do edital | Completo: dialog e arrastar-soltar, validação `%PDF`, SHA-256, duplicatas, extração por página com pdf.js, qualidade do texto por página (`ok`, `sem_texto`, `suspeito`), OCR local com tesseract.js nas páginas ruins, cancelamento, normalização com mapa de índices, remoção |
| 3. Claude CLI | Feito: descoberta, health check, runner (`stream-json`, prompt por stdin, isolado de hooks/MCP/CLAUDE.md), fila serial persistida, catálogo de erros com novas tentativas, cancelamento da árvore de processos (Job Object no Windows), consumo por job, limite de uso com pausa e retomada, aviso de uso alto, teste de execução no Diagnóstico |
| 4. Extração | Não iniciado |
