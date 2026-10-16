import { useEffect, type ReactNode } from "react";

/** Painel lateral sobreposto (diagnóstico, jobs). Fecha com Esc ou clique fora. */
export function Painel({ titulo, onFechar, children }: { titulo: string; onFechar: () => void; children: ReactNode }) {
  useEffect(() => {
    const h = (e: KeyboardEvent) => e.key === "Escape" && onFechar();
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [onFechar]);

  return (
    <div className="painel-fundo" onClick={onFechar}>
      <aside className="painel" onClick={(e) => e.stopPropagation()} aria-label={titulo}>
        <header>
          <h2>{titulo}</h2>
          <button className="link" onClick={onFechar} aria-label="Fechar">
            ✕
          </button>
        </header>
        <div className="painel-corpo">{children}</div>
      </aside>
    </div>
  );
}
