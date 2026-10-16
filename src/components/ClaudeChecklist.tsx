import type { BackendStatus } from "../bindings/BackendStatus";
import { ErrorBox } from "./ErrorBox";

type Item = { ok: boolean | null; rotulo: string; info?: string | null };

/** Checklist do Claude CLI: instalado, versão, login. Usado no Setup e no diagnóstico. */
export function ClaudeChecklist({ status }: { status: BackendStatus }) {
  const verificando = status.estado === "verificando";
  // Cada passo só é avaliado se o anterior passou; `null` = não avaliado.
  const instalado = status.caminho !== null;
  const executa = instalado ? status.versao !== null : null;
  const logado = executa ? status.logado : null;

  const itens: Item[] = [
    { ok: instalado, rotulo: "Claude Code instalado", info: status.caminho },
    { ok: executa, rotulo: "Executa e responde", info: status.versao && `versão ${status.versao}` },
    {
      ok: logado,
      rotulo: "Conta logada",
      info: status.metodo_login ?? (executa && status.logado === null ? "não foi possível confirmar" : null),
    },
  ];

  return (
    <div className="checklist">
      <ul>
        {itens.map((it) => (
          <li key={it.rotulo} className={verificando ? "pendente" : it.ok === true ? "ok" : it.ok === false ? "falha" : "pendente"}>
            <span className="marca" aria-hidden>
              {verificando ? "…" : it.ok === true ? "✓" : it.ok === false ? "✕" : "–"}
            </span>
            <span>{it.rotulo}</span>
            {it.info && <span className="info">{it.info}</span>}
          </li>
        ))}
      </ul>
      {!verificando && status.erro && <ErrorBox erro={status.erro} />}
    </div>
  );
}
