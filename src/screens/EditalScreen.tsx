import { useEffect, useState } from "react";
import type { AppError } from "../bindings/AppError";
import type { BackendStatus } from "../bindings/BackendStatus";
import type { Edital } from "../bindings/Edital";
import { ErrorBox } from "../components/ErrorBox";
import { Icone } from "../components/Icone";
import { PdfViewer } from "../components/pdf/PdfViewer";
import { ipc, toAppError } from "../ipc";
import { CATALOGO } from "../ipc/errors";
import {
  consolidar,
  contarTopicos,
  obterExtraido,
  type Consolidado,
  type Etapa,
  type Extraido,
  type Materia,
  type Topico,
} from "../model/consolidado";
import { faixas, plural } from "../model/formato";

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
  const [extraido, setExtraido] = useState<Extraido | null>(null);
  const [erro, setErro] = useState<AppError | null>(null);
  const [mostrarAviso, setMostrarAviso] = useState(!!aviso);
  const [aba, setAba] = useState<Aba>("visao");
  const [pagina, setPagina] = useState<{ n: number } | null>(null);

  useEffect(() => {
    ipc.editalObter(id).then(setEdital, (e) => setErro(toAppError(e)));
    void obterExtraido(id).then(setExtraido);
  }, [id]);

  function irParaPagina(n: number) {
    setPagina({ n });
    setAba("documento");
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

  const visao = consolidar(edital, extraido);

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
          <ProximoPasso visao={visao} claude={claude} onAjustes={onAjustes} />
          <Numeros edital={edital} visao={visao} />

          {visao.cargos && <Cargos visao={visao} />}

          <section className="bloco">
            <header className="bloco-cabeca">
              <h2 className="secao">
                <span className="secao-num">§</span> Conteúdo programático
              </h2>
              {visao.cargo && <p className="mudo">Para {visao.cargo.nome}</p>}
            </header>
            {visao.materias ? (
              <ConteudoProgramatico materias={visao.materias} onPagina={irParaPagina} />
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
            <button className="perigo" onClick={remover}>
              <Icone nome="lixeira" /> Remover edital
            </button>
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

/** Uma única chamada para a ação, com a explicação do porquê. Nada de jobs, filas ou modelos. */
function ProximoPasso({
  visao,
  claude,
  onAjustes,
}: {
  visao: Consolidado;
  claude: BackendStatus;
  onAjustes: () => void;
}) {
  const semClaude = claude.estado === "indisponivel";
  const conteudo: Record<Etapa, { titulo: string; texto: string; acao?: string }> = {
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
        "O Claude lê o edital inteiro, encontra os cargos oferecidos e monta o conteúdo programático do seu, com cada tópico ligado à página de onde veio.",
      acao: "Analisar edital",
    },
    escolher_cargo: {
      titulo: "Qual destes cargos é o seu?",
      texto: "Escolha o cargo para o qual você vai estudar. O conteúdo programático é montado só para ele.",
      acao: "Confirmar cargo",
    },
    extrair_conteudo: {
      titulo: "Monte o conteúdo programático",
      texto: "Com o cargo definido, o Claude separa as matérias e os tópicos que caem na sua prova.",
      acao: "Extrair conteúdo",
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

  return (
    <section className={`proximo ${bloqueado ? "bloqueado" : ""}`}>
      <p className="kicker">{bloqueado ? "Atenção" : "Próximo passo"}</p>
      <h2 className="proximo-titulo">{c.titulo}</h2>
      <p className="proximo-texto">{c.texto}</p>
      {c.acao && (
        <div className="acoes">
          {semClaude && visao.etapa !== "pronto" ? (
            <>
              <button className="primario" onClick={onAjustes}>
                Conectar o Claude <Icone nome="seta" />
              </button>
              <span className="mudo">A análise usa o Claude da sua conta. Ele ainda não está conectado.</span>
            </>
          ) : (
            <>
              {/* Habilitado quando a extração (Item 4) e o planejamento chegarem ao core. */}
              <button className="primario" disabled>
                {c.acao} <Icone nome="seta" />
              </button>
              <span className="mudo">Disponível na próxima versão.</span>
            </>
          )}
        </div>
      )}
    </section>
  );
}

function Numeros({ edital, visao }: { edital: Edital; visao: Consolidado }) {
  const itens: { valor: number | null; rotulo: string }[] = [
    { valor: edital.total_paginas, rotulo: "páginas" },
    { valor: visao.cargos?.length ?? null, rotulo: "cargos" },
    { valor: visao.materias?.length ?? null, rotulo: "matérias" },
    { valor: visao.totalTopicos, rotulo: "tópicos" },
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

function Cargos({ visao }: { visao: Consolidado }) {
  return (
    <section className="bloco">
      <header className="bloco-cabeca">
        <h2 className="secao">
          <span className="secao-num">§</span> Cargos do edital
        </h2>
      </header>
      <ul className="cargos">
        {visao.cargos!.map((c) => (
          <li key={c.id} className={c.id === visao.cargo?.id ? "escolhido" : ""}>
            <span className="cargo-nome">{c.nome}</span>
            <span className="mudo">
              {[c.escolaridade, c.vagas !== null && plural(c.vagas, "vaga", "vagas")].filter(Boolean).join(" · ")}
            </span>
            {c.id === visao.cargo?.id && <span className="selo pronto">Seu cargo</span>}
          </li>
        ))}
      </ul>
    </section>
  );
}

function ConteudoProgramatico({ materias, onPagina }: { materias: Materia[]; onPagina: (n: number) => void }) {
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
        return (
          <li key={m.id} className={aberta ? "aberta" : ""}>
            <button className="materia-cabeca" onClick={() => alternar(m.id)} aria-expanded={aberta}>
              <span className="materia-num">{String(i + 1).padStart(2, "0")}</span>
              <span className="materia-nome">{m.nome}</span>
              <span className="mudo">{plural(contarTopicos(m.topicos), "tópico", "tópicos")}</span>
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
            {t.pagina !== null && (
              <button className="pag" onClick={() => onPagina(t.pagina!)} title="Ver no documento original">
                p. {t.pagina}
              </button>
            )}
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

  function abrir() {
    setNome(edital.nome);
    setOrgao(edital.orgao ?? "");
    setBanca(edital.banca ?? "");
    setEditando(true);
  }

  async function salvar(ev: React.FormEvent) {
    ev.preventDefault();
    try {
      onSalvo(await ipc.editalAtualizar(edital.id, { nome, orgao: orgao || null, banca: banca || null }));
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
          <button className="icone-btn editar" onClick={abrir} aria-label="Editar nome, órgão e banca" title="Editar">
            <Icone nome="lapis" />
          </button>
        </h1>
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
      <div className="acoes">
        <button className="primario" type="submit">
          Salvar
        </button>
        <button type="button" onClick={() => setEditando(false)}>
          Cancelar
        </button>
      </div>
    </form>
  );
}
