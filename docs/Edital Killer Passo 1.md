# Edital Killer — Especificação do Passo 1

Oct 5, 2026 · @vbo

## Objetivo e escopo

Ao fim do Passo 1, o usuário abre o app, sobe um edital em PDF, escolhe o cargo e vê o conteúdo programático extraído e verificado contra o PDF, lado a lado. O Claude Code CLI instalado na máquina faz a parte de IA.

**Entregas**

1. App desktop funcional: janela, navegação, persistência local.
2. Upload do edital: importação, extração de texto por página, biblioteca de editais.
3. Integração com o Claude Code CLI: detecção, health check, execução de jobs, catálogo de erros com UI explicativa.
4. Extração em tela: localizar cargos, validar o cargo, extrair matérias e tópicos, revisar contra o PDF.

**Premissas**

- Uso pessoal. O app chama o `claude` que o próprio usuário instalou e logou, no modelo do SwarmForge. O app nunca lê nem armazena credenciais do Claude.
- Alvos: Windows e macOS como principais, Linux sem garantia. A confirmar.
- Editais com texto nativo. PDF escaneado é detectado e sinalizado, mas o OCR fica fora do escopo.
- Um usuário por máquina, sem sincronização.

**Fora do escopo do Passo 1**

- Planejamento de estudos, cronograma e sessões.
- Vários editais cruzados: tópicos canônicos e matriz de sobreposição. A biblioteca já aceita vários editais, mas cada um é processado isoladamente.
- Cadernos de questões, integração com OpenQuestões e AnkiFactor.
- Adaptador BYOK (API key). A interface `LlmBackend` já nasce preparada, mas sem implementação.
- OCR.

## Stack e arquitetura

O core em Rust é dono de tudo que tem efeito colateral ou decide validade: processo, arquivos, banco, validação. A UI em React só pede e exibe.

| Camada | Escolha | Responsabilidade |
| --- | --- | --- |
| Shell | Tauri 2 | Janela, IPC (commands + events), empacotamento, dialog de arquivo |
| Core | Rust (tokio) | Runner do `claude`, fila de jobs, workspace, validação, verificação literal, SQLite |
| UI | React + TypeScript + Vite | Telas, estado de tela, pdf.js |
| PDF | pdf.js (`pdfjs-dist`) | Renderização, camada de texto, extração por página |
| Banco | SQLite via `rusqlite` + migrações versionadas | Editais, páginas, jobs, cargos, conteúdo extraído |
| Validação | `serde` + `jsonschema` | Saída do Claude validada no core |
| Testes | `cargo test`, Vitest, `fake-claude` | Ver seção de testes |

**Regras de fronteira**

- A UI nunca chama processo, nunca lê arquivo do workspace, nunca decide se um JSON é válido.
- Todo comando IPC devolve um tipo Rust serializado com tipo TS gerado (via `ts-rs` ou `specta`). Sem `any` cruzando a fronteira.
- Erros cruzam a fronteira como `AppError { code, detail }`, com `code` vindo do catálogo de erros. A UI traduz o `code` em tela explicativa; nunca mostra stderr cru como mensagem principal.
- A única exceção é o texto do PDF: o pdf.js extrai na UI e envia ao core uma vez, na importação. A partir daí o texto salvo no banco é a fonte de verdade.
- O core define a interface `LlmBackend`. No Passo 1 a única implementação é `ClaudeCliBackend`; os testes usam o mesmo backend apontando para o `fake-claude`.

**Estrutura de pastas**

```
edital-killer/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/        # handlers IPC, finos
│   │   ├── domain/          # Edital, Cargo, Materia, Topico (sem IO)
│   │   ├── llm/
│   │   │   ├── backend.rs   # trait LlmBackend
│   │   │   ├── claude_cli/  # discovery, health, runner, error_map
│   │   │   └── jobs.rs      # fila serial
│   │   ├── extraction/      # papéis, schemas, verificação literal
│   │   ├── storage/         # SQLite, migrações, repositórios
│   │   └── error.rs         # AppError + catálogo
│   ├── prompts/             # localizador.prompt, extrator.prompt
│   └── schemas/             # cargos.schema.json, conteudo.schema.json
├── src/                     # React
│   ├── screens/
│   ├── components/pdf/      # viewer pdf.js + destaque
│   ├── ipc/                 # wrappers tipados dos commands
│   └── bindings/            # tipos gerados do Rust
└── tools/fake-claude/       # binário de teste
```

Prompts e schemas são embutidos no binário (`include_str!`) e copiados para o workspace de cada job. Assim a versão do prompt fica atrelada à versão do app.

## Item 1 — App desktop funcional

