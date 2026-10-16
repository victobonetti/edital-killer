import { useCallback, useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { AppError } from "../bindings/AppError";
import type { Edital } from "../bindings/Edital";
import type { EditalStatus } from "../bindings/EditalStatus";
import { ErrorBox } from "../components/ErrorBox";
import { importarArquivo, retomarImportacao, type Progresso } from "../import/importar";
import { ipc, toAppError } from "../ipc";

const STATUS: Record<EditalStatus, string> = {
  importando: "Importação incompleta",
  importado: "Importado",
  sem_texto: "Sem texto (OCR não suportado)",
};

type EmAndamento = { arquivo: string; progresso: Progresso | null };
type Falha = { arquivo: string; erro: AppError };

function nomeArquivo(caminho: string) {
  return caminho.split(/[\\/]/).pop() ?? caminho;
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

  const carregar = useCallback(() => {
    ipc.editaisListar().then(setEditais, (e) => setErroLista(toAppError(e)));
  }, []);
  useEffect(carregar, [carregar]);

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
            aviso: r.duplicado ? `“${arquivo}” já estava na biblioteca; abrimos o edital existente.` : undefined,
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
      setFalhas([{ arquivo: "seleção de arquivos", erro: toAppError(e) }]);
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
          Importando <strong>{andamento.arquivo}</strong>
          {andamento.progresso
            ? ` · extraindo texto, página ${andamento.progresso.pagina} de ${andamento.progresso.total}`
            : " · copiando…"}
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
