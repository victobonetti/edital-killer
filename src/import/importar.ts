// Fluxo de importação: o core valida e copia o PDF; a UI abre com pdf.js,
// extrai o texto por página e envia em lotes; o core fecha a importação.
import type { Edital } from "../bindings/Edital";
import type { PaginaInput } from "../bindings/PaginaInput";
import type { AppError } from "../bindings/AppError";
import { abrirPdfDoEdital, type PDFDocumentProxy } from "../components/pdf/pdfjs";
import { montarTextoPagina, type ItemPdf } from "../components/pdf/texto";
import { ipc, isAppError } from "../ipc";

const LOTE = 10;

export type Progresso = { pagina: number; total: number };

export type ResultadoImport = { edital: Edital; duplicado: boolean };

export async function importarArquivo(
  caminho: string,
  onProgresso?: (p: Progresso) => void,
): Promise<ResultadoImport> {
  const { edital, duplicado } = await ipc.importarIniciar(caminho);
  if (duplicado) return { edital, duplicado };
  return { edital: await extrairESalvar(edital.id, onProgresso), duplicado: false };
}

/** Retoma um edital que ficou em `importando` (app fechado no meio da importação). */
export async function retomarImportacao(
  id: string,
  onProgresso?: (p: Progresso) => void,
): Promise<Edital> {
  await ipc.importarRetomar(id);
  return extrairESalvar(id, onProgresso);
}

async function extrairESalvar(id: string, onProgresso?: (p: Progresso) => void): Promise<Edital> {
  let doc: PDFDocumentProxy;
  try {
    doc = await abrirPdfDoEdital(id);
  } catch (e) {
    if (isAppError(e)) throw e;
    const code = (e as { name?: string })?.name === "PasswordException" ? "IMPORT_ENCRYPTED" : "IMPORT_CORRUPTED";
    await ipc.importarAbortar(id, code);
    throw { code, detail: String((e as Error)?.message ?? e) } satisfies AppError;
  }

  try {
    const total = doc.numPages;
    let lote: PaginaInput[] = [];
    for (let n = 1; n <= total; n++) {
      let pagina: PaginaInput;
      try {
        const page = await doc.getPage(n);
        const conteudo = await page.getTextContent();
        const itens = conteudo.items.filter((it): it is ItemPdf & typeof it => "str" in it);
        pagina = { pagina: n, ...montarTextoPagina(itens) };
        page.cleanup();
      } catch (e) {
        await ipc.importarAbortar(id, "IMPORT_CORRUPTED");
        throw {
          code: "IMPORT_CORRUPTED",
          detail: `página ${n}: ${String((e as Error)?.message ?? e)}`,
        } satisfies AppError;
      }
      lote.push(pagina);
      if (lote.length === LOTE || n === total) {
        await ipc.importarPaginas(id, lote);
        lote = [];
      }
      onProgresso?.({ pagina: n, total });
    }
    return await ipc.importarFinalizar(id, total);
  } finally {
    void doc.loadingTask.destroy();
  }
}
