import { useCallback, useEffect, useState } from "react";
import "./App.css";
import type { AppError } from "./bindings/AppError";
import type { BackendStatus } from "./bindings/BackendStatus";
import type { Bootstrap } from "./bindings/Bootstrap";
import { ClaudeChecklist } from "./components/ClaudeChecklist";
import { ErrorBox } from "./components/ErrorBox";
import { JobsPanel } from "./components/JobsPanel";
import { Painel } from "./components/Painel";
import { StatusBar } from "./components/StatusBar";
import { ipc, onClaudeStatus, toAppError } from "./ipc";
import { Biblioteca } from "./screens/Biblioteca";
import { Configuracoes } from "./screens/Configuracoes";
import { EditalScreen } from "./screens/EditalScreen";
import { Setup } from "./screens/Setup";

type Tela =
  | { nome: "biblioteca" }
  | { nome: "edital"; id: string; aviso?: string }
  | { nome: "config" };

const VERIFICANDO: BackendStatus = {
  estado: "verificando",
  caminho: null,
  origem_caminho: null,
  versao: null,
  logado: null,
  metodo_login: null,
  erro: null,
  retorno_em: null,
  verificado_em: null,
};

export default function App() {
  const [boot, setBoot] = useState<Bootstrap | null>(null);
  const [erroBoot, setErroBoot] = useState<AppError | null>(null);
  const [claude, setClaude] = useState<BackendStatus>(VERIFICANDO);
  const [tela, setTela] = useState<Tela>({ nome: "biblioteca" });
  const [painel, setPainel] = useState<"diagnostico" | "jobs" | null>(null);
  const [pulouSetup, setPulouSetup] = useState(false);

  useEffect(() => {
    ipc.appBootstrap().then(setBoot, (e) => setErroBoot(toAppError(e)));
    const desliga = onClaudeStatus(setClaude);
    // O health check pode ter terminado antes de o listener existir.
    ipc.claudeStatus().then(setClaude, () => {});
    return () => {
      void desliga.then((f) => f());
    };
  }, []);

  const verificar = useCallback(() => {
    ipc.claudeVerificar().then(setClaude, () => {});
  }, []);
  const fecharPainel = useCallback(() => setPainel(null), []);
  const abrirEdital = useCallback((id: string, aviso?: string) => setTela({ nome: "edital", id, aviso }), []);
  const irBiblioteca = useCallback(() => setTela({ nome: "biblioteca" }), []);

  if (erroBoot) return <main className="tela"><ErrorBox erro={erroBoot} contexto="Inicialização" /></main>;
  if (!boot) return null;

  const emSetup =
    !boot.health_ja_passou && !pulouSetup && claude.estado === "indisponivel" && tela.nome !== "config";

  return (
    <div className="app">
      <nav className="topo">
        <span className="marca-app">Edital Killer</span>
        <button className={tela.nome !== "config" ? "ativo" : ""} onClick={irBiblioteca}>
          Biblioteca
        </button>
        <button className={tela.nome === "config" ? "ativo" : ""} onClick={() => setTela({ nome: "config" })}>
          Configurações
        </button>
      </nav>

      <div className="conteudo">
        {emSetup ? (
          <Setup
            status={claude}
            onVerificar={verificar}
            onConfiguracoes={() => setTela({ nome: "config" })}
            onPular={() => setPulouSetup(true)}
          />
        ) : tela.nome === "biblioteca" ? (
          <Biblioteca onAbrir={abrirEdital} />
        ) : tela.nome === "edital" ? (
          <EditalScreen key={tela.id} id={tela.id} aviso={tela.aviso} onVoltar={irBiblioteca} />
        ) : (
          <Configuracoes pastaDados={boot.pasta_dados} onVoltar={irBiblioteca} />
        )}
      </div>

      <StatusBar status={claude} onDiagnostico={() => setPainel("diagnostico")} onJobs={() => setPainel("jobs")} />

      {painel === "diagnostico" && (
        <Painel titulo="Diagnóstico do Claude CLI" onFechar={fecharPainel}>
          <ClaudeChecklist status={claude} />
          {claude.origem_caminho && <p className="mudo">Caminho obtido por: {claude.origem_caminho}</p>}
          {claude.verificado_em && (
            <p className="mudo">Última verificação: {new Date(claude.verificado_em).toLocaleString("pt-BR")}</p>
          )}
          <div className="acoes">
            <button className="primario" onClick={verificar} disabled={claude.estado === "verificando"}>
              Verificar de novo
            </button>
          </div>
        </Painel>
      )}
      {painel === "jobs" && <JobsPanel onFechar={fecharPainel} />}
    </div>
  );
}
