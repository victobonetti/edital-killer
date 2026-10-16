import { getDocument, GlobalWorkerOptions, type PDFDocumentProxy } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { ipc } from "../../ipc";

GlobalWorkerOptions.workerSrc = workerUrl;

export type { PDFDocumentProxy };

/** Abre o PDF de um edital. Os bytes vêm do core; a UI nunca lê o workspace. */
export async function abrirPdfDoEdital(editalId: string): Promise<PDFDocumentProxy> {
  const bytes = await ipc.editalPdf(editalId);
  return getDocument({ data: new Uint8Array(bytes) }).promise;
}
