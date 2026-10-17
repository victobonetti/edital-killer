import { useCallback, useEffect, useState } from "react";
import type { Analise } from "../bindings/Analise";
import type { AppError } from "../bindings/AppError";
import type { BackendStatus } from "../bindings/BackendStatus";
import type { Cargo } from "../bindings/Cargo";
import type { Edital } from "../bindings/Edital";
import type { Materia } from "../bindings/Materia";
import type { Topico } from "../bindings/Topico";
import { ErrorBox } from "../components/ErrorBox";
import { Icone } from "../components/Icone";
import { PdfViewer } from "../components/pdf/PdfViewer";
import { ipc, onAnalise, toAppError } from "../ipc";
import { CATALOGO } from "../ipc/errors";
import { consolidar, contarTopicos, type Consolidado, type Etapa } from "../model/consolidado";
import { dataCurta, dataLonga, duracao, faixas, plural } from "../model/formato";
import { raioX, type RaioX } from "../model/raioX";

type Aba = "visao" | "documento";

export function EditalScreen({
  id,
  aviso,
  claude,
  onVoltar,
  onAjustes,
}: {
  id: string;
  aviso?: string;
  claude: BackendStatus;
  onVoltar: () => void;
  onAjustes: () => void;
}) {
  const [edital, setEdital] = useState<Edital | null>(null);
  const [analise, setAnalise] = useState<Analise | null>(null);
  const [erro, setErro] = useState<AppError | null>(null);
  const [mostrarAviso, setMostrarAviso] = useState(!!aviso);
  const [aba, setAba] = useState<Aba>("visao");
  const [pagina, setPagina] = useState<{ n: number } | null>(null);
  const [trocandoCargo, setTrocandoCargo] = useState(false);

  const carregar = useCallback(() => {
    ipc.editalObter(id).then(setEdital, (e) => setErro(toAppError(e)));
    ipc.editalAnalise(id).then(setAnalise, () => setAnalise(null));
  }, [id]);

  useEffect(() => {
    carregar();
    const desliga = onAnalise((editalId) => editalId === id && carregar());
    return () => {
      void desliga.then((f) => f());
    };
  }, [id, carregar]);

  function irParaPagina(n: number) {
    setPagina({ n });
    setAba("documento");
  }

  async function acao(f: () => Promise<unknown>) {
    setErro(null);
    try {
      await f();
    } catch (e) {
      setErro(toAppError(e));
    }
    carregar();
  }

  async function remover() {
    if (!edital) return;
    const ok = window.confirm(
      `Remover “${edital.nome}”? O PDF guardado e tudo o que foi extraído dele serão apagados deste computador. Não dá para desfazer.`,
    );
    if (!ok) return;
    try {
      await ipc.editalRemover(edital.id);
      onVoltar();
    } catch (e) {
      setErro(toAppError(e));
    }
  }

  if (!edital) {
    return (
      <main className="tela">
        <button className="voltar" onClick={onVoltar}>
          <Icone nome="voltar" /> Editais
        </button>
        {erro ? <ErrorBox erro={erro} /> : <p className="mudo carregando">Abrindo o edital…</p>}
      </main>
    );
  }

  const visao = consolidar(edital, analise);
  const rx = visao.materias ? raioX(visao.materias, analise?.estrutura ?? null) : null;
  const escolhendo = visao.etapa === "escolher_cargo" || (trocandoCargo && !!visao.cargos && !visao.rodando);

  return (
    <main className={`tela edital aba-${aba}`}>
      <div className="edital-cabeca">
        <button className="voltar" onClick={onVoltar}>
          <Icone nome="voltar" /> Editais
        </button>
        <Cabecalho edital={edital} onSalvo={setEdital} onErro={setErro} />
        <div className="abas" role="tablist">
          <button role="tab" aria-selected={aba === "visao"} onClick={() => setAba("visao")}>
            <Icone nome="visao" /> Visão geral
          </button>
          <button role="tab" aria-selected={aba === "documento"} onClick={() => setAba("documento")}>
            <Icone nome="documento" /> Documento original
          </button>
        </div>
      </div>

      {mostrarAviso && aviso && (
        <div className="nota">
          <span>{aviso}</span>
          <button className="icone-btn" onClick={() => setMostrarAviso(false)} aria-label="Fechar">
            <Icone nome="fechar" />
          </button>
        </div>
      )}
      {erro && <ErrorBox erro={erro} onFechar={() => setErro(null)} />}

      {aba === "documento" ? (
        <section className="edital-pdf">
          <PdfViewer editalId={edital.id} pagina={pagina} />
        </section>
      ) : (
        <div className="visao">
          <Jornada etapa={visao.etapa} />
          {escolhendo ? (
            <EscolherCargo
              visao={visao}
              onEscolher={(c) => {
                setTrocandoCargo(false);
                void acao(() => ipc.cargoEscolher(edital.id, c));
              }}
              onCancelar={trocandoCargo ? () => setTrocandoCargo(false) : undefined}
            />
          ) : (
            <ProximoPasso
              visao={visao}
              claude={claude}
              onAjustes={onAjustes}
              onAnalisar={() => acao(() => ipc.analiseIniciar(edital.id))}
              onExtrair={() => visao.cargo && acao(() => ipc.cargoEscolher(edital.id, visao.cargo!.id))}
              onCancelar={() => acao(() => ipc.analiseCancelar(edital.id))}
            />
          )}
          {visao.cargo && !escolhendo && (
            <p className="seu-cargo">
              <span className="kicker">Seu cargo</span>
              <strong>{visao.cargo.nome}</strong>
              {visao.cargos!.length > 1 && !visao.rodando && (
                <button className="link" onClick={() => setTrocandoCargo(true)}>
                  Trocar
                </button>
              )}
            </p>
          )}
          <Numeros edital={edital} visao={visao} rx={rx} />

          {rx && <RaioXProva rx={rx} />}

          <section className="bloco">
            <header className="bloco-cabeca">
              <h2 className="secao">
                <span className="secao-num">§</span> Conteúdo programático
              </h2>
              {visao.cargo && <p className="mudo">Para {visao.cargo.nome}</p>}
            </header>
            {visao.materias ? (
              <>
                {visao.topicosConferir > 0 && (
                  <p className="nota discreta">
                    <Icone nome="alerta" />
                    <span>
                      {plural(visao.topicosConferir, "tópico não foi encontrado", "tópicos não foram encontrados")}{" "}
                      exatamente como no texto do edital e {visao.topicosConferir === 1 ? "está marcado" : "estão marcados"}{" "}
                      com <em>conferir</em>: clique na página para ver no documento original.
                    </span>
                  </p>
                )}
                <ConteudoProgramatico materias={visao.materias} rx={rx} onPagina={irParaPagina} />
              </>
            ) : (
              <ConteudoVazio etapa={visao.etapa} />
            )}
          </section>

          {edital.status !== "sem_texto" && edital.paginas_sem_texto.length > 0 && (
            <p className="nota discreta">
              <Icone nome="alerta" />
              <span>
                {edital.paginas_sem_texto.length === 1 ? "A página " : "As páginas "}
                {faixas(edital.paginas_sem_texto)}
                {edital.paginas_sem_texto.length === 1 ? " é uma imagem e não será lida" : " são imagens e não serão lidas"}
                . Confira no documento original se há algo importante nelas.
              </span>
            </p>
          )}

          <details className="sobre">
            <summary>
              <Icone nome="chevron" /> Sobre o arquivo
            </summary>
            <dl className="meta">
              <dt>Arquivo importado</dt>
              <dd>{edital.arquivo_original}</dd>
              <dt>Páginas</dt>
              <dd>{edital.total_paginas ?? "—"}</dd>
              <dt>Importado em</dt>
              <dd>{new Date(edital.importado_em).toLocaleString("pt-BR")}</dd>
              <dt>Impressão digital</dt>
              <dd className="hash" title="SHA-256: identifica este PDF exato">
                {edital.sha256}
              </dd>
            </dl>
            <div className="acoes">
              {visao.cargos && !visao.rodando && (
                <button
                  onClick={() =>
                    window.confirm(
                      "Analisar o edital de novo? Cargos e conteúdo atuais serão substituídos pelo resultado da nova análise.",
                    ) && acao(() => ipc.analiseIniciar(edital.id))
                  }
                >
                  Analisar de novo
                </button>
              )}
              <button className="perigo" onClick={remover}>
                <Icone nome="lixeira" /> Remover edital
              </button>
            </div>
          </details>
        </div>
      )}
    </main>
  );
}

