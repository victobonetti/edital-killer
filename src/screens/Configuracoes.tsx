import { useEffect, useState } from "react";
import type { AppError } from "../bindings/AppError";
import type { Settings } from "../bindings/Settings";
import { ErrorBox } from "../components/ErrorBox";
import { ipc, toAppError } from "../ipc";

export function Configuracoes({ pastaDados, onVoltar }: { pastaDados: string; onVoltar: () => void }) {
  const [s, setS] = useState<Settings | null>(null);
  const [erro, setErro] = useState<AppError | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  useEffect(() => {
    ipc.settingsObter().then(setS, (e) => setErro(toAppError(e)));
  }, []);

  async function acao(f: () => Promise<unknown>, sucesso?: (r: unknown) => string) {
    setErro(null);
    setMsg(null);
    try {
      const r = await f();
      if (sucesso) setMsg(sucesso(r));
    } catch (e) {
      setErro(toAppError(e));
    }
  }

  if (!s) return <main className="tela">{erro ? <ErrorBox erro={erro} /> : <p className="mudo">Carregando…</p>}</main>;

  return (
    <main className="tela config">
      <button className="link" onClick={onVoltar}>
        ← Voltar
      </button>
      <h1>Configurações</h1>
      {erro && <ErrorBox erro={erro} onFechar={() => setErro(null)} />}
      {msg && <div className="aviso">{msg}</div>}

      <form
        className="form"
        onSubmit={(ev) => {
          ev.preventDefault();
          void acao(
            async () => setS(await ipc.settingsSalvar(s)),
            () => "Configurações salvas.",
          );
        }}
      >
        <h2>Claude Code</h2>
        <label>
          Caminho do <code>claude</code>
          <input
            value={s.claude_path ?? ""}
            placeholder="Automático (PATH e locais de instalação conhecidos)"
            onChange={(e) => setS({ ...s, claude_path: e.target.value || null })}
          />
        </label>
        <label>
          Modelo
          <input
            value={s.model ?? ""}
            placeholder="Padrão do Claude Code"
            onChange={(e) => setS({ ...s, model: e.target.value || null })}
          />
        </label>
        <label>
          Tempo limite por job (segundos)
          <input
            type="number"
            min={30}
            max={7200}
            value={s.timeout_secs}
            onChange={(e) => setS({ ...s, timeout_secs: Number(e.target.value) })}
          />
        </label>
        <div className="acoes">
          <button className="primario" type="submit">
            Salvar
          </button>
        </div>
      </form>

      <section className="form">
        <h2>Dados</h2>
        <p>
          Pasta de dados: <code>{pastaDados}</code>
        </p>
        <div className="acoes">
          <button onClick={() => acao(ipc.abrirPastaDados)}>Abrir pasta de dados</button>
          <button onClick={() => acao(ipc.abrirPastaLogs)}>Abrir logs</button>
          <button
            onClick={() =>
              window.confirm("Apagar os workspaces de jobs já finalizados? Eles só servem para auditoria.") &&
              acao(ipc.limparWorkspacesJobs, (n) => `${n} workspace(s) de jobs apagados.`)
            }
          >
            Limpar workspaces de jobs
          </button>
        </div>
      </section>
    </main>
  );
}
