import type { BackendStatus } from "../bindings/BackendStatus";
import type { EstadoFila } from "../bindings/EstadoFila";
import type { UsoClaude } from "../bindings/UsoClaude";
import { hora, horaOuDia, percentual } from "./formato";

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

const ALTA = 0.8;

export function StatusBar({
  status,
  fila,
  uso,
  onDiagnostico,
  onJobs,
}: {
  status: BackendStatus;
  fila: EstadoFila;
  uso: UsoClaude | null;
  onDiagnostico: () => void;
  onJobs: () => void;
}) {
  let rotulo = ROTULO[status.estado];
  if (status.estado === "limite_uso" && status.retorno_em) {
    rotulo += ` · volta às ${horaOuDia(status.retorno_em)}`;
  }
  const pendentes = fila.na_fila + (fila.executando ? 1 : 0);
  const janelas = uso
    ? ([
        ["5 h", uso.janela_5h],
        ["7 d", uso.janela_7d],
      ] as const).filter(([, j]) => j)
    : [];

  return (
    <footer className="statusbar">
      <button className="status-claude" onClick={onDiagnostico} title="Abrir diagnóstico do Claude CLI">
        <span className={`led ${COR[status.estado]}`} aria-hidden />
        {rotulo}
      </button>
      <span className="status-direita">
        {janelas.length > 0 && (
          <span className="uso" title={uso ? `Uso do plano, visto às ${hora(uso.atualizado_em)}` : undefined}>
            {janelas.map(([nome, j]) => (
              <span key={nome} className={j!.utilizacao >= ALTA ? "uso-alto" : undefined}>
                {nome} {percentual(j!.utilizacao)}
              </span>
            ))}
          </span>
        )}
        <button className="link" onClick={onJobs}>
          Jobs{pendentes > 0 ? ` (${pendentes})` : ""}
        </button>
      </span>
    </footer>
  );
}
