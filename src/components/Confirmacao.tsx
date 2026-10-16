import { useEffect, useState, type ReactNode } from "react";
import { toAppError } from "../ipc";
import { CATALOGO } from "../ipc/errors";

/** Diálogo modal de confirmação. Esc cancela. */
export function Confirmacao({
  titulo,
  children,
  confirmar,
  cancelar = "Cancelar",
  perigo,
  onConfirmar,
  onCancelar,
}: {
  titulo: string;
  children: ReactNode;
  confirmar: string;
  cancelar?: string;
  perigo?: boolean;
  onConfirmar: () => void;
  onCancelar: () => void;
}) {
  useEffect(() => {
    const h = (e: KeyboardEvent) => e.key === "Escape" && onCancelar();
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [onCancelar]);

  return (
    <div className="modal-fundo">
      <div className="modal" role="alertdialog" aria-modal="true" aria-label={titulo}>
        <h2>{titulo}</h2>
        <div className="modal-corpo">{children}</div>
        <div className="acoes modal-acoes">
          <button onClick={onCancelar}>{cancelar}</button>
          <button className={perigo ? "perigo" : "primario"} onClick={onConfirmar} autoFocus>
            {confirmar}
          </button>
        </div>
      </div>
    </div>
  );
}

type Pendente = { detalhe: string; responder: (ok: boolean) => void };

/**
 * Enfileirar com uso alto falha com `CLAUDE_USO_ALTO`; aqui o usuário confirma e a
 * chamada é refeita com `confirmado = true`. Devolve `null` se o usuário desistir.
 */
export function useConfirmacaoDeUso() {
  const [pendente, setPendente] = useState<Pendente | null>(null);

  async function executar<T>(chamada: (confirmado: boolean) => Promise<T>): Promise<T | null> {
    try {
      return await chamada(false);
    } catch (e) {
      const erro = toAppError(e);
      if (erro.code !== "CLAUDE_USO_ALTO") throw erro;
      const ok = await new Promise<boolean>((responder) => setPendente({ detalhe: erro.detail, responder }));
      setPendente(null);
      return ok ? chamada(true) : null;
    }
  }

  const modal = pendente && (
    <Confirmacao
      titulo={CATALOGO.CLAUDE_USO_ALTO.titulo}
      confirmar="Iniciar mesmo assim"
      onConfirmar={() => pendente.responder(true)}
      onCancelar={() => pendente.responder(false)}
    >
      <p>Sua {pendente.detalhe}.</p>
      <p className="mudo">
        Um job grande pode esgotar a janela no meio. Se isso acontecer, ele volta para a fila e retoma sozinho
        quando o limite renovar.
      </p>
    </Confirmacao>
  );

  return { executar, modal };
}