const PASSOS: { rotulo: string; etapas: Etapa[] }[] = [
  { rotulo: "Edital importado", etapas: ["importando", "sem_texto"] },
  { rotulo: "Cargo escolhido", etapas: ["localizar_cargos", "escolher_cargo"] },
  { rotulo: "Conteúdo consolidado", etapas: ["extrair_conteudo"] },
  { rotulo: "Plano de estudos", etapas: ["pronto"] },
];

function Jornada({ etapa }: { etapa: Etapa }) {
  const atual = PASSOS.findIndex((p) => p.etapas.includes(etapa));
  return (
    <ol className="jornada" aria-label="Progresso do edital">
      {PASSOS.map((p, i) => {
        const estado = i < atual ? "feito" : i === atual ? "atual" : "depois";
        return (
          <li key={p.rotulo} className={estado} aria-current={estado === "atual" ? "step" : undefined}>
            <span className="jornada-marca">{estado === "feito" ? <Icone nome="check" /> : i + 1}</span>
            <span className="jornada-rotulo">
              {p.rotulo}
              {i === PASSOS.length - 1 && <span className="em-breve">em breve</span>}
            </span>
          </li>
        );
      })}
    </ol>
  );
}

/** Segundos desde `iso`, atualizando a cada segundo. */
function useDecorrido(iso: string | null): number {
  const [agora, setAgora] = useState(() => Date.now());
  useEffect(() => {
    if (!iso) return;
    const t = setInterval(() => setAgora(Date.now()), 1000);
    return () => clearInterval(t);
  }, [iso]);
  return iso ? (agora - new Date(iso).getTime()) / 1000 : 0;
}

