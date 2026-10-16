import { useCallback, useEffect, useState } from "react";
import type { AppError } from "../bindings/AppError";
import type { Bloco } from "../bindings/Bloco";
import type { Categoria } from "../bindings/Categoria";
import type { Edital } from "../bindings/Edital";
import type { EstadoExtracao } from "../bindings/EstadoExtracao";
import type { Extracao } from "../bindings/Extracao";
import { ipc, onExtracaoAtualizada, onJobsAtualizados, toAppError } from "../ipc";
import { CATALOGO } from "../ipc/errors";
import { Confirmacao, useConfirmacaoDeUso } from "./Confirmacao";
import { ErrorBox } from "./ErrorBox";

export const CATEGORIA: Record<Categoria, string> = {
  dados_gerais: "Dados gerais",
  inscricao: "Inscrição",
  cargos_vagas: "Cargos e vagas",
  requisitos_atribuicoes: "Requisitos",
  cronograma: "Cronograma",
  provas_avaliacao: "Provas",
  conteudo_programatico: "Conteúdo programático",
  outros: "Outros",
};

const BLOCO: Record<Bloco, string> = {
  triagem: "Triagem",
  dados_gerais: "Dados gerais",
  cargos: "Cargos",
  cronograma: "Cronograma",
  provas: "Provas",
  conteudo: "Conteúdo programático",
};

/** A extração mais recente de um bloco (a lista vem da mais nova para a mais antiga). */
function ultima(estado: EstadoExtracao, bloco: Bloco): Extracao | undefined {
  return estado.extracoes.find((x) => x.bloco === bloco);
}

/** Painel lateral do Edital: processar (mapa + triagem) e o que já foi extraído. */
export function Processamento({ edital }: { edital: Edital }) {
  const [estado, setEstado] = useState<EstadoExtracao | null>(null);
  const [erro, setErro] = useState<AppError | null>(null);
  const [confirmarPaginas, setConfirmarPaginas] = useState<{ detalhe: string; responder: (ok: boolean) => void } | null>(
    null,
  );
  const [executando, setExecutando] = useState(false);
  const { executar, modal } = useConfirmacaoDeUso();

  const carregar = useCallback(() => {
    ipc.extracaoEstado(edital.id).then(setEstado, (e) => setErro(toAppError(e)));
  }, [edital.id]);

  useEffect(() => {
    carregar();
    const desliga = [
      onExtracaoAtualizada((id) => id === edital.id && carregar()),
      // Para ver "na fila" virar "executando".
      onJobsAtualizados(() => carregar()),
    ];
    return () => desliga.forEach((d) => void d.then((f) => f()));
  }, [carregar, edital.id]);

  async function processar() {
    setErro(null);
    setExecutando(true);
    try {
      let confirmarPag = false;
      for (;;) {
        try {
          const r = await executar((confirmarUso) => ipc.editalProcessar(edital.id, confirmarPag, confirmarUso));
          if (r) setEstado(r);
          return;
        } catch (e) {
          const err = toAppError(e);
          if (err.code !== "EDITAL_CONFIRMAR_PAGINAS" || confirmarPag) throw err;
          const ok = await new Promise<boolean>((responder) => setConfirmarPaginas({ detalhe: err.detail, responder }));
          setConfirmarPaginas(null);
          if (!ok) return;
          confirmarPag = true;
        }
      }
    } catch (e) {
      setErro(toAppError(e));
    } finally {
      setExecutando(false);
    }
  }

  if (edital.status === "sem_texto") {
    return (
      <>
        <h2>Processamento</h2>
        <p className="mudo">{CATALOGO.EDITAL_SEM_TEXTO.explicacao}</p>
      </>
    );
  }

  const triagem = estado && ultima(estado, "triagem");
  const pendente = triagem?.status === "pendente";
  const classificado = estado?.secoes.some((s) => s.categorias);

  return (
    <>
      <h2>Processamento</h2>
      {!triagem && (
        <p className="mudo">
          O app mapeia as seções do edital e o Claude classifica cada uma. Depois disso, os dados de cada parte são
          extraídos separadamente.
        </p>
      )}
      <div className="acoes">
        <button className="primario" onClick={processar} disabled={executando || pendente}>
          {pendente ? "Processando…" : triagem ? "Processar de novo" : "Processar edital"}
        </button>
      </div>

      {triagem && <StatusBloco extracao={triagem} />}
      {erro && <ErrorBox erro={erro} onFechar={() => setErro(null)} />}
      {estado?.e_edital && estado.triagem_motivo && <p className="mudo motivo">{estado.triagem_motivo}</p>}

      {estado && estado.secoes.length > 0 && (
        <>
          <h3>Seções ({estado.secoes.length})</h3>
          <ol className="secoes">
            {estado.secoes.map((s) => (
              <li key={s.ordem}>
                <span className="secao-titulo" title={s.titulo}>
                  {s.titulo}
                </span>
                <span className="mudo secao-paginas">
                  {s.pagina_ini === s.pagina_fim ? `p. ${s.pagina_ini}` : `p. ${s.pagina_ini}–${s.pagina_fim}`}
                </span>
                {classificado && (
                  <span className="secao-categorias">
                    {(s.categorias ?? ["outros"]).map((c) => (
                      <span key={c} className={`tag categoria ${c}`}>
                        {CATEGORIA[c]}
                      </span>
                    ))}
                  </span>
                )}
              </li>
            ))}
          </ol>
        </>
      )}

      {confirmarPaginas && (
        <Confirmacao
          titulo="Edital com muitas páginas problemáticas"
          confirmar="Processar mesmo assim"
          onConfirmar={() => confirmarPaginas.responder(true)}
          onCancelar={() => confirmarPaginas.responder(false)}
        >
          <p>{confirmarPaginas.detalhe}.</p>
          <p className="mudo">
            O texto dessas páginas está ausente ou ilegível, mesmo depois do OCR. O que estiver nelas pode faltar na
            extração.
          </p>
        </Confirmacao>
      )}
      {modal}
    </>
  );
}

function StatusBloco({ extracao }: { extracao: Extracao }) {
  if (extracao.status === "falhou" && extracao.erro_code) {
    return (
      <ErrorBox erro={{ code: extracao.erro_code, detail: extracao.erro_detail ?? "" }} contexto={BLOCO[extracao.bloco]} />
    );
  }
  const texto = extracao.status === "pendente" ? "na fila ou em execução (acompanhe em Jobs)" : "concluída";
  return (
    <p className={`status-bloco ${extracao.status}`}>
      {BLOCO[extracao.bloco]}: {texto}
    </p>
  );
}
