import { useCallback, useEffect, useState } from "react";
import "./App.css";
import type { AppError } from "./bindings/AppError";
import type { BackendStatus } from "./bindings/BackendStatus";
import type { Bootstrap } from "./bindings/Bootstrap";
import type { EstadoFila } from "./bindings/EstadoFila";
import type { UsoClaude } from "./bindings/UsoClaude";
import { ClaudeChecklist } from "./components/ClaudeChecklist";
import { Confirmacao } from "./components/Confirmacao";
import { ErrorBox } from "./components/ErrorBox";
import { JobsPanel } from "./components/JobsPanel";
import { Painel } from "./components/Painel";
import { StatusBar } from "./components/StatusBar";
import { TesteExecucao } from "./components/TesteExecucao";
import { ipc, onClaudeStatus, onClaudeUso, onFecharPedido, onJobsAtualizados, toAppError } from "./ipc";
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

const FILA_VAZIA: EstadoFila = { executando: null, na_fila: 0, pausada_ate: null };

export default function App() {
  const [boot, setBoot] = useState<Bootstrap | null>(null);
  const [erroBoot, setErroBoot] = useState<AppError | null>(null);
  const [claude, setClaude] = useState<BackendStatus>(VERIFICANDO);
  const [tela, setTela] = useState<Tela>({ nome: "biblioteca" });
  const [painel, setPainel] = useState<"diagnostico" | "jobs" | null>(null);
  const [pulouSetup, setPulouSetup] = useState(false);
  const [fila, setFila] = useState<EstadoFila>(FILA_VAZIA);
  const [uso, setUso] = useState<UsoClaude | null>(null);
  const [fecharPedido, setFecharPedido] = useState<EstadoFila | null>(null);

  useEffect(() => {
    ipc.appBootstrap().then(setBoot, (e) => setErroBoot(toAppError(e)));
    const desliga = [
      onClaudeStatus(setClaude),
      onJobsAtualizados(setFila),
      onClaudeUso(setUso),
      onFecharPedido(setFecharPedido),
    ];
    // Os eventos podem ter disparado antes de os listeners existirem.
    ipc.claudeStatus().then(setClaude, () => {});
    ipc.filaEstado().then(setFila, () => {});
    ipc.claudeUso().then(setUso, () => {});
    return () => desliga.forEach((d) => void d.then((f) => f()));
  }, []);

  const verificar = useCallback(() => {
    ipc.claudeVerificar().then(setClaude, () => {});
  }, []);
  const fecharPainel = useCallback(() => setPainel(null), []);
  const abrirEdital = useCallback((id: string, aviso?: string) => setTela({ nome: "edital", id, aviso }), []);
  const irBiblioteca = useCallback(() => setTela({ nome: "biblioteca" }), []);
  const ficar = useCallback(() => setFecharPedido(null), []);

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

      <StatusBar status={claude} fila={fila} uso={uso} onDiagnostico={() => setPainel("diagnostico")} onJobs={() => setPainel("jobs")} />

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
          <hr />
          <TesteExecucao disponivel={claude.estado !== "indisponivel" && claude.estado !== "verificando"} />
        </Painel>
      )}
      {painel === "jobs" && <JobsPanel fila={fila} uso={uso} onFechar={fecharPainel} />}
      {fecharPedido && (
        <Confirmacao
          titulo="Sair com job em execução?"
          confirmar="Cancelar e sair"
          cancelar="Continuar no app"
          perigo
          onConfirmar={() => void ipc.appSair()}
          onCancelar={ficar}
        >
          <p>
            {fecharPedido.na_fila > 0
              ? `Há 1 job em execução e ${fecharPedido.na_fila} na fila. Sair cancela todos.`
              : "Há 1 job em execução. Sair cancela o job e encerra o Claude Code."}
          </p>
        </Confirmacao>
      )}
    </div>
  );
}