/** Uma única chamada para a ação, com a explicação do porquê. Nada de jobs, filas ou modelos. */
function ProximoPasso({
  visao,
  claude,
  onAjustes,
  onAnalisar,
  onExtrair,
  onCancelar,
}: {
  visao: Consolidado;
  claude: BackendStatus;
  onAjustes: () => void;
  onAnalisar: () => void;
  onExtrair: () => void;
  onCancelar: () => void;
}) {
  const decorrido = useDecorrido(visao.rodando?.status === "executando" ? visao.rodando.desde : null);

  if (visao.rodando) {
    const localizando = visao.rodando.etapa === "localizar";
    const naFila = visao.rodando.status === "na_fila";
    return (
      <section className="proximo rodando" aria-live="polite">
        <p className="kicker">{naFila ? "Na fila" : "Trabalhando"}</p>
        <h2 className="proximo-titulo">
          {localizando ? "Lendo o edital…" : `Montando o conteúdo de ${visao.cargo?.nome ?? "seu cargo"}…`}
        </h2>
        <p className="proximo-texto">
          {localizando
            ? "O Claude está identificando o concurso, as datas e os cargos oferecidos."
            : "O Claude está separando as matérias e os tópicos do seu cargo e o app confere cada um contra o PDF."}{" "}
          Costuma levar de 1 a 5 minutos. Você pode sair desta tela; a análise continua.
        </p>
        <div className="progresso-indeterminado" aria-hidden>
          <span />
        </div>
        <div className="acoes">
          <button onClick={onCancelar}>Cancelar</button>
          {!naFila && <span className="mudo">{duracao(decorrido)}</span>}
        </div>
      </section>
    );
  }

  const semClaude = claude.estado === "indisponivel";
  const conteudo: Record<Etapa, { titulo: string; texto: string; acao?: string; onAcao?: () => void }> = {
    importando: {
      titulo: "A importação não terminou",
      texto: "O app foi fechado enquanto lia o PDF. Volte para a biblioteca e abra o edital para continuar de onde parou.",
    },
    sem_texto: {
      titulo: CATALOGO.EDITAL_SEM_TEXTO.titulo,
      texto: `${CATALOGO.EDITAL_SEM_TEXTO.explicacao} ${CATALOGO.EDITAL_SEM_TEXTO.acao ?? ""}`,
    },
    localizar_cargos: {
      titulo: "Descubra os cargos deste edital",
      texto:
        "O Claude lê o edital inteiro, identifica o concurso, as datas e os cargos oferecidos. Depois você escolhe o seu e ele monta o conteúdo programático, com cada tópico ligado à página de onde veio.",
      acao: "Analisar edital",
      onAcao: onAnalisar,
    },
    escolher_cargo: { titulo: "", texto: "" },
    extrair_conteudo: {
      titulo: "Monte o conteúdo programático",
      texto: "Com o cargo definido, o Claude separa as matérias e os tópicos que caem na sua prova.",
      acao: "Extrair conteúdo",
      onAcao: onExtrair,
    },
    pronto: {
      titulo: "Seu edital está consolidado",
      texto:
        "Revise as matérias abaixo. O próximo passo é transformar este conteúdo em um plano de estudos sob medida para a sua data de prova.",
      acao: "Criar plano de estudos",
    },
  };
  const c = conteudo[visao.etapa];
  const bloqueado = visao.etapa === "importando" || visao.etapa === "sem_texto";
  const falha = visao.falha;

  return (
    <section className={`proximo ${bloqueado ? "bloqueado" : ""}`}>
      <p className="kicker">{bloqueado ? "Atenção" : "Próximo passo"}</p>
      <h2 className="proximo-titulo">{c.titulo}</h2>
      <p className="proximo-texto">{c.texto}</p>
      {falha &&
        (falha.erro ? (
          <ErrorBox erro={falha.erro} contexto="A última análise não terminou" />
        ) : (
          <p className="nota">
            <span>A última análise foi interrompida porque o app foi fechado no meio dela.</span>
          </p>
        ))}
      {c.acao && (
        <div className="acoes">
          {semClaude && c.onAcao ? (
            <>
              <button className="primario" onClick={onAjustes}>
                Conectar o Claude <Icone nome="seta" />
              </button>
              <span className="mudo">A análise usa o Claude da sua conta. Ele ainda não está conectado.</span>
            </>
          ) : c.onAcao ? (
            <button className="primario" onClick={c.onAcao} disabled={claude.estado === "limite_uso"}>
              {falha ? "Tentar de novo" : c.acao} <Icone nome="seta" />
            </button>
          ) : (
            <>
              {/* Habilitado quando o planejamento chegar. */}
              <button className="primario" disabled>
                {c.acao} <Icone nome="seta" />
              </button>
              <span className="mudo">Disponível na próxima versão.</span>
            </>
          )}
          {claude.estado === "limite_uso" && c.onAcao && (
            <span className="mudo">
              Claude em pausa pelo limite de uso
              {claude.retorno_em &&
                `; volta às ${new Date(claude.retorno_em).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" })}`}
              .
            </span>
          )}
        </div>
      )}
    </section>
  );
}

