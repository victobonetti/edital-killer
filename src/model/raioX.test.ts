import { describe, expect, it } from "vitest";
import type { EstruturaProva } from "../bindings/EstruturaProva";
import type { Materia } from "../bindings/Materia";
import type { Topico } from "../bindings/Topico";
import { leisCitadas, raioX } from "./raioX";

const topico = (texto: string): Topico => ({
  id: texto,
  texto,
  pagina: 1,
  verificado: true,
  inicio: null,
  fim: null,
  filhos: [],
});

const materia = (nome: string, bloco: string, questoes: number | null, peso = 1, nTopicos = 2): Materia => ({
  id: nome,
  nome,
  pagina: 1,
  bloco,
  questoes,
  peso,
  quadro_verificado: true,
  topicos: Array.from({ length: nTopicos }, (_, i) => topico(`${nome} ${i}`)),
});

const estrutura: EstruturaProva = {
  duracao_minutos: 240,
  duracao_verificada: true,
  minimo_geral: 60,
  minimo_verificado: true,
  blocos: [
    { nome: "Gerais", questoes: 30, minimo: 50, verificado: true },
    { nome: "Específicos", questoes: null, minimo: 50, verificado: true },
  ],
};

describe("raioX", () => {
  const ms = [
    materia("Português", "Gerais", 20),
    materia("Matemática", "Gerais", null),
    materia("Informática", "Gerais", null),
    materia("TI", "Específicos", 40, 2, 20),
  ];
  const r = raioX(ms, estrutura)!;

  it("divide as questões do bloco entre as matérias sem número", () => {
    const mat = r.materias.find((m) => m.nome === "Matemática")!;
    expect(mat.questoes).toBe(5);
    expect(mat.estimado).toBe(true);
  });

  it("calcula pontos, fração e ordena pelo valor", () => {
    expect(r.totalPontos).toBe(20 + 5 + 5 + 80);
    expect(r.materias[0].nome).toBe("TI");
    expect(r.materias[0].fracao).toBeCloseTo(80 / 110);
    expect(r.pareto50).toBe(1);
    expect(r.pareto80).toBe(2);
  });

  it("calcula densidade, ritmo e mínimos", () => {
    expect(r.materias[0].porTopico).toBe(4);
    expect(r.totalQuestoes).toBe(70);
    expect(r.ritmo).toBeCloseTo((240 * 60) / 70);
    const gerais = r.blocos.find((b) => b.nome === "Gerais")!;
    expect(gerais.pontosMinimos).toBe(15);
    expect(gerais.conferir).toBe(true); // tem matérias estimadas
    expect(r.minimoGeral!.pontosMinimos).toBe(66);
  });

  it("sem questões em lugar nenhum não há raio-X", () => {
    expect(raioX([materia("X", "Gerais", null)], null)).toBeNull();
  });
});

describe("leisCitadas", () => {
  it("acha e normaliza leis sem repetir", () => {
    const m = materia("Legislação", "Específicos", 10);
    m.topicos = [
      topico("Lei nº 14.133/2021 (licitações)"),
      topico("Lei 14.133, art. 5º"),
      topico("LC nº 182/2021 e Lei Complementar 101/2000"),
      topico("Decreto-Lei nº 200/1967"),
    ];
    expect(leisCitadas([m])).toEqual(["Lei 14.133/2021", "LC 182/2021", "LC 101/2000", "Decreto-Lei 200/1967"]);
  });
});
