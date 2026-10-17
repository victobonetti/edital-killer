// Raio-X da prova: onde está a nota. Tudo derivado do quadro de provas e do conteúdo extraídos.
import type { EstruturaProva } from "../bindings/EstruturaProva";
import type { Materia } from "../bindings/Materia";
import { contarTopicos } from "./consolidado";

export type ValorMateria = {
  id: string;
  nome: string;
  bloco: string | null;
  questoes: number | null;
  peso: number;
  /** questões × peso; `null` sem informação de questões. */
  pontos: number | null;
  /** Questões divididas a partir do total do bloco. */
  estimado: boolean;
  /** Quadro não confirmado no texto do edital. */
  conferir: boolean;
  /** Fração (0–1) da nota total. */
  fracao: number | null;
  topicos: number;
  /** Pontos por tópico: o "ROI" de estudar um tópico desta matéria. */
  porTopico: number | null;
};

export type MinimoBloco = { nome: string; pontos: number; minimo: number; pontosMinimos: number; conferir: boolean };

export type RaioX = {
  /** Ordenadas por pontos, maior primeiro; sem pontos vão para o fim. */
  materias: ValorMateria[];
  totalPontos: number;
  totalQuestoes: number;
  /** Menor número de matérias que somam 50% e 80% da nota. */
  pareto50: number;
  pareto80: number;
  /** Segundos por questão objetiva. */
  ritmo: number | null;
  ritmoConferir: boolean;
  blocos: MinimoBloco[];
  minimoGeral: { minimo: number; pontosMinimos: number; conferir: boolean } | null;
  /** Leis e normas citadas no conteúdo programático, sem repetição. */
  leis: string[];
};

export function raioX(materias: Materia[], estrutura: EstruturaProva | null): RaioX | null {
  const blocos = estrutura?.blocos ?? [];

  // Questões informadas por bloco: o que sobra no bloco é dividido entre as matérias sem número.
  const questoes = new Map<string, { q: number; estimado: boolean }>();
  for (const m of materias) if (m.questoes !== null) questoes.set(m.id, { q: m.questoes, estimado: false });
  for (const b of blocos) {
    if (b.questoes === null) continue;
    const doBloco = materias.filter((m) => m.bloco === b.nome);
    const semNumero = doBloco.filter((m) => m.questoes === null);
    const informadas = doBloco.reduce((n, m) => n + (m.questoes ?? 0), 0);
    const sobra = b.questoes - informadas;
    if (semNumero.length === 0 || sobra <= 0) continue;
    for (const m of semNumero) questoes.set(m.id, { q: sobra / semNumero.length, estimado: true });
  }

  const linhas = materias.map((m) => {
    const q = questoes.get(m.id) ?? null;
    const peso = m.peso ?? 1;
    const pontos = q ? q.q * peso : null;
    const topicos = contarTopicos(m.topicos);
    return {
      id: m.id,
      nome: m.nome,
      bloco: m.bloco,
      questoes: q?.q ?? null,
      peso,
      pontos,
      estimado: q?.estimado ?? false,
      conferir: !m.quadro_verificado,
      fracao: null as number | null,
      topicos,
      porTopico: pontos !== null && topicos > 0 ? pontos / topicos : null,
    };
  });

  const totalPontos = linhas.reduce((n, l) => n + (l.pontos ?? 0), 0);
  if (totalPontos <= 0) return null;
  for (const l of linhas) l.fracao = l.pontos !== null ? l.pontos / totalPontos : null;
  linhas.sort((a, b) => (b.pontos ?? -1) - (a.pontos ?? -1));

  const pareto = (alvo: number) => {
    let acc = 0;
    for (let i = 0; i < linhas.length; i++) {
      acc += linhas[i].fracao ?? 0;
      if (acc >= alvo - 1e-9) return i + 1;
    }
    return linhas.length;
  };

  const totalQuestoes = linhas.reduce((n, l) => n + (l.questoes ?? 0), 0);
  const duracao = estrutura?.duracao_minutos ?? null;

  const minimos: MinimoBloco[] = blocos.flatMap((b) => {
    if (b.minimo === null) return [];
    const doBloco = linhas.filter((l) => l.bloco === b.nome);
    const pontos = doBloco.reduce((n, l) => n + (l.pontos ?? 0), 0);
    if (pontos <= 0) return [];
    return [
      {
        nome: b.nome,
        pontos,
        minimo: b.minimo,
        pontosMinimos: Math.ceil((pontos * b.minimo) / 100 - 1e-9),
        conferir: !b.verificado || doBloco.some((l) => l.conferir || l.estimado),
      },
    ];
  });

  return {
    materias: linhas,
    totalPontos,
    totalQuestoes,
    pareto50: pareto(0.5),
    pareto80: pareto(0.8),
    ritmo: duracao && totalQuestoes > 0 ? (duracao * 60) / totalQuestoes : null,
    ritmoConferir: !estrutura?.duracao_verificada,
    blocos: minimos,
    minimoGeral:
      estrutura?.minimo_geral != null
        ? {
            minimo: estrutura.minimo_geral,
            pontosMinimos: Math.ceil((totalPontos * estrutura.minimo_geral) / 100 - 1e-9),
            conferir: !estrutura.minimo_verificado,
          }
        : null,
    leis: leisCitadas(materias),
  };
}

const LEI =
  /\b(Lei Complementar|Lei|LC|Decreto-Lei|Decreto|Emenda Constitucional|EC|Instrução Normativa|IN)\s*(?:Federal\s*)?(?:n[º°o.]?\s*)?(\d{1,3}(?:\.\d{3})*(?:\/\d{2,4})?)/gi;

/** "Lei nº 14.133/2021", "LC 182/2021" etc., normalizados e sem repetição. */
export function leisCitadas(materias: Materia[]): string[] {
  const vistas = new Map<string, string>();
  const textos = materias.flatMap((m) => [m.nome, ...todosTopicos(m.topicos)]);
  for (const t of textos) {
    for (const [, tipo, numero] of t.matchAll(LEI)) {
      const nomeTipo = TIPOS[tipo.toLowerCase()] ?? tipo;
      const chave = `${nomeTipo} ${numero.split("/")[0]}`;
      if (!vistas.has(chave) || numero.includes("/")) vistas.set(chave, `${nomeTipo} ${numero}`);
    }
  }
  return [...vistas.values()];
}

function todosTopicos(ts: Materia["topicos"]): string[] {
  return ts.flatMap((t) => [t.texto, ...todosTopicos(t.filhos)]);
}

const TIPOS: Record<string, string> = {
  lei: "Lei",
  "lei complementar": "LC",
  lc: "LC",
  "decreto-lei": "Decreto-Lei",
  decreto: "Decreto",
  "emenda constitucional": "EC",
  ec: "EC",
  "instrução normativa": "IN",
  in: "IN",
};
