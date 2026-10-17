# Edital Killer — Especificação do Passo 2

Oct 17, 2026 · @vbo

## Objetivo e escopo

Ao fim do Passo 2, o usuário importa o edital, escolhe o cargo e recebe três coisas: um raio-X da prova (onde está a nota), um diagnóstico do que já sabe, feito com questões no estilo da banca, e um plano de estudos até a data da prova com revisão espaçada.

**Entregas**

1. Análise automática e raio-X da prova: a análise começa na importação; estrutura da prova (blocos, questões, pesos, mínimos) e os insights derivados.
2. Gerador de questões: perfis de banca embutidos, átomos de conhecimento com fonte, construção por mutação controlada e verificação em camadas.
3. Diagnóstico adaptativo ("Verificar conhecimento"): domínio por subtópico com parada sequencial, certeza por resposta, resultado consolidado e reteste.
4. Plano de estudos: priorização por valor de prova, ciclo de conteúdo novo, revisão espaçada com FSRS ciente da data da prova e projeção de nota.

**Decisões já tomadas**

- Questões só de múltipla escolha (A–E), inclusive para bancas de certo/errado. Limitação conhecida: em editais do Cebraspe o diagnóstico mede conhecimento, mas não treina o formato certo/errado nem a estratégia de chute com penalização.
- Sem integração com OpenQuestões, QConcursos ou TEC. Nada de scraping.
- O estilo de cada banca vem de perfis embutidos no app, gerados em desenvolvimento a partir de provas inteiras. O usuário não precisa fornecer nada; uma prova de amostra é opcional.
- Legislação: texto atual do Planalto, baixado e guardado em cache. Vigência não é controlada (raramente é problema).
- Jurisprudência: só quando o edital citar súmulas específicas.
- Discursivas e estudos de caso ficam fora (feature futura). Os perfis de banca já registram o formato delas.
- Questões críticas (legislação, comandos "INCORRETA") passam por um resolvedor com modelo mais forte.

**Fora do escopo do Passo 2**

- Formato certo/errado, discursivas, correção de redação.
- Vários editais cruzados e matriz de sobreposição.
- Material de estudo (PDFs, videoaulas): o plano aponta o que estudar, não fornece o conteúdo teórico.
- Sincronização entre máquinas.

## Item 1 — Análise automática e raio-X da prova

**Fluxo**

1. A importação termina e o core já enfileira o localizador. A primeira tela depois da importação mostra "Lendo o edital…".
2. Os cargos aparecem uma única vez, na escolha. Com um cargo só, ele é escolhido sozinho e a extração começa direto.
3. Depois da escolha, a tela mostra só "Seu cargo: X · trocar". A lista completa não fica exposta.
4. O extrator lê, junto com o conteúdo programático, o **quadro de provas do cargo**.

**Estrutura da prova (por cargo)**

| Campo | Exemplo |
| --- | --- |
| Blocos | Conhecimentos Gerais, Conhecimentos Específicos |
| Matéria → bloco | Língua Portuguesa → Gerais |
| Questões por matéria | 15 |
| Peso por matéria | 1 (ou 2) |
| Mínimo por bloco | 50% dos pontos do bloco |
| Mínimo geral | 60% do total |
| Duração | 4 h 30 min |
| Questões discursivas | registradas, fora do cálculo objetivo |

Cada campo vem com `pagina` e `trecho` e passa pela verificação literal do Passo 1. Quadros de provas costumam virar texto fragmentado na extração do PDF, então um campo não verificado ainda entra nos cálculos, mas aparece marcado como "conferir no edital", junto com tudo o que deriva dele.

Quando o edital informa as questões por bloco e não por matéria (ex.: "Conhecimentos Gerais: 30 questões"), as questões do bloco são divididas igualmente entre as matérias dele e o valor aparece como estimado.

**Insights do raio-X**

| Insight | Cálculo |
| --- | --- |
| Valor de prova por matéria | questões × peso ÷ total de pontos |
| Concentração (Pareto) | menor conjunto de matérias que soma 50% e 80% da nota |
| Densidade por tópico | valor da matéria ÷ número de tópicos dela |
| Matéria-armadilha | matéria pequena num bloco com mínimo eliminatório |
| Ritmo de prova | duração ÷ questões objetivas |
| Leis a ler | leis citadas no conteúdo, com contagem de artigos (do cache do Planalto) |
| Contagem regressiva | dias até a prova; viabilidade fica para o Item 4 |

## Item 2 — Gerador de questões

O gabarito nunca depende de o LLM "acertar": a questão nasce de um fato verificado e o gabarito sai da construção. Camadas independentes conferem o resultado; só passa o que sobrevive a todas.

**Critérios de uma questão assertiva**

