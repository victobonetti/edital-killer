import { useCallback, useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { AppError } from "../bindings/AppError";
import type { Edital } from "../bindings/Edital";
import { ErrorBox } from "../components/ErrorBox";
import { Icone } from "../components/Icone";
import { importarArquivo, retomarImportacao, type Progresso } from "../import/importar";
import { ipc, onAnalise, toAppError } from "../ipc";
import { consolidar, ETAPA_ROTULO, type Consolidado } from "../model/consolidado";
import { plural, quando } from "../model/formato";

type EmAndamento = { arquivo: string; progresso: Progresso | null };
type Falha = { arquivo: string; erro: AppError };
type Item = { edital: Edital; visao: Consolidado };

function nomeArquivo(caminho: string) {
  return caminho.split(/[\\/]/).pop() ?? caminho;
}

export function Biblioteca({ onAbrir }: { onAbrir: (id: string, aviso?: string) => void }) {
  const [itens, setItens] = useState<Item[] | null>(null);
  const [erroLista, setErroLista] = useState<AppError | null>(null);
  const [andamento, setAndamento] = useState<EmAndamento | null>(null);
  const [falhas, setFalhas] = useState<Falha[]>([]);
  const [arrastando, setArrastando] = useState(false);

  const carregar = useCallback(() => {
    ipc.editaisListar().then(
      async (editais) =>
        setItens(
          await Promise.all(
            editais.map(async (e) => ({
              edital: e,
              visao: consolidar(e, await ipc.editalAnalise(e.id).catch(() => null)),
            })),
          ),
        ),
      (e) => setErroLista(toAppError(e)),
    );
  }, []);
  useEffect(() => {
    carregar();
    // Análises terminam em segundo plano: o selo do cartão acompanha.
    const desliga = onAnalise(carregar);
    return () => {
      void desliga.then((f) => f());
    };
  }, [carregar]);

  const importar = useCallback(
    async (caminhos: string[]) => {
      if (andamento || caminhos.length === 0) return;
      const novasFalhas: Falha[] = [];
      let ultimo: { id: string; aviso?: string } | null = null;
      for (const caminho of caminhos) {
        const arquivo = nomeArquivo(caminho);
        setAndamento({ arquivo, progresso: null });
        try {
          const r = await importarArquivo(caminho, (progresso) => setAndamento({ arquivo, progresso }));
          ultimo = {
            id: r.edital.id,
            aviso: r.duplicado ? `Você já tinha importado “${arquivo}”. Abrimos o edital que já estava aqui.` : undefined,
          };
        } catch (e) {
          novasFalhas.push({ arquivo, erro: toAppError(e) });
        }
      }
      setAndamento(null);
      setFalhas(novasFalhas);
      carregar();
      // Um arquivo só e sem falha: abre direto o edital.
      if (caminhos.length === 1 && ultimo) onAbrir(ultimo.id, ultimo.aviso);
    },
    [andamento, carregar, onAbrir],
  );

  useEffect(() => {
    const desliga = getCurrentWebview().onDragDropEvent((ev) => {
      const p = ev.payload;
      if (p.type === "enter" || p.type === "over") setArrastando(true);
      else if (p.type === "leave") setArrastando(false);
      else if (p.type === "drop") {
        setArrastando(false);
        void importar(p.paths.filter((c) => c.toLowerCase().endsWith(".pdf")));
      }
    });
    return () => {
      void desliga.then((f) => f());
    };
  }, [importar]);

  async function escolher() {
    try {
      await importar(await ipc.escolherPdfs());
    } catch (e) {
      setFalhas([{ arquivo: "Seleção de arquivos", erro: toAppError(e) }]);
    }
  }

  async function retomar(e: Edital) {
    setAndamento({ arquivo: e.arquivo_original, progresso: null });
    try {
      const ed = await retomarImportacao(e.id, (progresso) => setAndamento({ arquivo: e.arquivo_original, progresso }));
      onAbrir(ed.id);
    } catch (err) {
      setFalhas([{ arquivo: e.arquivo_original, erro: toAppError(err) }]);
      carregar();
    } finally {
      setAndamento(null);
    }
  }

  const abrir = (it: Item) => (it.edital.status === "importando" ? retomar(it.edital) : onAbrir(it.edital.id));
  const vazio = itens && itens.length === 0 && !andamento;

  return (
    <main className={`tela biblioteca ${arrastando ? "arrastando" : ""}`}>
      <header className="tela-cabeca">
        <div>
          <p className="kicker">Biblioteca</p>
          <h1 className="titulo-display">Seus editais</h1>
          {itens && itens.length > 0 && (
            <p className="mudo">
              {plural(itens.length, "edital guardado", "editais guardados")} neste computador. Arraste PDFs para cá a
              qualquer momento.
            </p>
          )}
        </div>
        {!vazio && (
          <button className="primario" onClick={escolher} disabled={!!andamento}>
            <Icone nome="mais" /> Importar edital
          </button>
        )}
      </header>

      {falhas.map((f) => (
        <ErrorBox
          key={f.arquivo}
          erro={f.erro}
          contexto={f.arquivo}
          onFechar={() => setFalhas((fs) => fs.filter((x) => x !== f))}
        />
      ))}
      {erroLista && <ErrorBox erro={erroLista} />}

      {vazio && (
        <button className="vazio" onClick={escolher}>
          <span className="vazio-folha" aria-hidden>
            <span />
            <span />
            <span />
          </span>
          <span className="vazio-titulo">Comece pelo edital do seu concurso</span>
          <span className="mudo">
            Clique para escolher o PDF, ou arraste o arquivo para esta janela. Ele fica guardado só neste computador.
          </span>
          <span className="primario falso">
            <Icone nome="mais" /> Importar edital
          </span>
        </button>
      )}

      {(andamento || (itens && itens.length > 0)) && (
        <ul className="grade-editais">
          {andamento && (
            <li className="cartao importando" aria-live="polite">
              <p className="kicker">Importando</p>
              <h2 className="cartao-nome">{andamento.arquivo}</h2>
              <div className="cartao-pe">
                <span className="mudo">
                  {andamento.progresso
                    ? `Lendo página ${andamento.progresso.pagina} de ${andamento.progresso.total}`
                    : "Preparando o arquivo…"}
                </span>
              </div>
              <div className="barra">
                <span
                  style={{
                    width: andamento.progresso
                      ? `${(100 * andamento.progresso.pagina) / andamento.progresso.total}%`
                      : "8%",
                  }}
                />
              </div>
            </li>
          )}
          {itens?.map((it, i) => (
            <li key={it.edital.id} style={{ animationDelay: `${Math.min(i, 8) * 45}ms` }}>
              <button className={`cartao etapa-${it.visao.etapa}`} onClick={() => abrir(it)} disabled={!!andamento}>
                <p className="kicker">
                  {[it.edital.orgao, it.edital.banca].filter(Boolean).join(" · ") || "Órgão a definir"}
                </p>
                <h2 className="cartao-nome">{it.edital.nome}</h2>
                {it.visao.cargo && <p className="cartao-cargo">{it.visao.cargo.nome}</p>}
                <div className="cartao-pe">
                  {it.visao.rodando ? (
                    <span className="selo analisando">Analisando…</span>
                  ) : (
                    <span className={`selo ${it.visao.etapa}`}>{ETAPA_ROTULO[it.visao.etapa]}</span>
                  )}
                  <span className="mudo">
                    {it.visao.materias
                      ? plural(it.visao.materias.length, "matéria", "matérias")
                      : it.edital.total_paginas
                        ? plural(it.edital.total_paginas, "página", "páginas")
                        : "clique para continuar"}
                    {" · "}
                    {quando(it.edital.importado_em)}
                  </span>
                </div>
              </button>
            </li>
          ))}
        </ul>
      )}
      {arrastando && (
        <div className="soltar">
          <span>Solte para importar</span>
        </div>
      )}
    </main>
  );
}
