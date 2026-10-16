import { useEffect, useRef, useState } from "react";
import type { AppError } from "../../bindings/AppError";
import { toAppError } from "../../ipc";
import { ErrorBox } from "../ErrorBox";
import { abrirPdfDoEdital, type PDFDocumentProxy } from "./pdfjs";

/** Visualizador do PDF do edital. Páginas são renderizadas quando entram na tela. */
export function PdfViewer({ editalId }: { editalId: string }) {
  const [doc, setDoc] = useState<PDFDocumentProxy | null>(null);
  const [erro, setErro] = useState<AppError | null>(null);

  useEffect(() => {
    let vivo = true;
    let aberto: PDFDocumentProxy | null = null;
    setDoc(null);
    setErro(null);
    abrirPdfDoEdital(editalId).then(
      (d) => {
        aberto = d;
        if (vivo) setDoc(d);
        else void d.loadingTask.destroy();
      },
      (e) => vivo && setErro(toAppError(e)),
    );
    return () => {
      vivo = false;
      void aberto?.loadingTask.destroy();
    };
  }, [editalId]);

  if (erro) return <ErrorBox erro={erro} contexto="Visualizador" />;
  if (!doc) return <p className="mudo">Carregando PDF…</p>;
  return (
    <div className="pdf-viewer">
      {Array.from({ length: doc.numPages }, (_, i) => (
        <Pagina key={i + 1} doc={doc} numero={i + 1} />
      ))}
    </div>
  );
}

const ESCALA = 1.4;

function Pagina({ doc, numero }: { doc: PDFDocumentProxy; numero: number }) {
  const wrap = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [visivel, setVisivel] = useState(false);
  const [proporcao, setProporcao] = useState(1.414);

  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const obs = new IntersectionObserver(([e]) => e.isIntersecting && setVisivel(true), { rootMargin: "600px" });
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  useEffect(() => {
    if (!visivel || !canvas.current) return;
    let tarefa: { cancel: () => void } | null = null;
    let cancelado = false;
    doc.getPage(numero).then((page) => {
      if (cancelado || !canvas.current) return;
      const dpr = window.devicePixelRatio || 1;
      const vp = page.getViewport({ scale: ESCALA * dpr });
      const c = canvas.current;
      c.width = vp.width;
      c.height = vp.height;
      setProporcao(vp.height / vp.width);
      const t = page.render({ canvas: c, viewport: vp });
      tarefa = t;
      t.promise.catch(() => {
        /* cancelado ao desmontar */
      });
    });
    return () => {
      cancelado = true;
      tarefa?.cancel();
    };
  }, [visivel, doc, numero]);

  return (
    <div ref={wrap} className="pdf-pagina" style={{ aspectRatio: `1 / ${proporcao}` }} data-pagina={numero}>
      <canvas ref={canvas} />
      <span className="pdf-num">{numero}</span>
    </div>
  );
}
