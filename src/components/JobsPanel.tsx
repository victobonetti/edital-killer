import { useCallback, useEffect, useState } from "react";
import type { AppError } from "../bindings/AppError";
import type { EstadoFila } from "../bindings/EstadoFila";
import type { Job } from "../bindings/Job";
import type { JobStatus } from "../bindings/JobStatus";
import type { UsoClaude } from "../bindings/UsoClaude";
import { ipc, onJobsAtualizados, onJobTerminou, toAppError } from "../ipc";
import { CATALOGO } from "../ipc/errors";
import { ErrorBox } from "./ErrorBox";
import { consumo, duracao, horaOuDia, janela } from "./formato";
import { Painel } from "./Painel";

const STATUS: Record<JobStatus, string> = {
  na_fila: "Na fila",
  executando: "Executando",
  concluido: "Concluído",
  falhou: "Falhou",
  cancelado: "Cancelado",
  interrompido: "Interrompido",
};

const TIPO: Record<string, string> = {
  diagnostico: "Teste de execução",
  localizador: "Localizar cargos",
  extrator: "Extrair conteúdo",
};

export function JobsPanel({
  fila,
  uso,
  onFechar,
}: {
  fila: EstadoFila;
  uso: UsoClaude | null;
  onFechar: () => void;
}) {
  const [jobs, setJobs] = useState<Job[] | null>(null);
  const [erro, setErro] = useState<AppError | null>(null);
  const [agora, setAgora] = useState(Date.now());

  const carregar = useCallback(() => {
    ipc.jobsListar().then(setJobs, (e) => setErro(toAppError(e)));
  }, []);

  useEffect(() => {
    carregar();
    const desliga = [onJobsAtualizados(carregar), onJobTerminou(carregar)];
    return () => desliga.forEach((d) => void d.then((f) => f()));
  }, [carregar]);

  // Relógio do tempo decorrido, só enquanto há job rodando.
  useEffect(() => {
    if (!fila.executando) return;
    const t = setInterval(() => setAgora(Date.now()), 1000);
    return () => clearInterval(t);
  }, [fila.executando]);

  const cancelar = (id: string) => ipc.jobCancelar(id).then(carregar, (e) => setErro(toAppError(e)));
  const retomar = () => ipc.filaRetomar().catch((e) => setErro(toAppError(e)));

  const ativos = jobs?.filter((j) => j.status === "na_fila" || j.status === "executando") ?? [];
  const historico = jobs?.filter((j) => j.status !== "na_fila" && j.status !== "executando") ?? [];
  const janelas = uso && [janela("Janela de 5 h", uso.janela_5h), janela("7 dias", uso.janela_7d)].filter(Boolean);

  return (
    <Painel titulo="Jobs" onFechar={onFechar}>
      {erro && <ErrorBox erro={erro} onFechar={() => setErro(null)} />}
      {fila.pausada_ate && (
        <div className="aviso">
          <span>
            <strong>Fila pausada: limite de uso do Claude.</strong> Retoma sozinha às {horaOuDia(fila.pausada_ate)}.
          </span>
          <button onClick={retomar}>Retomar agora</button>
        </div>
      )}
      {janelas && janelas.length > 0 && (
        <p className="mudo uso-linha" title={`Status do plano: ${uso.status}`}>
          {janelas.join(" · ")}
        </p>
      )}
      {jobs && (
        <>
          <h3>Fila</h3>
          {ativos.length === 0 ? (
            <p className="mudo">Nenhum job na fila.</p>
          ) : (
            <ListaJobs jobs={ativos} agora={agora} onCancelar={cancelar} />
          )}
          <h3>Histórico</h3>
          {historico.length === 0 ? (
            <p className="mudo">Nenhum job executado ainda.</p>
          ) : (
            <ListaJobs jobs={historico} agora={agora} />
          )}
        </>
      )}
    </Painel>
  );
}

function ListaJobs({ jobs, agora, onCancelar }: { jobs: Job[]; agora: number; onCancelar?: (id: string) => void }) {
  return (
    <ul className="lista-jobs">
      {jobs.map((j) => (
        <li key={j.id}>
          <span className="job-tipo">{TIPO[j.tipo] ?? j.tipo}</span>
          <span className={`tag ${j.status}`}>{STATUS[j.status]}</span>
          <span className="mudo">
            {j.status === "executando" && j.iniciado_em
              ? duracao(agora - new Date(j.iniciado_em).getTime())
              : horaOuDia(j.criado_em)}
          </span>
          {onCancelar && (
            <button className="link job-cancelar" onClick={() => onCancelar(j.id)}>
              Cancelar
            </button>
          )}
          {j.consumo && (
            <span className="job-consumo mudo" title="Custo em preço de tabela da API; com login do claude.ai não é cobrado">
              {consumo(j.consumo)}
              {j.tentativas > 1 && ` · ${j.tentativas} tentativas`}
            </span>
          )}
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
