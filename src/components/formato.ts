// Formatação compartilhada (pt-BR) de horários, durações, tokens e uso.
import type { ConsumoJob } from "../bindings/ConsumoJob";
import type { JanelaUso } from "../bindings/JanelaUso";

export function hora(iso: string): string {
  return new Date(iso).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
}

/** "18:40" hoje; "sex 18:40" em outro dia. */
export function horaOuDia(iso: string): string {
  const d = new Date(iso);
  if (d.toDateString() === new Date().toDateString()) return hora(iso);
  return d.toLocaleString("pt-BR", { weekday: "short", hour: "2-digit", minute: "2-digit" });
}

export function duracao(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  return m < 60 ? `${m}min ${s % 60}s` : `${Math.floor(m / 60)}h ${m % 60}min`;
}

export function tokens(n: number): string {
  if (n < 1000) return String(n);
  return `${(n / 1000).toLocaleString("pt-BR", { maximumFractionDigits: 1 })} mil`;
}

export function percentual(u: number): string {
  return `${Math.round(u * 100)}%`;
}

export function janela(nome: string, j: JanelaUso | null): string | null {
  if (!j) return null;
  return `${nome}: ${percentual(j.utilizacao)}${j.renova_em ? ` (renova ${horaOuDia(j.renova_em)})` : ""}`;
}

export function consumo(c: ConsumoJob): string {
  const partes = [`${tokens(c.tokens_entrada + c.tokens_cache_escrita + c.tokens_cache_leitura)} entrada`];
  partes.push(`${tokens(c.tokens_saida)} saída`);
  partes.push(`US$ ${c.custo_estimado_usd.toLocaleString("pt-BR", { maximumFractionDigits: 4 })} estimado`);
  return partes.join(" · ");
}
