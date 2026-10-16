import { useEffect, useState } from "react";
import type { AppError } from "../bindings/AppError";
import type { Job } from "../bindings/Job";
import type { JobStatus } from "../bindings/JobStatus";
import { ipc } from "../ipc";
import { CATALOGO } from "../ipc/errors";
import { ErrorBox } from "./ErrorBox";
import { Painel } from "./Painel";

const STATUS: Record<JobStatus, string> = {
  na_fila: "Na fila",
  executando: "Executando",
  concluido: "Concluído",
  falhou: "Falhou",
  cancelado: "Cancelado",
  interrompido: "Interrompido",
};

export function JobsPanel({ onFechar }: { onFechar: () => void }) {
  const [jobs, setJobs] = useState<Job[] | null>(null);
  const [erro, setErro] = useState<AppError | null>(null);

  useEffect(() => {
    ipc.jobsListar().then(setJobs, setErro);
  }, []);

  const fila = jobs?.filter((j) => j.status === "na_fila" || j.status === "executando") ?? [];
  const historico = jobs?.filter((j) => j.status !== "na_fila" && j.status !== "executando") ?? [];

  return (
    <Painel titulo="Jobs" onFechar={onFechar}>
      {erro && <ErrorBox erro={erro} />}
      {jobs && (
        <>
          <h3>Fila</h3>
          {fila.length === 0 ? <p className="mudo">Nenhum job na fila.</p> : <ListaJobs jobs={fila} />}
          <h3>Histórico</h3>
          {historico.length === 0 ? <p className="mudo">Nenhum job executado ainda.</p> : <ListaJobs jobs={historico} />}
        </>
      )}
    </Painel>
  );
}

function ListaJobs({ jobs }: { jobs: Job[] }) {
  return (
    <ul className="lista-jobs">
      {jobs.map((j) => (
        <li key={j.id}>
          <span className="job-tipo">{j.tipo}</span>
          <span className={`tag ${j.status}`}>{STATUS[j.status]}</span>
          <span className="mudo">{new Date(j.criado_em).toLocaleString("pt-BR")}</span>
          {j.erro_code && <span className="job-erro">{CATALOGO[j.erro_code]?.titulo ?? j.erro_code}</span>}
        </li>
      ))}
    </ul>
  );
}
