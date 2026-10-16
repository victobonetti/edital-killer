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
- Editais com texto nativo ou escaneados. Páginas sem texto ou com texto ilegível passam por OCR local (tesseract.js), sem gastar cota do Claude.
- Um usuário por máquina, sem sincronização.

**Fora do escopo do Passo 1**

- Planejamento de estudos, cronograma e sessões.
- Vários editais cruzados: tópicos canônicos e matriz de sobreposição. A biblioteca já aceita vários editais, mas cada um é processado isoladamente.
- Cadernos de questões, integração com OpenQuestões e AnkiFactor.
- Adaptador BYOK (API key). A interface `LlmBackend` já nasce preparada, mas sem implementação.
- OCR de tabelas complexas ou manuscritos: o OCR do Passo 1 mira texto impresso corrido.

## Stack e arquitetura

O core em Rust é dono de tudo que tem efeito colateral ou decide validade: processo, arquivos, banco, validação. A UI em React só pede e exibe.

| Camada | Escolha | Responsabilidade |
| --- | --- | --- |
| Shell | Tauri 2 | Janela, IPC (commands + events), empacotamento, dialog de arquivo |
| Core | Rust (tokio) | Runner do `claude`, fila de jobs, workspace, validação, verificação literal, SQLite |
| UI | React + TypeScript + Vite | Telas, estado de tela, pdf.js |
| PDF | pdf.js (`pdfjs-dist`) | Renderização, camada de texto, extração por página |
| OCR | tesseract.js + `por.traineddata` embutido | OCR local das páginas sem texto ou ilegíveis, com caixa de cada palavra |
| Banco | SQLite via `rusqlite` + migrações versionadas | Editais, páginas, jobs, cargos, conteúdo extraído |
| Validação | `serde` + `jsonschema` | Saída do Claude validada no core |
| Testes | `cargo test`, Vitest, `fake-claude` | Fila e runner testados de ponta a ponta contra o `fake-claude` (`tests/fila.rs`); um teste `#[ignore]` roda contra o `claude` real |

**Regras de fronteira**

- A UI nunca chama processo, nunca lê arquivo do workspace, nunca decide se um JSON é válido.
- Todo comando IPC devolve um tipo Rust serializado com tipo TS gerado (via `ts-rs` ou `specta`). Sem `any` cruzando a fronteira.
- Erros cruzam a fronteira como `AppError { code, detail }`, com `code` vindo do catálogo de erros. A UI traduz o `code` em tela explicativa; nunca mostra stderr cru como mensagem principal.
- A única exceção é o texto do PDF: o pdf.js (ou o OCR) extrai na UI e envia ao core uma vez, na importação. A partir daí o texto salvo no banco é a fonte de verdade. Quem classifica a qualidade do texto é o core, não a UI.
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
│   │   │   ├── claude_cli/  # discovery, health, runner, stream (erros), processo (árvore)
│   │   │   └── jobs.rs      # fila serial
│   │   ├── bin/fake-claude.rs  # imitação do claude para os testes
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
6. Core salva as páginas e classifica a qualidade de cada uma (ver abaixo). A resposta do lote lista as páginas que precisam de OCR.
7. UI roda o OCR nessas páginas e reenvia cada uma com `origem = ocr`. O core reclassifica.
8. Core aplica a regra do edital (ver abaixo) e muda o status para `importado` ou `sem_texto`.
9. UI mostra o edital com nome provisório (nome do arquivo), editável.

**Qualidade do texto por página**

O core classifica cada página como `ok`, `sem_texto` ou `suspeito`. `suspeito` é o PDF que tem camada de texto, mas ela sai ilegível (fonte sem mapa de caracteres: `Ã§`, `�`, símbolos soltos). Sem essa checagem, a página passaria como válida e o Claude receberia lixo.

| Sinal | Página `suspeito` quando |
| --- | --- |
| Caracteres de substituição (`U+FFFD`) e de uso privado | > 1% dos caracteres |
| Pares de mojibake (`Ã§`, `Ã£`, `Â`…: UTF-8 lido como Latin-1) | > 0,5% dos caracteres |
| Letras e dígitos sobre caracteres não-brancos (dígitos contam: quadros de vagas e cronogramas são numéricos) | < 60% |
| Palavras comuns do português (lista fixa de ~200: de, a, o, que, para, com…) | < 15% dos tokens, em página com ≥ 50 tokens |

