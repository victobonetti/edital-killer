import { useEffect, useState } from "react";
import type { AppError } from "../bindings/AppError";
import type { BackendStatus } from "../bindings/BackendStatus";
import type { Job } from "../bindings/Job";
import type { JobStatus } from "../bindings/JobStatus";
import type { Settings } from "../bindings/Settings";
import { ClaudeChecklist } from "../components/ClaudeChecklist";
import { ErrorBox } from "../components/ErrorBox";
import { Icone } from "../components/Icone";
import { rotuloClaude } from "../components/IndicadorClaude";
import { ipc, toAppError } from "../ipc";
import { CATALOGO } from "../ipc/errors";

/** Ajustes: o essencial em linguagem simples; caminho, modelo, timeout e histórico ficam em "Avançado". */
export function Configuracoes({
  pastaDados,
  versaoApp,
  claude,
  onVerificar,
}: {
  pastaDados: string;
  versaoApp: string;
  claude: BackendStatus;
  onVerificar: () => void;
}) {
  const [erro, setErro] = useState<AppError | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

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

  const { texto, cor } = rotuloClaude(claude);
  const pronto = claude.estado === "pronto" || claude.estado === "ocupado";

  return (
    <main className="tela ajustes">
      <header className="tela-cabeca">
        <div>
          <p className="kicker">Ajustes</p>
          <h1 className="titulo-display">Como o app funciona aqui</h1>
        </div>
      </header>
      {erro && <ErrorBox erro={erro} onFechar={() => setErro(null)} />}
      {msg && (
        <div className="nota ok">
          <Icone nome="check" /> <span>{msg}</span>
        </div>
      )}

      <section className="cartao-ajuste">
        <div className="ajuste-linha">
          <span className={`led grande ${cor}`} aria-hidden />
          <div>
            <h2>{texto}</h2>
            <p className="mudo">
              {pronto
                ? `A análise dos editais usa a sua conta${claude.metodo_login ? ` (${claude.metodo_login})` : ""}. O app nunca vê nem guarda sua senha.`
                : "A análise dos editais usa o Claude Code instalado neste computador, com a sua conta. Sem ele, você ainda pode importar e ler editais."}
            </p>
          </div>
          <button onClick={onVerificar} disabled={claude.estado === "verificando"}>
            Verificar de novo
          </button>
        </div>
        {!pronto && claude.estado !== "verificando" && <ClaudeChecklist status={claude} />}
      </section>

      <section className="cartao-ajuste">
        <div className="ajuste-linha">
          <Icone nome="pasta" className="grande" />
          <div>
            <h2>Seus dados ficam neste computador</h2>
            <p className="mudo">Os PDFs e tudo o que for extraído deles. Nada é enviado para servidores do app.</p>
          </div>
          <button onClick={() => acao(ipc.abrirPastaDados)}>Abrir pasta</button>
        </div>
      </section>

      <details className="avancado">
        <summary>
          <Icone nome="chevron" /> Avançado
          <span className="mudo">Caminho do Claude, modelo, tempo limite, histórico e logs</span>
        </summary>
        <Avancado pastaDados={pastaDados} claude={claude} acao={acao} />
      </details>

      <p className="mudo rodape-versao">Edital Killer {versaoApp}</p>
    </main>
  );
}

function Avancado({
  pastaDados,
  claude,
  acao,
}: {
  pastaDados: string;
  claude: BackendStatus;
  acao: (f: () => Promise<unknown>, sucesso?: (r: unknown) => string) => Promise<void>;
}) {
  const [s, setS] = useState<Settings | null>(null);
  const [jobs, setJobs] = useState<Job[] | null>(null);

  useEffect(() => {
    void acao(async () => setS(await ipc.settingsObter()));
    ipc.jobsListar().then(setJobs, () => setJobs([]));
    // Só na abertura do painel; `acao` muda a cada render do pai.
  }, []);

  return (
    <div className="avancado-corpo">
      {s && (
        <form
          className="form"
          onSubmit={(ev) => {
            ev.preventDefault();
            void acao(
              async () => setS(await ipc.settingsSalvar(s)),
              () => "Ajustes salvos.",
            );
          }}
        >
          <h3>Claude Code</h3>
          <label>
            Caminho do executável
            <input
              value={s.claude_path ?? ""}
              placeholder="Automático"
              onChange={(e) => setS({ ...s, claude_path: e.target.value || null })}
            />
            <small className="mudo">Deixe em branco para o app encontrar sozinho.</small>
          </label>
          <label>
            Modelo
            <input
              value={s.model ?? ""}
              placeholder="sonnet (padrão)"
              onChange={(e) => setS({ ...s, model: e.target.value || null })}
            />
            <small className="mudo">Usado nas análises de edital. Aceita apelidos como sonnet ou opus.</small>
          </label>
          <label>
            Tempo limite por análise (segundos)
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
      )}

      <div className="form">
        <h3>Diagnóstico</h3>
        <ClaudeChecklist status={claude} detalhado />
        <dl className="meta">
          {claude.origem_caminho && (
            <>
              <dt>Encontrado por</dt>
              <dd>{claude.origem_caminho}</dd>
            </>
          )}
          {claude.verificado_em && (
            <>
              <dt>Última verificação</dt>
              <dd>{new Date(claude.verificado_em).toLocaleString("pt-BR")}</dd>
            </>
          )}
          <dt>Pasta de dados</dt>
          <dd>
            <code>{pastaDados}</code>
          </dd>
        </dl>
      </div>

      <div className="form">
        <h3>Histórico de processamento</h3>
        {jobs === null ? (
          <p className="mudo">Carregando…</p>
        ) : jobs.length === 0 ? (
          <p className="mudo">Nenhuma análise executada ainda.</p>
        ) : (
          <ListaJobs jobs={jobs} />
        )}
        <div className="acoes">
          <button onClick={() => acao(ipc.abrirPastaLogs)}>Abrir logs</button>
          <button
            onClick={() =>
              window.confirm("Apagar os arquivos de trabalho das análises já finalizadas? Eles só servem para auditoria.") &&
              acao(ipc.limparWorkspacesJobs, (n) => `${n} pasta(s) de trabalho apagadas.`)
            }
          >
            Limpar arquivos de trabalho
          </button>
        </div>
      </div>
    </div>
  );
}

const STATUS: Record<JobStatus, string> = {
  na_fila: "Na fila",
  executando: "Executando",
  concluido: "Concluído",
  falhou: "Falhou",
  cancelado: "Cancelado",
  interrompido: "Interrompido",
};

function ListaJobs({ jobs }: { jobs: Job[] }) {
  return (
    <ul className="lista-jobs">
      {jobs.map((j) => (
        <li key={j.id}>
          <code>{j.tipo}</code>
          <span className={`tag ${j.status}`}>{STATUS[j.status]}</span>
          <span className="mudo">{new Date(j.criado_em).toLocaleString("pt-BR")}</span>
          {j.erro_code && (
            <span className="job-erro" title={j.erro_detail ?? undefined}>
              {CATALOGO[j.erro_code]?.titulo ?? j.erro_code}
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}
