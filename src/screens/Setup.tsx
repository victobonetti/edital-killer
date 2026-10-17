import type { BackendStatus } from "../bindings/BackendStatus";
import { ClaudeChecklist } from "../components/ClaudeChecklist";

export function Setup({
  status,
  onVerificar,
  onAjustes,
  onPular,
}: {
  status: BackendStatus;
  onVerificar: () => void;
  onAjustes: () => void;
  onPular: () => void;
}) {
  return (
    <main className="tela setup">
      <p className="kicker">Primeiro uso</p>
      <h1 className="titulo-display">
        Do edital ao plano de estudos, <em>sem ler 80 páginas.</em>
      </h1>
      <p className="setup-lede">
        O Edital Killer usa o <strong>Claude</strong> da sua própria conta para ler o edital e organizar o que cai na sua
        prova. Para isso, o Claude Code precisa estar instalado e conectado neste computador. O app nunca vê nem guarda a
        sua senha.
      </p>
      <ClaudeChecklist status={status} />
      <div className="acoes">
        <button className="primario" onClick={onVerificar} disabled={status.estado === "verificando"}>
          Verificar de novo
        </button>
        <button onClick={onAjustes}>Já instalei, mas não foi encontrado</button>
      </div>
      <button className="link" onClick={onPular}>
        Continuar sem o Claude por enquanto →
      </button>
      <p className="mudo">Você ainda pode importar e ler editais; só a análise fica esperando.</p>
    </main>
  );
}
