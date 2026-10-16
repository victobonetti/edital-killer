// OCR local das páginas sem texto ou com texto ilegível (tesseract.js, offline).
// Os arquivos do tesseract e o modelo de português vêm de public/ocr/
// (scripts/ocr-assets.mjs); nada é buscado na rede.
import { createWorker, type Worker } from "tesseract.js";
import type { PDFPageProxy } from "pdfjs-dist";
import type { ItemTexto } from "../bindings/ItemTexto";

const ARQUIVOS = "/ocr";
/** 300 dpi: a página PDF mede em pontos (72 por polegada). */
const ESCALA = 300 / 72;

/** Subconjunto da saída do tesseract (`blocks`) que o OCR usa. */
export type BlocoOcr = {
  paragraphs: { lines: { words: { text: string; bbox: { x0: number; y0: number; x1: number; y1: number } }[] }[] }[];
};

export type PaginaOcr = { texto: string; itens: ItemTexto[]; confianca: number };

export async function criarOcr(): Promise<Worker> {
  return createWorker("por", 1 /* OEM.LSTM_ONLY */, {
    workerPath: `${ARQUIVOS}/worker.min.js`,
    corePath: ARQUIVOS,
    langPath: ARQUIVOS,
    gzip: false,
    cacheMethod: "none",
    workerBlobURL: false,
  });
}

const arredonda = (n: number) => Math.round(n * 10) / 10;

/**
 * Monta o texto da página a partir das palavras: espaço entre palavras, `\n` entre
 * linhas e parágrafos. Cada palavra vira um item com o offset (UTF-16) e a caixa em
 * pontos da página (a imagem foi renderizada em `escala`).
 */
export function montarTextoOcr(blocos: BlocoOcr[], escala: number): { texto: string; itens: ItemTexto[] } {
  let texto = "";
  const itens: ItemTexto[] = [];
  for (const bloco of blocos) {
    for (const par of bloco.paragraphs) {
      for (const linha of par.lines) {
        const palavras = linha.words.filter((w) => w.text.trim().length > 0);
        palavras.forEach((w, k) => {
          if (k > 0) texto += " ";
          const { x0, y0, x1, y1 } = w.bbox;
          itens.push({
            t: w.text,
            i: texto.length,
            b: [arredonda(x0 / escala), arredonda(y0 / escala), arredonda((x1 - x0) / escala), arredonda((y1 - y0) / escala)],
          });
          texto += w.text;
        });
        if (palavras.length > 0) texto += "\n";
      }
    }
  }
  return { texto, itens };
}

export async function ocrPagina(worker: Worker, page: PDFPageProxy): Promise<PaginaOcr> {
  const viewport = page.getViewport({ scale: ESCALA });
  const canvas = document.createElement("canvas");
  canvas.width = Math.ceil(viewport.width);
  canvas.height = Math.ceil(viewport.height);
  try {
    await page.render({ canvas, viewport }).promise;
    const { data } = await worker.recognize(canvas, {}, { blocks: true, text: false });
    const { texto, itens } = montarTextoOcr((data.blocks ?? []) as BlocoOcr[], ESCALA);
    return { texto, itens, confianca: data.confidence };
  } finally {
    // Libera a memória do canvas (~35 MB numa A4 a 300 dpi) sem esperar o GC.
    canvas.width = 0;
    canvas.height = 0;
  }
}