Uma página com menos de 20 caracteres não-brancos é `sem_texto`. Os limites são iniciais. Calibração em 2026-10-16 com o edital TCE-SP 2026 (47 páginas, texto nativo): 47/47 `ok`, inclusive cronograma e anexos curtos. Para recalibrar com outro banco: `EK_DB=<cópia do banco> cargo test --test calibrar_qualidade -- --ignored --nocapture`.

**OCR**

- Roda na UI, com tesseract.js num Web Worker, só nas páginas `sem_texto` ou `suspeito`. A página é renderizada pelo pdf.js a 300 dpi.
- `por.traineddata` vai embutido no app; o OCR não acessa a rede.
- Cada palavra reconhecida vira um item `{t, i, b}`: além do texto e do offset, `b` guarda a caixa `[x, y, largura, altura]` em pontos, a partir do canto superior esquerdo da página (o viewport do pdf.js em escala 1). É essa caixa que permite destacar trechos em páginas sem camada de texto.
- Medido em 2026-10-16 (modelo `tessdata_fast`, 300 dpi em Node): 2,5–9 s por página, confiança 86–94 em páginas do TCE-SP 2026 rasterizadas; o conteúdo programático sai limpo, o ruído fica em logotipos.
- A página guarda `origem` (`pdf` ou `ocr`) e, quando é OCR, a confiança média do tesseract (0–100). Com confiança abaixo de 70, a página continua `suspeito`.
- O progresso aparece por página e pode ser cancelado. Cancelar aborta a importação inteira (`importar_abortar`).
- Se o OCR deixar a página pior que o texto nativo, o core fica com o texto nativo.

**Regra do edital**

- Todas as páginas `sem_texto` ou `suspeito` depois do OCR: status `sem_texto`, processamento bloqueado (`EDITAL_SEM_TEXTO`).
- Até 20% das páginas ruins: importa e lista as páginas no cabeçalho como aviso. As páginas lidas por OCR também são listadas, como informação.
- Mais de 20% ruins: importa, mas o processamento exige confirmação explícita do usuário.
- Se o edital é mesmo um edital, quem diz é o localizador (Item 4), que pode responder `nao_e_edital`.

**Validações e casos de borda**

| Caso | Comportamento |
| --- | --- |
| Arquivo não é PDF | Recusa com `IMPORT_NOT_PDF`, nada é copiado |
| PDF protegido por senha | Recusa com `IMPORT_ENCRYPTED`. Sem campo de senha no Passo 1 |
| PDF corrompido (pdf.js falha ao abrir) | Recusa com `IMPORT_CORRUPTED`, apaga a cópia |
| Nenhuma página com texto | Roda OCR em todas. Se continuar sem texto, marca `sem_texto` e bloqueia o processamento |
| Algumas páginas sem texto ou ilegíveis | Roda OCR nelas e aplica a regra do edital |
| OCR falha (worker cai, dado de idioma ausente) | `IMPORT_OCR_FAILED`; a página fica como estava e a regra do edital decide |
| Duplicata | Abre o existente e avisa |
| Edital retificado (outro PDF do mesmo concurso) | Importa como edital separado no Passo 1. Vincular retificações fica para depois |

**Editais importados antes do OCR:** a migração 003 reclassifica as páginas já salvas e recalcula o status dos editais. O OCR não roda retroativamente; para aplicá-lo, remova e importe o edital de novo.

**Normalização do texto salvo**

O texto é salvo como o pdf.js entrega, sem limpeza. Uma segunda coluna guarda a versão normalizada usada na busca: sem acentos, minúsculas, espaços e quebras colapsados, hífen de fim de linha removido. A normalização guarda o mapa de índices de volta ao texto original, para o destaque cair no lugar certo.

**Remover edital:** apaga registros, a pasta do edital e os workspaces de jobs ligados a ele, após confirmação.

## Item 3 — Integração com o Claude Code CLI

