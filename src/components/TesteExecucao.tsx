import { useEffect, useState } from "react";
import type { AppError } from "../bindings/AppError";
import type { Job } from "../bindings/Job";
import { ipc, onJobTerminou, toAppError } from "../ipc";
import { useConfirmacaoDeUso } from "./Confirmacao";
import { ErrorBox } from "./ErrorBox";
import { consumo } from "./formato";

/** Roda um job mínimo pelo caminho real (fila, runner, consumo), para diagnóstico. */
export function TesteExecucao({ disponivel }: { disponivel: boolean }) {
  const { executar, modal } = useConfirmacaoDeUso();
  const [job, setJob] = useState<Job | null>(null);
  const [erro, setErro] = useState<AppError | null>(null);

  useEffect(() => {
    const desliga = onJobTerminou((j) => setJob((atual) => (atual && atual.id === j.id ? j : atual)));
    return () => void desliga.then((f) => f());
  }, []);

  const rodando = job?.status === "na_fila" || job?.status === "executando";
  const rodar = () => {
    setErro(null);
    executar(ipc.claudeTestarExecucao).then(
      (j) => j && setJob(j),
      (e) => setErro(toAppError(e)),
    );
  };

  return (
    <div className="teste-execucao">
      <h3>Teste de execução</h3>
      <p className="mudo">Roda um pedido mínimo ao Claude, pelo mesmo caminho dos jobs de extração.</p>
      <div className="acoes">
        <button onClick={rodar} disabled={!disponivel || rodando}>
          {rodando ? "Testando…" : "Testar execução"}
        </button>
        {job?.status === "na_fila" && <span className="mudo">Na fila.</span>}
        {job?.status === "concluido" && (
          <span>
            ✓ Funcionou{job.consumo ? ` · ${consumo(job.consumo)}` : ""}
          </span>
        )}
        {job?.status === "cancelado" && <span className="mudo">Cancelado.</span>}
      </div>
      {job?.status === "falhou" && job.erro_code && (
        <ErrorBox erro={{ code: job.erro_code, detail: job.erro_detail ?? "" }} contexto="Teste" />
      )}
      {erro && <ErrorBox erro={erro} contexto="Teste" onFechar={() => setErro(null)} />}
      {modal}
    </div>
  );
}
