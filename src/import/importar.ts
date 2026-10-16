// Fluxo de importação: o core valida e copia o PDF; a UI abre com pdf.js, extrai o
// texto por página e envia em lotes; o core classifica cada página e diz quais
// precisam de OCR; a UI roda o OCR nelas e reenvia; o core fecha a importação.
import type { Edital } from "../bindings/Edital";
import type { PaginaInput } from "../bindings/PaginaInput";
import type { AppError } from "../bindings/AppError";
import { abrirPdfDoEdital, type PDFDocumentProxy } from "../components/pdf/pdfjs";
import { montarTextoPagina, type ItemPdf } from "../components/pdf/texto";
import { ipc, isAppError } from "../ipc";
import { criarOcr, ocrPagina } from "./ocr";

const LOTE = 10;

export type Progresso = { fase: "texto" | "ocr"; pagina: number; total: number };

export type ResultadoImport = {
  edital: Edital;
  duplicado: boolean;
  /** Páginas em que o OCR falhou (ficaram como estavam). */
  falhasOcr: number[];
};

type Opcoes = { onProgresso?: (p: Progresso) => void; sinal?: AbortSignal };

export async function importarArquivo(caminho: string, opcoes: Opcoes = {}): Promise<ResultadoImport> {
  const { edital, duplicado } = await ipc.importarIniciar(caminho);
  if (duplicado) return { edital, duplicado, falhasOcr: [] };
  return { ...(await extrairESalvar(edital.id, opcoes)), duplicado: false };
}

/** Retoma um edital que ficou em `importando` (app fechado no meio da importação). */
export async function retomarImportacao(id: string, opcoes: Opcoes = {}): Promise<ResultadoImport> {
  await ipc.importarRetomar(id);
  return { ...(await extrairESalvar(id, opcoes)), duplicado: false };
}

async function abortarSeCancelado(id: string, sinal?: AbortSignal) {
  if (!sinal?.aborted) return;
  await ipc.importarAbortar(id, "IMPORT_CANCELADO");
  throw { code: "IMPORT_CANCELADO", detail: "" } satisfies AppError;
}

async function extrairESalvar(
  id: string,
  { onProgresso, sinal }: Opcoes,
): Promise<{ edital: Edital; falhasOcr: number[] }> {
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
    const precisamOcr: number[] = [];
    let lote: PaginaInput[] = [];
    for (let n = 1; n <= total; n++) {
      await abortarSeCancelado(id, sinal);
      let pagina: PaginaInput;
      try {
        const page = await doc.getPage(n);
        const conteudo = await page.getTextContent();
        const itens = conteudo.items.filter((it): it is ItemPdf & typeof it => "str" in it);
        pagina = { pagina: n, ...montarTextoPagina(itens), origem: "pdf" };
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
        precisamOcr.push(...(await ipc.importarPaginas(id, lote)));
        lote = [];
      }
      onProgresso?.({ fase: "texto", pagina: n, total });
    }

    const falhasOcr = await rodarOcr(id, doc, precisamOcr, { onProgresso, sinal });
    return { edital: await ipc.importarFinalizar(id, total), falhasOcr };
  } finally {
    void doc.loadingTask.destroy();
  }
}

/** OCR página a página. Falha numa página não derruba a importação: ela fica como estava. */
async function rodarOcr(id: string, doc: PDFDocumentProxy, paginas: number[], { onProgresso, sinal }: Opcoes) {
  const falhas: number[] = [];
  if (paginas.length === 0) return falhas;
  onProgresso?.({ fase: "ocr", pagina: 0, total: paginas.length });
  let worker;
  try {
    worker = await criarOcr();
  } catch (e) {
    console.error("OCR indisponível", e);
    return [...paginas];
  }
  try {
    for (const [k, n] of paginas.entries()) {
      await abortarSeCancelado(id, sinal);
      try {
        const page = await doc.getPage(n);
        const r = await ocrPagina(worker, page);
        page.cleanup();
        await ipc.importarPaginas(id, [
          { pagina: n, texto: r.texto, itens: r.itens, origem: "ocr", confianca: r.confianca },
        ]);
      } catch (e) {
        if (isAppError(e)) throw e;
        console.error(`OCR da página ${n} falhou`, e);
        falhas.push(n);
      }
      onProgresso?.({ fase: "ocr", pagina: k + 1, total: paginas.length });
    }
  } finally {
    void worker.terminate();
  }
  return falhas;
}
