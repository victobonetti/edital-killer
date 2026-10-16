import { describe, expect, it } from "vitest";
import { montarTextoOcr, type BlocoOcr } from "./ocr";

const w = (text: string, x0: number, y0: number, x1: number, y1: number) => ({ text, bbox: { x0, y0, x1, y1 } });

describe("montarTextoOcr", () => {
  it("junta palavras com espaço e linhas com quebra, com offsets e caixas em pontos", () => {
    const blocos: BlocoOcr[] = [
      {
        paragraphs: [
          { lines: [{ words: [w("Conteúdo", 100, 50, 300, 90), w("Programático", 320, 50, 600, 90)] }] },
          { lines: [{ words: [w("Língua", 100, 150, 250, 190), w(" ", 0, 0, 0, 0), w("Portuguesa", 260, 150, 500, 190)] }] },
        ],
      },
    ];
    const { texto, itens } = montarTextoOcr(blocos, 2);
    expect(texto).toBe("Conteúdo Programático\nLíngua Portuguesa\n");
    expect(itens.map((i) => [i.t, i.i])).toEqual([
      ["Conteúdo", 0],
      ["Programático", 9],
      ["Língua", 22],
      ["Portuguesa", 29],
    ]);
    for (const it of itens) expect(texto.slice(it.i, it.i + it.t.length)).toBe(it.t);
    expect(itens[0].b).toEqual([50, 25, 100, 20]);
  });

  it("página em branco vira texto vazio", () => {
    expect(montarTextoOcr([], 1)).toEqual({ texto: "", itens: [] });
  });
});