function EscolherCargo({
  visao,
  onEscolher,
  onCancelar,
}: {
  visao: Consolidado;
  onEscolher: (cargoId: string) => void;
  onCancelar?: () => void;
}) {
  const [escolha, setEscolha] = useState<string | null>(visao.cargo?.id ?? null);
  const cargos = visao.cargos ?? [];
  return (
    <section className="proximo">
      <p className="kicker">Próximo passo</p>
      <h2 className="proximo-titulo">Qual destes cargos é o seu?</h2>
      <p className="proximo-texto">
        {plural(cargos.length, "cargo encontrado", "cargos encontrados")} no edital. Escolha o seu: o conteúdo
        programático é montado só para ele.
      </p>
      <fieldset className="escolha-cargos">
        <legend className="sr">Cargos do edital</legend>
        {cargos.map((c) => (
          <label key={c.id} className={escolha === c.id ? "marcado" : ""}>
            <input
              type="radio"
              name="cargo"
              value={c.id}
              checked={escolha === c.id}
              onChange={() => setEscolha(c.id)}
            />
            <span className="cargo-nome">{c.nome}</span>
            <span className="mudo">{detalheCargo(c)}</span>
          </label>
        ))}
      </fieldset>
      <div className="acoes">
        <button className="primario" disabled={!escolha} onClick={() => escolha && onEscolher(escolha)}>
          Este é o meu cargo <Icone nome="seta" />
        </button>
        {onCancelar && <button onClick={onCancelar}>Manter o atual</button>}
      </div>
    </section>
  );
}

function detalheCargo(c: Cargo): string {
  return [c.escolaridade, c.vagas !== null && plural(c.vagas, "vaga", "vagas"), c.pagina && `p. ${c.pagina}`]
    .filter(Boolean)
    .join(" · ");
}

