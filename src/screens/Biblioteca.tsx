import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { AppError } from "../bindings/AppError";
import type { Edital } from "../bindings/Edital";
import type { EditalStatus } from "../bindings/EditalStatus";
import { ErrorBox } from "../components/ErrorBox";
import { importarArquivo, retomarImportacao, type Progresso, type ResultadoImport } from "../import/importar";
import { faixas } from "../components/faixas";
import { ipc, toAppError } from "../ipc";

const STATUS: Record<EditalStatus, string> = {
  importando: "Importação incompleta",
  importado: "Importado",
  sem_texto: "Sem texto legível",
};

type EmAndamento = { arquivo: string; progresso: Progresso | null };
type Falha = { arquivo: string; erro: AppError };

function nomeArquivo(caminho: string) {
  return caminho.split(/[\\/]/).pop() ?? caminho;
}

function descreverProgresso(p: Progresso | null): string {
  if (!p) return " · copiando…";
  if (p.fase === "texto") return ` · extraindo texto, página ${p.pagina} de ${p.total}`;
  return p.pagina === 0
    ? ` · preparando OCR para ${p.total} página(s)…`
    : ` · lendo por OCR, página ${p.pagina} de ${p.total}`;
}

/** Aviso para abrir junto com o edital recém-importado. */
function avisoDe(r: ResultadoImport, arquivo: string): string | undefined {
  if (r.duplicado) return `“${arquivo}” já estava na biblioteca; abrimos o edital existente.`;
  if (r.falhasOcr.length > 0) return `O OCR falhou nas páginas ${faixas(r.falhasOcr)}; elas ficaram como estavam no PDF.`;
  return undefined;
}

export function Biblioteca({
  onAbrir,
}: {
  onAbrir: (id: string, aviso?: string) => void;
}) {
  const [editais, setEditais] = useState<Edital[] | null>(null);
  const [erroLista, setErroLista] = useState<AppError | null>(null);
  const [andamento, setAndamento] = useState<EmAndamento | null>(null);
  const [falhas, setFalhas] = useState<Falha[]>([]);
  const [arrastando, setArrastando] = useState(false);
  const cancelamento = useRef<AbortController | null>(null);

  const carregar = useCallback(() => {
    ipc.editaisListar().then(setEditais, (e) => setErroLista(toAppError(e)));
  }, []);
  useEffect(carregar, [carregar]);

  const importar = useCallback(
    async (caminhos: string[]) => {
      if (andamento || caminhos.length === 0) return;
      const novasFalhas: Falha[] = [];
      let ultimo: { id: string; aviso?: string } | null = null;
      const controle = new AbortController();
      cancelamento.current = controle;
      for (const caminho of caminhos) {
        if (controle.signal.aborted) break;
        const arquivo = nomeArquivo(caminho);
        setAndamento({ arquivo, progresso: null });
        try {
          const r = await importarArquivo(caminho, {
            onProgresso: (progresso) => setAndamento({ arquivo, progresso }),
            sinal: controle.signal,
          });
          ultimo = { id: r.edital.id, aviso: avisoDe(r, arquivo) };
          if (r.falhasOcr.length > 0 && caminhos.length > 1) {
            novasFalhas.push({ arquivo, erro: { code: "IMPORT_OCR_FAILED", detail: `páginas ${faixas(r.falhasOcr)}` } });
          }
        } catch (e) {
          const erro = toAppError(e);
          if (erro.code !== "IMPORT_CANCELADO") novasFalhas.push({ arquivo, erro });
        }
      }
      cancelamento.current = null;
      setAndamento(null);
      setFalhas(novasFalhas);
      carregar();
      // Um arquivo só e sem falha: abre direto o edital.
      if (caminhos.length === 1 && ultimo && !controle.signal.aborted) onAbrir(ultimo.id, ultimo.aviso);
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
      setFalhas([{ arquivo: "seleção de arquivos", erro: toAppError(e) }]);
    }
  }

  async function retomar(e: Edital) {
    setAndamento({ arquivo: e.arquivo_original, progresso: null });
    const controle = new AbortController();
    cancelamento.current = controle;
    try {
      const r = await retomarImportacao(e.id, {
        onProgresso: (progresso) => setAndamento({ arquivo: e.arquivo_original, progresso }),
        sinal: controle.signal,
      });
      onAbrir(r.edital.id, avisoDe(r, e.arquivo_original));
    } catch (err) {
      const erro = toAppError(err);
      if (erro.code !== "IMPORT_CANCELADO") setFalhas([{ arquivo: e.arquivo_original, erro }]);
      carregar();
    } finally {
      cancelamento.current = null;
      setAndamento(null);
    }
  }

  return (
    <main className={`tela biblioteca ${arrastando ? "arrastando" : ""}`}>
      <header className="tela-topo">
        <h1>Biblioteca</h1>
        <button className="primario" onClick={escolher} disabled={!!andamento}>
          Importar edital
        </button>
      </header>

      {andamento && (
        <div className="andamento">
          <div className="andamento-topo">
            <span>
              Importando <strong>{andamento.arquivo}</strong>
              {descreverProgresso(andamento.progresso)}
            </span>
            <button className="link" onClick={() => cancelamento.current?.abort()}>
              Cancelar
            </button>
          </div>
          {andamento.progresso && (
            <progress value={andamento.progresso.pagina} max={andamento.progresso.total} />
          )}
        </div>
      )}

      {falhas.map((f) => (
        <ErrorBox
          key={f.arquivo}
          erro={f.erro}
          contexto={f.arquivo}
          onFechar={() => setFalhas((fs) => fs.filter((x) => x !== f))}
        />
      ))}
      {erroLista && <ErrorBox erro={erroLista} />}

      {editais && editais.length === 0 && !andamento && (
        <div className="vazio">
          <p>Nenhum edital ainda.</p>
          <p className="mudo">Clique em “Importar edital” ou arraste arquivos PDF para esta janela.</p>
        </div>
      )}

      {editais && editais.length > 0 && (
        <table className="tabela">
          <thead>
            <tr>
              <th>Nome</th>
              <th>Órgão</th>
              <th>Banca</th>
              <th>Importado em</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {editais.map((e) => (
              <tr
                key={e.id}
                onClick={() => (e.status === "importando" ? retomar(e) : onAbrir(e.id))}
                tabIndex={0}
                onKeyDown={(ev) => ev.key === "Enter" && (e.status === "importando" ? retomar(e) : onAbrir(e.id))}
              >
                <td>{e.nome}</td>
                <td>{e.orgao ?? "—"}</td>
                <td>{e.banca ?? "—"}</td>
                <td>{new Date(e.importado_em).toLocaleDateString("pt-BR")}</td>
                <td>
                  <span className={`tag ${e.status}`}>{STATUS[e.status]}</span>
                  {e.status === "importando" && <span className="mudo"> · clique para retomar</span>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {arrastando && <div className="soltar">Solte os PDFs para importar</div>}
    </main>
  );
}
