import type { AppError } from "../bindings/AppError";
import { CATALOGO } from "../ipc/errors";

export function ErrorBox({ erro, contexto, onFechar }: { erro: AppError; contexto?: string; onFechar?: () => void }) {
  const e = CATALOGO[erro.code] ?? CATALOGO.INTERNAL;
  return (
    <div className="erro" role="alert">
      <div className="erro-topo">
        <strong>{contexto ? `${contexto}: ${e.titulo}` : e.titulo}</strong>
        {onFechar && (
          <button className="link" onClick={onFechar} aria-label="Fechar">
            ✕
          </button>
        )}
      </div>
      <p>{e.explicacao}</p>
      {e.acao && <p className="erro-acao">{e.acao}</p>}
      {erro.detail && (
        <details>
          <summary>Detalhes técnicos</summary>
          <code>
            {erro.code}: {erro.detail}
          </code>
        </details>
      )}
    </div>
  );
}
