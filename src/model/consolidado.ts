// Visão consolidada de um edital: o que a tela mostra ao usuário, sem jobs nem detalhes de processamento.
// Junta o `Edital` com a `Analise` (cargos, conteúdo e andamento) vindos do core.
import type { Analise } from "../bindings/Analise";
import type { Andamento } from "../bindings/Andamento";
import type { Cargo } from "../bindings/Cargo";
import type { Edital } from "../bindings/Edital";
import type { Materia } from "../bindings/Materia";
import type { Topico } from "../bindings/Topico";

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
  /** Análise na fila ou executando agora. */
  rodando: Andamento | null;
  /** Última análise falhou ou foi interrompida (cancelamento pelo usuário não conta). */
  falha: Andamento | null;
  cargos: Cargo[] | null;
  cargo: Cargo | null;
  materias: Materia[] | null;
  totalTopicos: number | null;
  topicosConferir: number;
};

export function contarTopicos(ts: Topico[], filtro: (t: Topico) => boolean = () => true): number {
  return ts.reduce((n, t) => n + (filtro(t) ? 1 : 0) + contarTopicos(t.filhos, filtro), 0);
}

export function consolidar(edital: Edital, a: Analise | null): Consolidado {
  const cargos = a?.cargos.length ? a.cargos : null;
  const cargo = cargos?.find((c) => c.id === a?.cargo_escolhido) ?? null;
  const materias = cargo ? (a?.materias ?? null) : null;
  const and = a?.andamento ?? null;
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
  const conferir = (t: Topico) => !t.verificado;
  return {
    etapa,
    rodando: and && (and.status === "na_fila" || and.status === "executando") ? and : null,
    falha: and && (and.status === "falhou" || and.status === "interrompido") ? and : null,
    cargos,
    cargo,
    materias,
    totalTopicos: materias ? materias.reduce((n, m) => n + contarTopicos(m.topicos), 0) : null,
    topicosConferir: materias ? materias.reduce((n, m) => n + contarTopicos(m.topicos, conferir), 0) : 0,
  };
}