**Runner**

Cada job roda uma vez o `claude` em modo não interativo, com o workspace `jobs/<job_id>/` como diretório de trabalho:

```
claude -p --output-format stream-json --verbose --no-session-persistence
       --tools "" --strict-mcp-config --setting-sources ""
       --system-prompt <prompt de sistema do papel>
       --json-schema <schema do papel> [--model <Configurações>]
```

- O prompt do usuário vai por **stdin**, com o texto do edital embutido. Não passa pela linha de comando (limite de ~32 mil caracteres no Windows) e não usa ferramentas: o Claude recebe tudo no prompt e responde numa volta só. Ler arquivos com ferramentas custaria uma volta, e um reenvio do contexto, por arquivo.
- `--tools ""`, `--strict-mcp-config` e `--setting-sources ""` isolam o job da configuração pessoal do usuário: nada de hooks, MCP ou CLAUDE.md dele interferindo. O login continua funcionando. `--bare` não serve, porque ignora o login OAuth.
- `--system-prompt` substitui o prompt padrão do Claude Code. Medido em 2026-10-16: o overhead fixo cai de ~13,2 mil para ~1,1 mil tokens por chamada.
- O modo `stream-json` é obrigatório: é o único que traz o `rate_limit_event`. O evento final `result` traz consumo, `structured_output` e `api_error_status`.
- O core valida `structured_output` contra o schema (`jsonschema`), mesmo com o `--json-schema` do CLI: a fronteira de validade é do core.
- O workspace guarda, para auditoria: `prompt.md`, `system.md`, `schema.json`, `saida.jsonl` (stdout cru), `stderr.txt` e `resultado.json`.

**Fila**

- Serial: um job por vez, em ordem de criação. Sem prioridades no Passo 1; os papéis de um edital rodam em sequência de qualquer forma.
- A fila é persistida no banco (`na_fila`). Ao abrir o app, os jobs `na_fila` continuam, exceto quando a fila está pausada por limite de uso.
- Eventos para a UI: `jobs-atualizados` (qualquer mudança de status) e `claude-uso` (utilização das janelas).
- Enquanto um job roda, o indicador fica amarelo ("ocupado"). Com a fila pausada por limite, fica amarelo com "volta às HH:MM".

**Erros de execução**

| Código | Quando | Novas tentativas |
| --- | --- | --- |
| `CLAUDE_LIMITE_USO` | `rate_limit_event` de recusa ou HTTP 429 | Volta para a fila; não conta tentativa |
| `CLAUDE_TIMEOUT` | Passou de `timeout_secs` (Configurações) | Nenhuma: o processo é morto e o job falha |
| `CLAUDE_SAIDA_INVALIDA` | Sem `structured_output` ou fora do schema | 1 automática |
| `CLAUDE_ERRO_API` | `result` com erro de API (5xx, 529 sobrecarga) | 2 automáticas, com espera de 30 s e 120 s |
| `CLAUDE_NOT_LOGGED_IN` | HTTP 401/403 durante o job | Nenhuma; dispara um health check |
| `CLAUDE_PROCESSO_FALHOU` | Processo saiu sem evento `result` | Nenhuma; `detail` traz o fim do stderr |

**Cancelamento**

- Windows: cada execução entra num Job Object com `KILL_ON_JOB_CLOSE`. Cancelar encerra o Job Object, o que mata a árvore inteira. Se o app morrer por qualquer motivo, o Windows fecha o handle e mata o `claude` junto.
- macOS/Linux: o `claude` roda no próprio grupo de processos; cancelar envia `SIGKILL` ao grupo.
- O painel de jobs tem Cancelar no job em execução e em cada job na fila.
- Fechar a janela com job em execução: o app pergunta ("Cancelar 1 job em execução e 2 na fila e sair?"). Confirmando, cancela todos e sai. Com a fila só pausada por limite, fecha sem perguntar e os jobs continuam na fila para a próxima abertura.

**Consumo por job**

Registrado a partir do evento `result` de cada execução, em colunas novas da tabela `job`:

