import { describe, expect, it } from "vitest";
import { montarTextoPagina } from "./texto";

describe("montarTextoPagina", () => {
  it("concatena itens e quebra linha em hasEOL", () => {
    const r = montarTextoPagina([
      { str: "1. Língua", hasEOL: false },
      { str: " Portuguesa", hasEOL: true },
      { str: "", hasEOL: true },
      { str: "Crase.", hasEOL: false },
    ]);
    expect(r.texto).toBe("1. Língua Portuguesa\n\nCrase.");
    expect(r.itens).toEqual([
      { t: "1. Língua", i: 0 },
      { t: " Portuguesa", i: 9 },
      { t: "Crase.", i: 22 },
    ]);
  });

  it("índices batem com o texto", () => {
    const r = montarTextoPagina([
      { str: "🚀 a", hasEOL: true },
      { str: "b", hasEOL: false },
    ]);
    for (const it of r.itens) expect(r.texto.slice(it.i, it.i + it.t.length)).toBe(it.t);
  });

  it("página vazia", () => {
    expect(montarTextoPagina([])).toEqual({ texto: "", itens: [] });
  });
});