O app abre numa tela de biblioteca, mostra o status do Claude CLI sempre visível e guarda tudo em SQLite no diretório de dados do app.

**Telas do Passo 1**

| Tela | Conteúdo | Sai para |
| --- | --- | --- |
| Boas-vindas / Setup | Checklist do Claude CLI (instalado, versão, login). Só aparece se o health check falhar na primeira execução | Biblioteca |
| Biblioteca | Lista de editais: nome, órgão, banca, data de importação, status do processamento. Botão "Importar edital" | Edital |
| Edital | Cabeçalho, visualizador do PDF, lista de cargos encontrados, ações de processamento | Revisão |
| Revisão da extração | Árvore de matérias e tópicos à esquerda, PDF com destaque à direita | Edital |
| Configurações | Caminho do `claude` (automático ou manual), modelo, timeout, pasta de dados, abrir logs | — |

**Elementos fixos**

- Barra de status com o indicador do Claude CLI: verde (pronto), amarelo (ocupado ou em limite de uso, com horário de retorno quando conhecido), vermelho (indisponível). Um clique abre o painel de diagnóstico.
- Painel de jobs: fila atual, job em execução com tempo decorrido e botão Cancelar, histórico dos últimos jobs.

**Persistência e diretórios**

- Raiz: diretório de dados do app resolvido pelo Tauri (`app_data_dir`).
- `edital-killer.db`: banco SQLite, com migrações aplicadas na inicialização.
- `editais/<edital_id>/original.pdf`: cópia do PDF importado. O app nunca depende do arquivo original do usuário.
- `jobs/<job_id>/`: workspace de cada execução do Claude. Mantido para auditoria; limpeza manual em Configurações no Passo 1.
- `logs/`: log estruturado do core, rotacionado.

**Comportamento de inicialização**

1. Aplica migrações do banco.
2. Marca como `interrompido` qualquer job que estava `executando` quando o app fechou.
3. Roda o health check do Claude CLI em segundo plano. A UI abre na hora; o indicador começa em "verificando".
4. Se o health check falhar e nunca tiver passado antes, abre a tela de Setup; senão, só o indicador fica vermelho.

**Fechar com job em execução:** o app pergunta se quer cancelar o job. Confirmando, mata o processo filho antes de sair. Nenhum processo `claude` pode sobreviver ao app.

## Item 2 — Upload do edital

Importar é só copiar o PDF e extrair o texto por página; nenhuma chamada ao Claude acontece aqui. Isso deixa a importação rápida, offline e testável.

**Fluxo**

1. Usuário clica em "Importar edital" (dialog nativo, filtro `.pdf`, seleção múltipla) ou arrasta arquivos para a Biblioteca.
2. Core valida e copia: confere o cabeçalho `%PDF`, calcula SHA-256 e copia para `editais/<id>/original.pdf`.
3. Duplicata (mesmo hash já importado) abre o edital existente em vez de criar outro.
4. UI abre o PDF com pdf.js e extrai o texto de cada página (`getTextContent`), mantendo a ordem dos itens.
5. UI envia ao core, em lotes, `{pagina, texto, itens[]}`. Cada item guarda o texto e o índice de caractere onde começa no texto da página. Esse mapa é o que permite destacar depois.
6. Core salva as páginas, marca páginas sem texto extraível e muda o status do edital para `importado`.
7. UI mostra o edital com nome provisório (nome do arquivo), editável.

**Validações e casos de borda**

| Caso | Comportamento |
| --- | --- |
| Arquivo não é PDF | Recusa com `IMPORT_NOT_PDF`, nada é copiado |
| PDF protegido por senha | Recusa com `IMPORT_ENCRYPTED`. Sem campo de senha no Passo 1 |
| PDF corrompido (pdf.js falha ao abrir) | Recusa com `IMPORT_CORRUPTED`, apaga a cópia |
| Nenhuma página com texto | Importa, mas marca `sem_texto` e bloqueia o processamento, explicando que OCR ainda não é suportado |
| Algumas páginas sem texto | Importa e lista as páginas no cabeçalho do edital como aviso |
| Duplicata | Abre o existente e avisa |
| Edital retificado (outro PDF do mesmo concurso) | Importa como edital separado no Passo 1. Vincular retificações fica para depois |

**Normalização do texto salvo**

O texto é salvo como o pdf.js entrega, sem limpeza. Uma segunda coluna guarda a versão normalizada usada na busca: sem acentos, minúsculas, espaços e quebras colapsados, hífen de fim de linha removido. A normalização guarda o mapa de índices de volta ao texto original, para o destaque cair no lugar certo.

**Remover edital:** apaga registros, a pasta do edital e os workspaces de jobs ligados a ele, após confirmação.