| Campo | Origem no `result` |
| --- | --- |
| `modelo` | chave de `modelUsage` |
| `tokens_entrada`, `tokens_saida` | `usage.input_tokens`, `usage.output_tokens` |
| `tokens_cache_escrita`, `tokens_cache_leitura` | `usage.cache_creation_input_tokens`, `usage.cache_read_input_tokens` |
| `tokens_raciocinio` | `usage.output_tokens_details.thinking_tokens` |
| `custo_estimado_usd` | `total_cost_usd` |
| `duracao_ms` | `duration_ms` |
| `utilizacao_5h`, `utilizacao_7d` | último `rate_limit_event` do job |

- `custo_estimado_usd` é preço de tabela. Com login do claude.ai ele não é cobrado; a UI rotula como "custo estimado (equivalente API)". O que limita o usuário é a utilização das janelas.
- Painel de jobs: tokens e custo de cada job. Cabeçalho do edital: soma dos jobs do edital.
- Barra de status: utilização das janelas de 5 h e de 7 dias, com horário de renovação, vindos do último `rate_limit_event` visto (persistido em `config`, para sobreviver ao reinício).
- Cada chamada carrega ~13 mil tokens fixos do próprio Claude Code (medido em 2026-10-16, CLI 2.1.301, sem ferramentas). O runner deve medir o efeito de `--bare` e de `--tools` mínimo antes de fixar as flags.

**Limite de uso**

Payload observado (`rate_limit_info`):

```json
{"status": "allowed", "resetsAt": 1792186800, "rateLimitType": "five_hour",
 "overageStatus": "rejected", "isUsingOverage": false,
 "unifiedWindows": {"five_hour": {"utilization": 0.08, "resetsAt": 1792186800},
                    "seven_day": {"utilization": 0.21, "resetsAt": 1792677600}}}
```

| Situação | Comportamento |
| --- | --- |
| `status` = `allowed` | Normal |
| `status` com aviso (qualquer valor diferente de `allowed` que não seja recusa) | Indicador amarelo com a utilização; o job segue |
| `status` de recusa, ou `result` com `api_error_status` 429 | Job falha com `CLAUDE_LIMITE_USO` e volta para a fila (`tentativas` + 1). A fila pausa até `resetsAt` + 1 min. O indicador fica amarelo com "volta às HH:MM" |
| Limite atingido sem `resetsAt` | A fila pausa por 30 min e tenta o job de novo; se ainda estiver em limite, pausa outra vez |
| App fechado durante a pausa | Na abertura, se `resetsAt` já passou, a fila retoma; senão, reagenda |

- A retomada é automática. Um botão "Retomar agora" fica disponível no painel de jobs.
- Erros de limite não contam para nenhum teto de tentativas: a culpa não é do job.
- **Aviso antecipado:** ao enfileirar um job, se a última utilização conhecida da janela de 5 h ou de 7 dias for ≥ 80%, a UI pede confirmação ("Sua janela de 5 h está em 83%, renova às 18:40. Iniciar mesmo assim?"). Sem utilização conhecida, não há aviso. Nenhuma trava corta job no meio.
- O formato do evento de recusa não foi observado (exigiria esgotar a cota). O `fake-claude` simula o caso nos testes; o primeiro caso real confirma o formato, e qualquer `status` desconhecido é registrado no log.

## Item 4 — Extração do edital

Dividir e conquistar: o core fatia o edital localmente, uma triagem barata diz o que é cada parte, e cada bloco de dados é extraído por um job que recebe **só o texto das suas seções**. Tudo o que for extraído cita a página e o trecho de origem e é verificado literalmente pelo core. Depois da curadoria do usuário, o edital nunca mais precisa ser relido pelo Claude: o resto do produto usa só o banco.

**Etapas**

| # | Etapa | Claude? | Entrada | Saída |
| --- | --- | --- | --- | --- |
| 1 | Mapa | Não | Texto das páginas | Seções: título, faixa de páginas, tamanho |
| 2 | Triagem | Sim, modelo rápido | Sumário: título + primeiros ~300 caracteres de cada seção | É edital? Categoria de cada seção. Lista de cargos |
| 3 | Extração por bloco | Sim | Só as seções da categoria do bloco | Dados estruturados, cada um com página e trecho |
| 4 | Verificação literal | Não | Trechos citados | Cada item `verificado` ou `nao_verificado`, com offsets para destaque |
| 5 | Curadoria | Não | Revisão do usuário | Itens `revisado`; edital `curado` |