function Numeros({ edital, visao, rx }: { edital: Edital; visao: Consolidado; rx: RaioX | null }) {
  const dias = edital.data_prova ? diasAte(edital.data_prova) : null;
  const itens: { valor: number | null; rotulo: string }[] = [
    rx ? { valor: Math.round(rx.totalQuestoes), rotulo: "questões" } : { valor: edital.total_paginas, rotulo: "páginas" },
    { valor: visao.materias?.length ?? null, rotulo: "matérias" },
    { valor: visao.totalTopicos, rotulo: "tópicos" },
    { valor: dias !== null && dias >= 0 ? dias : null, rotulo: dias === 1 ? "dia até a prova" : "dias até a prova" },
  ];
  return (
    <dl className="numeros">
      {itens.map((it) => (
        <div key={it.rotulo} className={it.valor === null ? "pendente" : ""}>
          <dt>{it.rotulo}</dt>
          <dd>{it.valor === null ? "—" : it.valor.toLocaleString("pt-BR")}</dd>
        </div>
      ))}
    </dl>
  );
}

function diasAte(ymd: string): number {
  const [a, m, d] = ymd.split("-").map(Number);
  const hoje = new Date();
  const inicio = new Date(hoje.getFullYear(), hoje.getMonth(), hoje.getDate());
  return Math.round((new Date(a, m - 1, d).getTime() - inicio.getTime()) / 86_400_000);
}

const pct = (f: number) => `${Math.round(f * 100)}%`;
const num = (n: number) => n.toLocaleString("pt-BR", { maximumFractionDigits: 1 });

/** Onde está a nota: valor de cada matéria, concentração, densidade, ritmo e mínimos. */
function RaioXProva({ rx }: { rx: RaioX }) {
  const comPontos = rx.materias.filter((m) => m.fracao !== null);
  const maior = Math.max(...comPontos.map((m) => m.fracao!));
  const densas = comPontos.filter((m) => m.porTopico !== null).sort((a, b) => b.porTopico! - a.porTopico!);
  const [maisDensa, menosDensa] = [densas[0], densas[densas.length - 1]];
  const algumConferir = comPontos.some((m) => m.conferir || m.estimado);

  const insights: { titulo: string; texto: string; conferir?: boolean }[] = [
    {
      titulo: `${rx.pareto50 === 1 ? "1 matéria" : `${rx.pareto50} matérias`} = metade da nota`,
      texto: `E ${rx.pareto80} de ${comPontos.length} somam 80%. Comece por elas.`,
    },
  ];
  if (maisDensa && menosDensa && maisDensa !== menosDensa && menosDensa.porTopico! > 0) {
    const vezes = maisDensa.porTopico! / menosDensa.porTopico!;
    insights.push({
      titulo: `Tópico de ${maisDensa.nome} vale ${num(maisDensa.porTopico!)} pt`,
      texto:
        vezes >= 1.5
          ? `${num(vezes)}× um tópico de ${menosDensa.nome}. Cada hora ali rende mais nota.`
          : "A nota está bem distribuída entre os tópicos das matérias.",
    });
  }
  if (rx.ritmo) {
    insights.push({
      titulo: `${duracao(rx.ritmo)} por questão`,
      texto: "É o seu ritmo de prova, sem contar o tempo de passar o gabarito.",
      conferir: rx.ritmoConferir,
    });
  }
  for (const b of rx.blocos) {
    insights.push({
      titulo: `${b.nome}: mínimo de ${num(b.pontosMinimos)} pt`,
      texto: `${b.minimo}% dos ${num(b.pontos)} pontos do bloco. Abaixo disso, eliminado.`,
      conferir: b.conferir,
    });
  }
  if (rx.minimoGeral) {
    insights.push({
      titulo: `Mínimo geral: ${num(rx.minimoGeral.pontosMinimos)} de ${num(rx.totalPontos)} pt`,
      texto: `${rx.minimoGeral.minimo}% do total da prova objetiva.`,
      conferir: rx.minimoGeral.conferir,
    });
  }

  return (
    <section className="bloco raio-x">
      <header className="bloco-cabeca">
        <h2 className="secao">
          <span className="secao-num">§</span> Raio-X da prova
        </h2>
        <p className="mudo">
          {num(rx.totalQuestoes)} questões · {num(rx.totalPontos)} pontos
        </p>
      </header>

      <div className="raio-x-corpo">
        <figure className="valor-materias">
          <figcaption className="mudo">Quanto cada matéria vale da nota</figcaption>
          <ol>
            {rx.materias.map((m) => (
              <li key={m.id} tabIndex={0}>
                <span className="vm-nome">{m.nome}</span>
                <span className="vm-trilho">
                  {m.fracao !== null && <span className="vm-barra" style={{ width: `${(m.fracao / maior) * 100}%` }} />}
                </span>
                <span className="vm-valor">
                  {m.fracao !== null ? pct(m.fracao) : "—"}
                  {(m.conferir || m.estimado) && m.fracao !== null && <span className="asterisco">*</span>}
                </span>
                <span className="vm-dica" role="tooltip">
                  <strong>{m.nome}</strong>
                  {m.questoes !== null ? (
                    <>
                      {num(m.questoes)} questões{m.estimado && " (estimado)"} × peso {num(m.peso)} = {num(m.pontos!)} pt
                      <br />
                      {pct(m.fracao!)} da nota · {plural(m.topicos, "tópico", "tópicos")}
                      {m.porTopico !== null && ` · ${num(m.porTopico)} pt por tópico`}
                      {m.bloco && (
                        <>
                          <br />
                          {m.bloco}
                        </>
                      )}
                    </>
                  ) : (
                    "O edital não informa as questões desta matéria."
                  )}
                </span>
              </li>
            ))}
          </ol>
          {algumConferir && (
            <p className="mudo nota-rodape">
              * Questões estimadas a partir do total do bloco, ou quadro não confirmado no texto do edital.
            </p>
          )}
        </figure>

        <ul className="insights">
          {insights.map((i) => (
            <li key={i.titulo}>
              <strong>{i.titulo}</strong>
              <span>{i.texto}</span>
              {i.conferir && <span className="conferir">conferir no edital</span>}
            </li>
          ))}
          {rx.leis.length > 0 && (
            <li>
              <strong>{plural(rx.leis.length, "lei citada", "leis citadas")} no conteúdo</strong>
              <span className="leis">
                {rx.leis.map((l) => (
                  <code key={l}>{l}</code>
                ))}
              </span>
            </li>
          )}
        </ul>
      </div>
    </section>
  );
}

