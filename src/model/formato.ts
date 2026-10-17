const rtf = new Intl.RelativeTimeFormat("pt-BR", { numeric: "auto" });

/** "há 2 dias", "ontem", "hoje" — datas exatas só em detalhes. */
export function quando(iso: string): string {
  const d = new Date(iso);
  const dias = Math.round((d.getTime() - Date.now()) / 86_400_000);
  if (dias === 0) return "hoje";
  if (Math.abs(dias) < 30) return rtf.format(dias, "day");
  return d.toLocaleDateString("pt-BR", { day: "numeric", month: "short", year: "numeric" });
}

export function plural(n: number, um: string, varios: string): string {
  return `${n.toLocaleString("pt-BR")} ${n === 1 ? um : varios}`;
}

/** [3,4,5,9] → "3–5 e 9" */
export function faixas(paginas: number[]): string {
  const out: string[] = [];
  for (let i = 0; i < paginas.length; i++) {
    const ini = paginas[i];
    while (i + 1 < paginas.length && paginas[i + 1] === paginas[i] + 1) i++;
    out.push(ini === paginas[i] ? `${ini}` : `${ini}–${paginas[i]}`);
  }
  return out.length > 1 ? `${out.slice(0, -1).join(", ")} e ${out[out.length - 1]}` : (out[0] ?? "");
}