**Etapa 1 — Mapa (local)**

- Títulos de seção por regex sobre o texto de cada página: `N. DA/DO/DAS/DOS …` em caixa alta e `ANEXO <romano> – …`. Validado no TCE-SP 2026: 15 seções e 6 anexos, todos nas páginas certas.
- Cabeçalhos e rodapés repetidos (linha idêntica em mais da metade das páginas, como "TRIBUNAL DE CONTAS DO ESTADO DE SÃO PAULO") são ignorados no mapa e no texto enviado ao Claude. O texto salvo não muda.
- Sem títulos reconhecíveis: o mapa cai para blocos de 5 páginas, e a triagem classifica blocos em vez de seções.

**Etapa 2 — Triagem** (papel `triagem`, modelo rápido)

- Entrada: o sumário do mapa. Alguns milhares de tokens, independente do tamanho do edital.
- Saída: `e_edital` (com motivo, para recusar provas, gabaritos e retificações soltas), a categoria de cada seção e a lista de cargos (nome, código, área, página).
- Categorias: `dados_gerais`, `inscricao`, `cargos_vagas`, `requisitos_atribuicoes`, `cronograma`, `provas_avaliacao`, `conteudo_programatico`, `outros`. `outros` (formulários, recursos, disposições finais) não é extraído.
- `e_edital = false` encerra o processamento com `EDITAL_NAO_E_EDITAL`.

**Etapa 3 — Blocos**

Depois da triagem, o usuário vê a lista de cargos e marca os de interesse. Os blocos gerais rodam sempre; os blocos por cargo rodam só para os marcados (e para outros, sob demanda).

| Bloco (papel) | Escopo | Modelo | Seções | Dados |
| --- | --- | --- | --- | --- |
| `dados_gerais` | Edital | Rápido | `dados_gerais`, `inscricao` | Órgão, banca, número e data do edital, taxa de inscrição, período de inscrição, isenção (período e critérios resumidos), validade do concurso, links oficiais |
| `cargos` | Edital | Rápido | `cargos_vagas`, `requisitos_atribuicoes` | Por cargo: nome, código, área/especialidade, escolaridade, requisitos, atribuições (resumo), vagas por modalidade (ampla, PcD, negros, indígenas, cadastro reserva), remuneração, jornada, lotação |
| `cronograma` | Edital | Rápido | `cronograma` (e datas citadas em `inscricao`) | Eventos: nome, data ou período, observação |
| `provas` | Por cargo | Forte | `provas_avaliacao` | Etapas (objetiva, discursiva, títulos, TAF, oral, outra): caráter (eliminatório, classificatório), data, duração; por etapa, matérias com nº de questões e peso; nota mínima; regras da discursiva; títulos pontuáveis; critérios de desempate |
| `conteudo` | Por cargo | Forte | `conteudo_programatico` | Matérias → tópicos → subtópicos, hierárquico e literal (ver abaixo) |

- O texto vai ao Claude com marcadores de página (`[[página 32]]`), para cada item citar `pagina`.
- Campos escalares (vagas, datas, valores) trazem `trecho`: a passagem literal, de até 200 caracteres, que comprova o valor.
- O modelo de cada papel vem das Configurações: **modelo rápido** (padrão `haiku`) e **modelo forte** (padrão `sonnet`). Isso substitui o campo único `model`.
- Prompts e schemas ficam em `src-tauri/prompts/` e `src-tauri/schemas/`, embutidos com `include_str!`. Cada papel tem uma `versao`, gravada com o resultado.

**Conteúdo programático — hierárquico e literal**

