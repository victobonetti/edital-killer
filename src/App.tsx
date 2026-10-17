import { useCallback, useEffect, useState } from "react";
import "./App.css";
import type { AppError } from "./bindings/AppError";
import type { BackendStatus } from "./bindings/BackendStatus";
import type { Bootstrap } from "./bindings/Bootstrap";
import { ErrorBox } from "./components/ErrorBox";
import { Icone } from "./components/Icone";
import { IndicadorClaude } from "./components/IndicadorClaude";
import { ipc, onClaudeStatus, toAppError } from "./ipc";
import { Biblioteca } from "./screens/Biblioteca";
import { Configuracoes } from "./screens/Configuracoes";
import { EditalScreen } from "./screens/EditalScreen";
import { Setup } from "./screens/Setup";

type Tela =
  | { nome: "biblioteca" }
  | { nome: "edital"; id: string; aviso?: string }
  | { nome: "ajustes" };

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
  const abrirEdital = useCallback((id: string, aviso?: string) => setTela({ nome: "edital", id, aviso }), []);
  const irBiblioteca = useCallback(() => setTela({ nome: "biblioteca" }), []);
  const irAjustes = useCallback(() => setTela({ nome: "ajustes" }), []);

  if (erroBoot) return <main className="tela"><ErrorBox erro={erroBoot} contexto="Inicialização" /></main>;
  if (!boot) return null;

  const emSetup =
    !boot.health_ja_passou && !pulouSetup && claude.estado === "indisponivel" && tela.nome !== "ajustes";

  return (
    <div className="app">
      <header className="masthead">
        <button className="marca-app" onClick={irBiblioteca} aria-label="Ir para a biblioteca">
          Edital <em>Killer</em>
        </button>
        <nav className="masthead-nav">
          <button className={tela.nome !== "ajustes" ? "ativo" : ""} onClick={irBiblioteca}>
            Editais
          </button>
        </nav>
        <div className="masthead-dir">
          <IndicadorClaude status={claude} onClick={irAjustes} />
          <button
            className={`icone-btn ${tela.nome === "ajustes" ? "ativo" : ""}`}
            onClick={irAjustes}
            aria-label="Ajustes"
            title="Ajustes"
          >
            <Icone nome="ajustes" />
          </button>
        </div>
      </header>

      <div className="conteudo">
        {emSetup ? (
          <Setup status={claude} onVerificar={verificar} onAjustes={irAjustes} onPular={() => setPulouSetup(true)} />
        ) : tela.nome === "biblioteca" ? (
          <Biblioteca onAbrir={abrirEdital} />
        ) : tela.nome === "edital" ? (
          <EditalScreen
            key={tela.id}
            id={tela.id}
            aviso={tela.aviso}
            claude={claude}
            onVoltar={irBiblioteca}
            onAjustes={irAjustes}
          />
        ) : (
          <Configuracoes
            pastaDados={boot.pasta_dados}
            versaoApp={boot.versao_app}
            claude={claude}
            onVerificar={verificar}
          />
        )}
      </div>
    </div>
  );
}
