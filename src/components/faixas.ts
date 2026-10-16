/** [1, 2, 3, 7, 9, 10] → "1–3, 7, 9–10". Espera números em ordem crescente. */
export function faixas(paginas: number[]): string {
  const out: string[] = [];
  for (let i = 0; i < paginas.length; i++) {
    const ini = paginas[i];
    while (i + 1 < paginas.length && paginas[i + 1] === paginas[i] + 1) i++;
    out.push(ini === paginas[i] ? `${ini}` : `${ini}–${paginas[i]}`);
  }
  return out.join(", ");
}