1. Correta: exatamente uma alternativa defensável.
2. Fiel ao subtópico pedido, não ao vizinho.
3. Com cara da banca (comando, extensão, pegadinhas).
4. No nível cognitivo pedido.
5. Diagnóstica: cada distrator representa um erro identificável.

**Perfis de banca embutidos**

- Um perfil por banca × área: Gerais/Português, Jurídica, Específicas técnicas. Bancas iniciais: VUNESP, FGV, Cebraspe, FCC, Cesgranrio, IBFC, Quadrix, AOCP, IDECAN, Consulplan.
- Gerados em desenvolvimento pela ferramenta `tools/perfil-banca`, a partir de 2 a 4 provas inteiras e recentes (2023 em diante) por banca e área:
  1. Separar questões, alternativas e gabarito.
  2. Medir de forma determinística: tamanho do enunciado e das alternativas, tipo de comando (correta, incorreta, afirmativas I/II/III, associação), texto-base, distribuição das letras do gabarito.
  3. Anotar com o LLM: nível cognitivo, tipo de mutação de cada distrator, literalidade ou aplicação.
  4. Agregar numa impressão digital (JSON).
  5. Escrever o guia de estilo amarrado aos números.
  6. Escrever de 6 a 8 exemplos originais no estilo e validá-los contra a impressão digital.
  7. Revisão humana e commit, com a data e a lista das provas de origem.
- O app distribui só o que é derivado: impressão digital, guia e exemplos originais. As provas de origem não entram no repositório.
- **Teste de qualidade do perfil:** questões reais guardadas para avaliação são misturadas com questões geradas; um juiz LLM (e o autor, numa amostra) tenta separá-las. Meta: perto do acaso.
- **Banca fora da lista:** usa o perfil mais próximo e oferece, com aprovação, a busca de uma prova pública de amostra para gerar um perfil local.
- **Amostra opcional:** "Tem uma prova antiga desta banca? Arraste aqui." Ajusta o perfil daquele concurso por cima do embutido.
- Perfis têm data e são regerados uma vez por ano.

**Pipeline por subtópico**

1. **Átomos de conhecimento.** De 8 a 15 fatos atômicos com fonte, reaproveitados em todas as questões e retestes.
   - Lei: artigo do texto do Planalto (cache local), conferido pela verificação literal.
   - Súmula: só as citadas no edital; o texto da súmula é o átomo.
   - Norma ou framework (ITIL, PMBOK, ISO): cita a seção; sem fonte local, passa pelo revisor com mais rigor.
   - Português: regra com exemplo e contraexemplo.
2. **Construção.** A correta é um átomo parafraseado no estilo da banca. Cada distrator é o átomo com uma mutação do catálogo:

   | Tipo | Mutações |
   | --- | --- |
   | Lei | prazo ou número trocado, regra virando exceção, competência trocada ("compete" / "poderá"), quantificador (sempre, em regra, vedado), sujeito trocado |
   | Técnica | conceito vizinho, propriedade de outro item, ordem de etapas invertida |
   | Português | regra aplicada no caso da exceção, hipercorreção, confusão de regência |

   Cada alternativa errada guarda a etiqueta da mutação: é o que alimenta a taxonomia de erros do diagnóstico.
3. **Nível cognitivo como parâmetro:** lembrar (literalidade), compreender (paráfrase), aplicar (caso concreto), analisar (afirmativas, combinações). A mistura segue a impressão digital da banca; a sutileza da mutação é o segundo controle de dificuldade.
4. **Português com texto-base:** gera-se um texto original curto com as armadilhas plantadas; as questões apontam para trechos do texto, conferidos pela verificação literal.
5. **Checagens determinísticas (core, sem LLM):** 5 alternativas distintas; a correta não pode ser sistematicamente a mais longa; sem "todas" ou "nenhuma" se a banca não usa; letras do gabarito uniformes no lote; citação de lei confere literalmente; similaridade baixa com questões já vistas.
6. **Resolvedor independente:** contexto limpo, sem gabarito nem átomos; resolve e responde se há outra alternativa defensável. Questões críticas usam dois resolvedores, um deles com modelo mais forte. Divergência: um reparo; falhou de novo, descarta.
7. **Comentário por alternativa** com a fonte. Serve de checagem (expõe questões quebradas) e de material de estudo.
8. **Fidelidade ao tópico:** um classificador barato confirma que a questão cobra o subtópico pedido.

**Melhoria contínua**

- "Questão com problema" num clique: a questão sai do banco e o defeito entra nas instruções do revisor.
- Erro com certeza num tópico dominado manda a questão para rechecagem automática.
- Avaliação offline: lote de 50 questões por banca, amostra de 20 revisada à mão, precisão medida. Questões reais guardadas avaliam também o resolvedor.

**Custo e latência**

- Átomos uma vez por subtópico; questões em lotes de 5 por chamada; resolvedor em lote.
- Geração adiantada na fila de jobs, com prioridade: o que o usuário vai responder agora, depois o pré-carregamento por valor de prova.

