import type { BackendStatus } from "../bindings/BackendStatus";

/** Estado do Claude em linguagem de usuário. O diagnóstico técnico fica em Ajustes. */
export function rotuloClaude(status: BackendStatus): { texto: string; cor: string } {
  switch (status.estado) {
    case "verificando":
      return { texto: "Conectando ao Claude…", cor: "cinza" };
    case "pronto":
      return { texto: "Claude conectado", cor: "verde" };
    case "ocupado":
      return { texto: "Claude trabalhando", cor: "amarelo" };
    case "limite_uso": {
      const ate = status.retorno_em
        ? ` até ${new Date(status.retorno_em).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" })}`
        : "";
      return { texto: `Claude em pausa${ate}`, cor: "amarelo" };
    }
    case "indisponivel":
      return { texto: "Claude desconectado", cor: "vermelho" };
  }
}

export function IndicadorClaude({ status, onClick }: { status: BackendStatus; onClick: () => void }) {
  const { texto, cor } = rotuloClaude(status);
  return (
    <button className={`indicador ${cor}`} onClick={onClick} title="Ver conexão com o Claude">
      <span className={`led ${cor}`} aria-hidden />
      {texto}
    </button>
  );
}
