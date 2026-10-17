// Visão consolidada de um edital: o que a tela mostra ao usuário, sem jobs nem detalhes de processamento.
// Junta o `Edital` (core) com o conteúdo extraído, quando houver. A extração (Item 4) ainda não existe;
// quando chegar, `obterExtraido` passa a chamar o core e estes tipos viram bindings gerados.
import type { Edital } from "../bindings/Edital";

export type Topico = { id: string; texto: string; pagina: number | null; filhos: Topico[] };
export type Materia = { id: string; nome: string; topicos: Topico[] };
export type Cargo = { id: string; nome: string; vagas: number | null; escolaridade: string | null };

export type Extraido = {
  cargos: Cargo[];
  /** Cargo que o usuário confirmou como o dele. */
  cargo_escolhido: string | null;
  /** Conteúdo programático do cargo escolhido. `null` = ainda não extraído. */
  materias: Materia[] | null;
};

/** Em que ponto da jornada o edital está. Cada etapa tem um único próximo passo. */
export type Etapa =
  | "importando"
  | "sem_texto"
  | "localizar_cargos"
  | "escolher_cargo"
  | "extrair_conteudo"
  | "pronto";

/** Situação do edital numa frase curta, como aparece na biblioteca. */
export const ETAPA_ROTULO: Record<Etapa, string> = {
  importando: "Importação interrompida",
  sem_texto: "PDF sem texto",
  localizar_cargos: "Pronto para análise",
  escolher_cargo: "Escolha seu cargo",
  extrair_conteudo: "Conteúdo a extrair",
  pronto: "Conteúdo consolidado",
};

export type Consolidado = {
  etapa: Etapa;
  cargos: Cargo[] | null;
  cargo: Cargo | null;
  materias: Materia[] | null;
  totalTopicos: number | null;
};

export function contarTopicos(ts: Topico[]): number {
  return ts.reduce((n, t) => n + 1 + contarTopicos(t.filhos), 0);
}

export function consolidar(edital: Edital, ex: Extraido | null): Consolidado {
  const cargos = ex?.cargos.length ? ex.cargos : null;
  const cargo = cargos?.find((c) => c.id === ex?.cargo_escolhido) ?? null;
  const materias = cargo ? (ex?.materias ?? null) : null;
  const etapa: Etapa =
    edital.status === "importando"
      ? "importando"
      : edital.status === "sem_texto"
        ? "sem_texto"
        : !cargos
          ? "localizar_cargos"
          : !cargo
            ? "escolher_cargo"
            : !materias
              ? "extrair_conteudo"
              : "pronto";
  return {
    etapa,
    cargos,
    cargo,
    materias,
    totalTopicos: materias ? materias.reduce((n, m) => n + contarTopicos(m.topicos), 0) : null,
  };
}

declare global {
  interface Window {
    /** Só em desenvolvimento no navegador: conteúdo de demonstração por edital (ver `dev/mockIpc.ts`). */
    __ekDemoExtraido?: Record<string, Extraido>;
  }
}

/** Conteúdo extraído do edital. Ainda não há extração no core: em produção devolve sempre `null`. */
export async function obterExtraido(editalId: string): Promise<Extraido | null> {
  return window.__ekDemoExtraido?.[editalId] ?? null;
}
