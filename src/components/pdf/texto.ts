import type { ItemTexto } from "../../bindings/ItemTexto";

/** Subconjunto de `TextItem` do pdf.js que a extração usa. */
export type ItemPdf = { str: string; hasEOL: boolean };

/**
 * Monta o texto da página na ordem em que o pdf.js entrega os itens, com `\n`
 * onde o item termina linha. Cada item guarda onde começa no texto (em unidades
 * UTF-16, o que `String.length` mede): esse mapa permite destacar depois.
 */
export function montarTextoPagina(itens: Iterable<ItemPdf>): { texto: string; itens: ItemTexto[] } {
  let texto = "";
  const saida: ItemTexto[] = [];
  for (const it of itens) {
    if (it.str.length > 0) {
      saida.push({ t: it.str, i: texto.length });
      texto += it.str;
    }
    if (it.hasEOL) texto += "\n";
  }
  return { texto, itens: saida };
}
