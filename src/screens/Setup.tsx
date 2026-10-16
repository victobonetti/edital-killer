import type { BackendStatus } from "../bindings/BackendStatus";
import { ClaudeChecklist } from "../components/ClaudeChecklist";

export function Setup({
  status,
  onVerificar,
  onConfiguracoes,
  onPular,
}: {
  status: BackendStatus;
  onVerificar: () => void;
  onConfiguracoes: () => void;
  onPular: () => void;
}) {
  return (
    <main className="tela setup">
      <h1>Bem-vindo ao Edital Killer</h1>
      <p>
        A parte de IA usa o <strong>Claude Code</strong> instalado e logado nesta máquina. O app nunca lê nem guarda
        suas credenciais: ele só chama o <code>claude</code> que você já usa.
      </p>
      <ClaudeChecklist status={status} />
      <div className="acoes">
        <button className="primario" onClick={onVerificar} disabled={status.estado === "verificando"}>
          Verificar de novo
        </button>
        <button onClick={onConfiguracoes}>Informar caminho do claude</button>
        <button className="link" onClick={onPular}>
          Continuar sem o Claude
        </button>
      </div>
      <p className="mudo">Sem o Claude você ainda pode importar e ler editais; só o processamento fica bloqueado.</p>
    </main>
  );
}