function ConteudoProgramatico({
  materias,
  rx,
  onPagina,
}: {
  materias: Materia[];
  rx: RaioX | null;
  onPagina: (n: number) => void;
}) {
  const [abertas, setAbertas] = useState<Set<string>>(() => new Set(materias.slice(0, 1).map((m) => m.id)));
  const alternar = (id: string) =>
    setAbertas((s) => {
      const n = new Set(s);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });

  return (
    <ol className="materias">
      {materias.map((m, i) => {
        const aberta = abertas.has(m.id);
        const v = rx?.materias.find((x) => x.id === m.id);
        return (
          <li key={m.id} className={aberta ? "aberta" : ""}>
            <button className="materia-cabeca" onClick={() => alternar(m.id)} aria-expanded={aberta}>
              <span className="materia-num">{String(i + 1).padStart(2, "0")}</span>
              <span className="materia-nome">{m.nome}</span>
              <span className="mudo">
                {plural(contarTopicos(m.topicos), "tópico", "tópicos")}
                {v?.fracao != null && ` · ${pct(v.fracao)} da nota`}
              </span>
              <Icone nome="chevron" className="materia-seta" />
            </button>
            {aberta && <Topicos topicos={m.topicos} onPagina={onPagina} />}
          </li>
        );
      })}
    </ol>
  );
}

function Topicos({ topicos, onPagina }: { topicos: Topico[]; onPagina: (n: number) => void }) {
  return (
    <ul className="topicos">
      {topicos.map((t) => (
        <li key={t.id}>
          <div className="topico">
            <span>{t.texto}</span>
            <span className="topico-ref">
              {!t.verificado && (
                <span className="conferir" title="Não encontrado exatamente assim no texto do edital">
                  conferir
                </span>
              )}
              {t.pagina !== null && (
                <button className="pag" onClick={() => onPagina(t.pagina!)} title="Ver no documento original">
                  p. {t.pagina}
                </button>
              )}
            </span>
          </div>
          {t.filhos.length > 0 && <Topicos topicos={t.filhos} onPagina={onPagina} />}
        </li>
      ))}
    </ul>
  );
}