- Cada nó é `{numero, texto, pagina, filhos[]}`. `numero` é a numeração do edital quando houver ("1.", "1.1", "a)"); `texto` é a cópia **literal** do edital, sem reescrever nem resumir.
- O nível 1 é a matéria ("Língua Portuguesa", "Conhecimentos Específicos: Estatística"). Tópicos e subtópicos seguem a estrutura do edital: enumerações separadas por ponto, ponto e vírgula ou numeração viram irmãos.
- Conteúdo comum a vários cargos ("Conhecimentos Gerais para todos os cargos") é extraído uma vez e ligado a cada cargo.
- Se o anexo passar de ~60 mil caracteres, o bloco é dividido por faixas de páginas, cortando sempre entre matérias, e os resultados são concatenados pelo core.

**Etapa 4 — Verificação literal (core)**

- Conteúdo programático: o próprio `texto` de cada nó precisa aparecer, normalizado, no texto normalizado da página citada (ou na seguinte, para trechos que quebram página).
- Campos escalares: o `trecho` precisa aparecer da mesma forma, e o valor extraído precisa estar contido no trecho (número, data ou nome).
- Encontrado: `verificado`, com início e fim no texto original (pelo mapa de índices da normalização), para o destaque no PDF.
- Não encontrado: `nao_verificado`. O item não é descartado, mas vai destacado para revisão.

**Etapa 5 — Curadoria e tela de Revisão**

- Tela de Revisão: à esquerda, a árvore (dados gerais, cargos, cronograma, provas, conteúdo), com marca de verificado, não verificado e revisado; à direita, o PDF com o trecho destacado.
- O usuário edita qualquer campo (o item fica `editado`), confirma blocos (`revisado`) e marca o edital como `curado`.
- Reextrair um bloco é sempre manual. Se o bloco tem itens editados, a UI avisa que as edições serão perdidas antes de seguir. Prompt de versão nova não reextrai nada sozinho; a UI só indica que há versão mais nova.

**Persistência (migração 004)**

| Tabela | Conteúdo |
| --- | --- |
| `secao` | Mapa e triagem: edital, ordem, título, página inicial e final, categoria |
| `extracao` | Estado por edital e bloco (e cargo, quando por cargo): status, job, versão do papel, datas |
| `cargo` | Bloco `cargos`, mais `selecionado` |
| `vaga` | Por cargo e modalidade: quantidade |
| `evento` | Cronograma |
| `etapa_prova`, `prova_materia` | Bloco `provas`, por cargo |
| `materia`, `topico` | Bloco `conteudo`; `topico.pai` para a hierarquia; tabela de ligação `cargo_materia` para conteúdo comum |
| `fonte` | Proveniência de qualquer campo: entidade, id, campo, página, trecho, início, fim, `verificado`, `editado` |

Dados gerais ganham colunas em `edital` (número, data de publicação, taxa, período de inscrição, validade, links), também com proveniência em `fonte`.

**Fluxo na tela do Edital**

1. "Processar edital": mapa e triagem. Se mais de 20% das páginas tiverem problema, pede confirmação antes. O aviso de uso alto do Item 3 também vale.
2. Aparecem a lista de cargos (com caixas de seleção) e, assim que os blocos terminam, os dados gerais e o cronograma.
3. "Extrair cargos selecionados": enfileira `provas` e `conteudo` de cada cargo marcado.
4. "Revisar": abre a tela de Revisão.

**Erros novos**

| Código | Quando |
| --- | --- |
| `EDITAL_NAO_E_EDITAL` | A triagem diz que o PDF não é um edital (o motivo vai no `detail`) |
| `EXTRACAO_SEM_SECOES` | A triagem não achou seção para o bloco (ex.: edital sem conteúdo programático) |
| `EXTRACAO_INCOMPLETA` | O resultado veio, mas a verificação literal reprovou mais de 30% dos itens; o bloco fica disponível para revisão e o usuário decide se reextrai |

**Custo esperado (TCE-SP 2026, 47 páginas, ~150 mil caracteres)**

Triagem ~5 mil tokens de entrada; blocos gerais ~10 mil; `provas` ~10 mil; `conteudo` ~25 mil de entrada e ~10 mil de saída. Na primeira passada o total fica perto de mandar o edital inteiro uma vez. A economia vem de quatro fontes: reextrair um bloco não refaz os outros; os blocos simples usam o modelo rápido; editais com muitos cargos só detalham os escolhidos; e, depois da curadoria, o edital não volta ao Claude.
