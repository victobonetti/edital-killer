import { useEffect, useState } from "react";
import type { AppError } from "../bindings/AppError";
import type { Edital } from "../bindings/Edital";
import { ErrorBox } from "../components/ErrorBox";
import { faixas } from "../components/faixas";
import { PdfViewer } from "../components/pdf/PdfViewer";
import { Processamento } from "../components/Processamento";
import { ipc, toAppError } from "../ipc";

/** Avisos de qualidade das páginas, já depois do OCR. */
function AvisoPaginas({ edital }: { edital: Edital }) {
  const semTexto = edital.paginas_problema.filter((p) => p.qualidade === "sem_texto").map((p) => p.pagina);
  const suspeitas = edital.paginas_problema.filter((p) => p.qualidade === "suspeito").map((p) => p.pagina);
  if (semTexto.length === 0 && suspeitas.length === 0 && edital.paginas_ocr.length === 0) return null;
  return (
    <div className="aviso aviso-coluna">
      {semTexto.length > 0 && <span>Páginas sem texto, mesmo com OCR: {faixas(semTexto)}.</span>}
      {suspeitas.length > 0 && (
        <span>Páginas com texto de leitura duvidosa: {faixas(suspeitas)}. Confira o conteúdo delas no PDF.</span>
      )}
      {edital.paginas_ocr.length > 0 && (
        <span className="mudo">Texto obtido por OCR nas páginas {faixas(edital.paginas_ocr)}.</span>
      )}
      {edital.exige_confirmacao && (
        <strong>Mais de 20% das páginas têm problema: o processamento vai pedir confirmação.</strong>
      )}
    </div>
  );
}

export function EditalScreen({
  id,
  aviso,
  onVoltar,
}: {
  id: string;
  aviso?: string;
  onVoltar: () => void;
}) {
  const [edital, setEdital] = useState<Edital | null>(null);
  const [erro, setErro] = useState<AppError | null>(null);
  const [mostrarAviso, setMostrarAviso] = useState(!!aviso);

  useEffect(() => {
    ipc.editalObter(id).then(setEdital, (e) => setErro(toAppError(e)));
  }, [id]);

  async function remover() {
    if (!edital) return;
    const ok = window.confirm(
      `Remover “${edital.nome}”? Isso apaga o PDF copiado, o texto extraído e os registros de processamento. Não dá para desfazer.`,
    );
    if (!ok) return;
    try {
      await ipc.editalRemover(edital.id);
      onVoltar();
    } catch (e) {
      setErro(toAppError(e));
    }
  }

  if (!edital) {
    return (
      <main className="tela">
        <button className="link" onClick={onVoltar}>
          ← Biblioteca
        </button>
        {erro ? <ErrorBox erro={erro} /> : <p className="mudo">Carregando…</p>}
      </main>
    );
  }

  const semTexto = edital.status === "sem_texto";
  return (
    <main className="tela edital">
      <div className="edital-topo">
        <button className="link" onClick={onVoltar}>
          ← Biblioteca
        </button>
        <Cabecalho edital={edital} onSalvo={setEdital} onErro={setErro} />
        {mostrarAviso && aviso && (
          <div className="aviso">
            {aviso}
            <button className="link" onClick={() => setMostrarAviso(false)} aria-label="Fechar">
              ✕
            </button>
          </div>
        )}
        {erro && <ErrorBox erro={erro} onFechar={() => setErro(null)} />}
        {semTexto && <ErrorBox erro={{ code: "EDITAL_SEM_TEXTO", detail: "" }} />}
        {!semTexto && <AvisoPaginas edital={edital} />}
      </div>

      <div className="edital-corpo">
        <section className="edital-pdf">
          <PdfViewer editalId={edital.id} />
        </section>
        <aside className="edital-lateral">
          <Processamento edital={edital} />
          <hr />
          <dl className="meta">
            <dt>Arquivo</dt>
            <dd>{edital.arquivo_original}</dd>
            <dt>Páginas</dt>
            <dd>{edital.total_paginas ?? "—"}</dd>
            <dt>Importado em</dt>
            <dd>{new Date(edital.importado_em).toLocaleString("pt-BR")}</dd>
            <dt>SHA-256</dt>
            <dd className="hash">{edital.sha256}</dd>
          </dl>
          <button className="perigo" onClick={remover}>
            Remover edital
          </button>
        </aside>
      </div>
    </main>
  );
}

function Cabecalho({
  edital,
  onSalvo,
  onErro,
}: {
  edital: Edital;
  onSalvo: (e: Edital) => void;
  onErro: (e: AppError) => void;
}) {
  const [editando, setEditando] = useState(false);
  const [nome, setNome] = useState(edital.nome);
  const [orgao, setOrgao] = useState(edital.orgao ?? "");
  const [banca, setBanca] = useState(edital.banca ?? "");

  function abrir() {
    setNome(edital.nome);
    setOrgao(edital.orgao ?? "");
    setBanca(edital.banca ?? "");
    setEditando(true);
  }

  async function salvar(ev: React.FormEvent) {
    ev.preventDefault();
    try {
      onSalvo(await ipc.editalAtualizar(edital.id, { nome, orgao: orgao || null, banca: banca || null }));
      setEditando(false);
    } catch (e) {
      onErro(toAppError(e));
    }
  }

  if (!editando) {
    return (
      <header className="cabecalho">
        <h1>{edital.nome}</h1>
        <p className="mudo">
          {[edital.orgao, edital.banca].filter(Boolean).join(" · ") || "Órgão e banca não informados"}
          {" · "}
          <button className="link" onClick={abrir}>
            Editar
          </button>
        </p>
      </header>
    );
  }
  return (
    <form className="cabecalho form-linha" onSubmit={salvar}>
      <label>
        Nome
        <input value={nome} onChange={(e) => setNome(e.target.value)} required autoFocus />
      </label>
      <label>
        Órgão
        <input value={orgao} onChange={(e) => setOrgao(e.target.value)} />
      </label>
      <label>
        Banca
        <input value={banca} onChange={(e) => setBanca(e.target.value)} />
      </label>
      <button className="primario" type="submit">
        Salvar
      </button>
      <button type="button" onClick={() => setEditando(false)}>
        Cancelar
      </button>
    </form>
  );
}