## Item 3 — Diagnóstico adaptativo

**Modelo**

- Domínio por subtópico como Beta(α, β), com prior compartilhado da matéria (pooling parcial).
- Começa com 2 questões por subtópico. Para quando a incerteza fica abaixo do limiar; no máximo 5 por subtópico.
- Autoavaliação prévia por matéria ("nunca estudei", "vi por cima", "domino") vira o prior. "Nunca estudei" já conta como lacuna e não gasta questões.
- Cada resposta tem certeza: "certeza", "dúvida" ou "chute". Acerto no chute conta como lacuna; erro com certeza é sinal de conceito errado consolidado.

**Sessões**

- Blocos de 20 a 30 questões, retomáveis, na ordem do valor de prova.
- Diagnóstico mínimo viável depois de 3 ou 4 sessões: o plano pode começar e se refina com o resto.

**Resultado consolidado**

- Mapa de calor matéria × subtópico.
- Lacuna ponderada = (1 − domínio) × valor do tópico, que ordena as prioridades do plano.
- Taxonomia de erros pelas etiquetas dos distratores: conceito, literalidade, exceção, interpretação.
- Tempo médio por questão contra o ritmo da prova.
- Calibração: quanto as certezas declaradas acertam de fato.

**Reteste:** usa questões ainda não vistas, gerando mais se acabarem, e mostra a evolução desde o diagnóstico anterior.

## Item 4 — Plano de estudos

**Entradas (com padrões razoáveis):** data da prova (do edital) e data de início; horas por dia da semana; dias de folga; duração do bloco (padrão 50 + 10 min); matérias por dia; margem para imprevistos (10 a 15%).

**Motor**

- Esforço por subtópico: estimativa inicial do LLM (complexidade × volume), corrigida pelo tempo real do usuário.
- Prioridade = valor do tópico × (meta − domínio) ÷ esforço.
- Mochila gulosa sobre as horas disponíveis, com **abandono estratégico declarado**: "com 280 h, cubra estes 85% do valor de prova; estes 9 tópicos ficam de fora de propósito".
- Três fases, planejadas de trás para frente a partir da prova: aprendizado (teoria + questões), consolidação (revisão + questões) e reta final (últimas 2 a 3 semanas: simulados, lei seca, nada novo).
- Conteúdo novo gira num ciclo por horas; revisões ficam no calendário. Dia perdido recalcula o plano, que nunca fica "atrasado".
- Intercalação: 2 ou 3 matérias por dia, alternando pesada e leve.

**Revisão espaçada (FSRS)**

- Implementação oficial em Rust (crate `fsrs`), no core.
- Cada subtópico é um cartão. A revisão é um minibloco de 3 a 5 questões; o desempenho vira a nota do cartão (menos de 40% → "errei"; 100% com certeza → "fácil").
- Ciente da data da prova: meta de retenção ≥ 90% no dia da prova; intervalos encolhem perto da data; todo tópico com valor recebe uma última revisão na janela final.
- Revisões ocupam no máximo uns 30% do dia, com picos espalhados.
- Lei seca em trilha própria: leituras repetidas com contagem de artigos e questões de literalidade.

**Painel**

- Cobertura ponderada (quanto do valor de prova já foi estudado).
- Retenção estimada por matéria, hoje e no dia da prova.
- Projeção de nota por Monte Carlo e probabilidade de passar cada nota de corte.
- Exportação para calendário (`.ics`) e Anki/AnkiFactor.

## Modelo de dados (novo)

| Tabela | Conteúdo |
| --- | --- |
| `estrutura_prova` | por cargo: bloco, matéria, questões, peso, mínimo, duração |
| `lei` | texto do Planalto em cache, por lei e artigo |
| `atomo` | subtópico, fato, fonte, trecho verificado |
| `questao` | origem, banca, subtópico, nível, enunciado, alternativas, gabarito, etiquetas dos distratores, comentários, hash, status |
| `resposta` | questão, quando, escolha, acerto, certeza, tempo |
| `dominio` | subtópico, α, β |
| `cartao_fsrs` | subtópico, estabilidade, dificuldade, próxima revisão |
| `plano`, `sessao` | parâmetros do plano; itens de cada dia |

Os algoritmos (Beta, parada sequencial, FSRS, mochila, agendador, checagens das questões) ficam no core em Rust, puros e testáveis, como a verificação literal. A fila de jobs do Passo 1 ganha prioridade.

## Ordem de execução

1. Item 1: análise automática, cargo uma vez, estrutura da prova e raio-X.
2. Perfis de banca (`tools/perfil-banca`) e cache do Planalto.
3. Gerador de questões com as camadas de verificação e a avaliação offline.
4. Diagnóstico adaptativo.
5. Plano com FSRS e projeção de nota.