function ConteudoVazio({ etapa }: { etapa: Etapa }) {
  const texto =
    etapa === "sem_texto"
      ? "Sem texto no PDF, não há conteúdo para organizar."
      : "As matérias e os tópicos do seu cargo aparecem aqui, organizados e com a página de origem de cada um.";
  return (
    <div className="conteudo-vazio">
      <div className="esqueleto" aria-hidden>
        {[0.62, 0.48, 0.71].map((w, i) => (
          <div key={i} className="esqueleto-materia">
            <span className="esqueleto-num">{String(i + 1).padStart(2, "0")}</span>
            <span className="esqueleto-linha" style={{ width: `${w * 100}%` }} />
          </div>
        ))}
      </div>
      <p className="mudo">{texto}</p>
    </div>
  );
}

function Fatos({ edital }: { edital: Edital }) {
  const fatos = [
    edital.numero_edital && `Edital nº ${edital.numero_edital}`,
    edital.data_prova && `Prova em ${dataLonga(edital.data_prova)}`,
    edital.inscricoes_fim &&
      (edital.inscricoes_inicio
        ? `Inscrições de ${dataCurta(edital.inscricoes_inicio)} a ${dataCurta(edital.inscricoes_fim)}`
        : `Inscrições até ${dataCurta(edital.inscricoes_fim)}`),
  ].filter(Boolean);
  if (fatos.length === 0) return null;
  return (
    <ul className="fatos">
      {fatos.map((f) => (
        <li key={f as string}>{f}</li>
      ))}
    </ul>
  );
}

function Cabecalho({
  edital,
  onSalvo,
  onErro,
}: {
  edital: Edital;
  onSalvo: (e: Edital) => void;
  onErro: (e: AppError) => void;
}) {
  const [editando, setEditando] = useState(false);
  const [nome, setNome] = useState(edital.nome);
  const [orgao, setOrgao] = useState(edital.orgao ?? "");
  const [banca, setBanca] = useState(edital.banca ?? "");
  const [numero, setNumero] = useState(edital.numero_edital ?? "");
  const [dataProva, setDataProva] = useState(edital.data_prova ?? "");

  function abrir() {
    setNome(edital.nome);
    setOrgao(edital.orgao ?? "");
    setBanca(edital.banca ?? "");
    setNumero(edital.numero_edital ?? "");
    setDataProva(edital.data_prova ?? "");
    setEditando(true);
  }

  async function salvar(ev: React.FormEvent) {
    ev.preventDefault();
    try {
      onSalvo(
        await ipc.editalAtualizar(edital.id, {
          nome,
          orgao: orgao || null,
          banca: banca || null,
          numero_edital: numero || null,
          data_prova: dataProva || null,
        }),
      );
      setEditando(false);
    } catch (e) {
      onErro(toAppError(e));
    }
  }

  if (!editando) {
    const linha = [edital.orgao, edital.banca].filter(Boolean).join(" · ");
    return (
      <header className="cabecalho">
        <p className="kicker">
          {linha || (
            <button className="link" onClick={abrir}>
              Informe o órgão e a banca
            </button>
          )}
        </p>
        <h1 className="titulo-display">
          {edital.nome}
          <button className="icone-btn editar" onClick={abrir} aria-label="Editar dados do edital" title="Editar">
            <Icone nome="lapis" />
          </button>
        </h1>
        <Fatos edital={edital} />
      </header>
    );
  }
  return (
    <form className="cabecalho form-linha" onSubmit={salvar}>
      <label className="largo">
        Nome do edital
        <input value={nome} onChange={(e) => setNome(e.target.value)} required autoFocus />
      </label>
      <label>
        Órgão
        <input value={orgao} onChange={(e) => setOrgao(e.target.value)} placeholder="Ex.: Tribunal de Contas" />
      </label>
      <label>
        Banca
        <input value={banca} onChange={(e) => setBanca(e.target.value)} placeholder="Ex.: VUNESP" />
      </label>
      <label>
        Nº do edital
        <input value={numero} onChange={(e) => setNumero(e.target.value)} placeholder="Ex.: 01/2026" />
      </label>
      <label>
        Data da prova
        <input type="date" value={dataProva} onChange={(e) => setDataProva(e.target.value)} />
      </label>
      <div className="acoes">
        <button className="primario" type="submit">
          Salvar
        </button>
        <button type="button" onClick={() => setEditando(false)}>
          Cancelar
        </button>
        <span className="mudo">Depois de editar, a análise não muda mais estes campos.</span>
      </div>
    </form>
  );
}
