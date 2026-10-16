import { describe, expect, it } from "vitest";
import { consumo, duracao, percentual, tokens } from "./formato";

describe("formato", () => {
  it("duração em s, min e h", () => {
    expect(duracao(4_400)).toBe("4s");
    expect(duracao(125_000)).toBe("2min 5s");
    expect(duracao(3_720_000)).toBe("1h 2min");
  });

  it("tokens em mil", () => {
    expect(tokens(950)).toBe("950");
    expect(tokens(13_203)).toBe("13,2 mil");
  });

  it("percentual arredondado", () => {
    expect(percentual(0.084)).toBe("8%");
  });

  it("consumo soma cache na entrada", () => {
    const texto = consumo({
      modelo: null,
      tokens_entrada: 2,
      tokens_saida: 115,
      tokens_cache_escrita: 13_203,
      tokens_cache_leitura: 0,
      tokens_raciocinio: 62,
      custo_estimado_usd: 0.0027,
      duracao_ms: 2304,
    });
    expect(texto).toBe("13,2 mil entrada · 115 saída · US$ 0,0027 estimado");
  });
});
