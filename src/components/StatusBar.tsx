import type { BackendStatus } from "../bindings/BackendStatus";

const ROTULO: Record<BackendStatus["estado"], string> = {
  verificando: "Claude: verificando…",
  pronto: "Claude: pronto",
  ocupado: "Claude: ocupado",
  limite_uso: "Claude: em limite de uso",
  indisponivel: "Claude: indisponível",
};

const COR: Record<BackendStatus["estado"], string> = {
  verificando: "cinza",
  pronto: "verde",
  ocupado: "amarelo",
  limite_uso: "amarelo",
  indisponivel: "vermelho",
};

export function StatusBar({
  status,
  onDiagnostico,
  onJobs,
}: {
  status: BackendStatus;
  onDiagnostico: () => void;
  onJobs: () => void;
}) {
  let rotulo = ROTULO[status.estado];
  if (status.estado === "limite_uso" && status.retorno_em) {
    rotulo += ` até ${new Date(status.retorno_em).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" })}`;
  }
  return (
    <footer className="statusbar">
      <button className="status-claude" onClick={onDiagnostico} title="Abrir diagnóstico do Claude CLI">
        <span className={`led ${COR[status.estado]}`} aria-hidden />
        {rotulo}
      </button>
      <button className="link" onClick={onJobs}>
        Jobs
      </button>
    </footer>
  );
}
