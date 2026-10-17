// Só para desenvolvimento no navegador (`npm run dev` fora do Tauri): simula o core com dados de
// demonstração, para trabalhar o layout sem o app desktop. Nunca entra no build de produção.
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Analise } from "../bindings/Analise";
import type { BackendStatus } from "../bindings/BackendStatus";
import type { Cargo } from "../bindings/Cargo";
import type { Edital } from "../bindings/Edital";
import type { EtapaAnalise } from "../bindings/EtapaAnalise";
import type { Job } from "../bindings/Job";
import type { Materia } from "../bindings/Materia";
import type { Settings } from "../bindings/Settings";
import type { Topico } from "../bindings/Topico";

const agora = Date.now();
const dia = 86_400_000;

const semMetadados = {
  numero_edital: null,
  data_prova: null,
  inscricoes_inicio: null,
  inscricoes_fim: null,
  metadados_editados: false,
};

const editais: Edital[] = [
  {
    id: "demo-completo",
    nome: "Concurso Exemplo 2026 — Edital nº 01",
    orgao: "Tribunal de Contas do Estado (exemplo)",
    banca: "Banca Exemplo",
    arquivo_original: "edital-exemplo-01-2026.pdf",
    sha256: "9f2c4e0b7a1d3c5e8f6a2b4d0c9e7f1a3b5c7d9e1f3a5b7c9d1e3f5a7b9c1d3e",
    total_paginas: 84,
    paginas_sem_texto: [81, 82, 83],
    status: "importado",
    importado_em: new Date(agora - 2 * dia).toISOString(),
    numero_edital: "01/2026",
    data_prova: "2027-03-14",
    inscricoes_inicio: "2026-11-03",
    inscricoes_fim: "2026-12-04",
    metadados_editados: false,
  },
  {
    id: "demo-novo",
    nome: "edital_abertura_prefeitura_2026",
    orgao: null,
    banca: null,
    arquivo_original: "edital_abertura_prefeitura_2026.pdf",
    sha256: "1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5d6e7f809",
    total_paginas: 46,
    paginas_sem_texto: [],
    status: "importado",
    importado_em: new Date(agora - 3600_000).toISOString(),
    ...semMetadados,
  },
  {
    id: "demo-interrompido",
    nome: "retificacao_02",
    orgao: null,
    banca: null,
    arquivo_original: "retificacao_02.pdf",
    sha256: "ff00ee11dd22cc33bb44aa5599668877ff00ee11dd22cc33bb44aa5599668877",
    total_paginas: null,
    paginas_sem_texto: [],
    status: "importando",
    importado_em: new Date(agora - 5 * dia).toISOString(),
    ...semMetadados,
  },
  {
    id: "demo-escaneado",
    nome: "Edital digitalizado (scan)",
    orgao: "Câmara Municipal (exemplo)",
    banca: null,
    arquivo_original: "scan.pdf",
    sha256: "0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff",
    total_paginas: 12,
    paginas_sem_texto: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
    status: "sem_texto",
    importado_em: new Date(agora - 9 * dia).toISOString(),
    ...semMetadados,
  },
];

let n = 0;
const t = (texto: string, pagina: number | null, filhos: Topico[] = [], verificado = true): Topico => ({
  id: `t${n++}`,
  texto,
  pagina,
  verificado,
  inicio: null,
  fim: null,
  filhos,
});

const cargosDemo: Cargo[] = [
  { id: "c1", nome: "Auditor de Controle Externo — Tecnologia da Informação", vagas: 12, escolaridade: "Superior", pagina: 3, verificado: true },
  { id: "c2", nome: "Auditor de Controle Externo — Contabilidade", vagas: 20, escolaridade: "Superior", pagina: 3, verificado: true },
  { id: "c3", nome: "Agente da Fiscalização", vagas: 35, escolaridade: "Superior", pagina: 4, verificado: true },
];

const analises: Record<string, Analise> = {
  "demo-completo": {
    andamento: null,
    cargos: cargosDemo,
    cargo_escolhido: "c1",
    materias: [
      {
        id: "m1",
        nome: "Língua Portuguesa",
        pagina: 41,
        topicos: [
          t("Leitura e interpretação de textos", 41, [t("Sentido próprio e figurado", 41), t("Coesão e coerência", 41)]),
          t("Pontuação", 41),
          t("Crase", 41),
          t("Concordância verbal e nominal", 41),
          t("Regência verbal e nominal", 42),
          t("Colocação pronominal", 42),
        ],
      },
      {
        id: "m2",
        nome: "Governança e Gestão de TI",
        pagina: 44,
        topicos: [
          t("ITIL 4: práticas e cadeia de valor de serviço", 44),
          t("COBIT 2019", 44),
          t("Gerenciamento de projetos: PMBOK 7ª edição", 44, [t("Domínios de desempenho", 44), t("Princípios", 44)]),
          t("ISO/IEC 20000", 45),
        ],
      },
      {
        id: "m3",
        nome: "Engenharia de Software",
        pagina: 45,
        topicos: [
          t("Princípios SOLID", 45),
          t("Modelagem de processos com BPMN", 45),
          t("DevOps, integração e entrega contínuas", 46),
          t("Contêineres: Docker e Kubernetes", 46),
        ],
      },
      {
        id: "m4",
        nome: "Banco de Dados",
        pagina: 46,
        topicos: [t("Modelo relacional e normalização", 46), t("Transações e propriedades ACID", 47), t("SQL", 47)],
      },
      {
        id: "m5",
        nome: "Segurança da Informação",
        pagina: 47,
        topicos: [t("ISO/IEC 27001 e 27002", 47), t("LGPD", 47), t("Criptografia e certificação digital", 48)],
      },
      {
        id: "m6",
        nome: "Legislação",
        pagina: 48,
        topicos: [
          t("Lei nº 14.133/2021 (licitações e contratos)", 48),
          t("Lei nº 13.303/2016", 48),
          t("Lei de Acesso à Informação", 49),
          t("LC nº 182/2021 (marco das startups)", 49, [], false),
        ],
      },
    ],
  },
};

const jobs: Job[] = [
  {
    id: "j1",
    edital_id: "demo-completo",
    tipo: "extrator",
    status: "concluido",
    criado_em: new Date(agora - 2 * dia + 600_000).toISOString(),
    iniciado_em: null,
    finalizado_em: null,
    erro_code: null,
    erro_detail: null,
  },
  {
    id: "j0",
    edital_id: "demo-completo",
    tipo: "localizador",
    status: "falhou",
    criado_em: new Date(agora - 2 * dia).toISOString(),
    iniciado_em: null,
    finalizado_em: null,
    erro_code: "CLAUDE_TIMEOUT",
    erro_detail: "timeout após 600s",
  },
];

let settings: Settings = { claude_path: null, model: null, timeout_secs: 600 };

const pronto: BackendStatus = {
  estado: "pronto",
  caminho: "C:\\Users\\exemplo\\.local\\bin\\claude.exe",
  origem_caminho: "path",
  versao: "2.4.1",
  logado: true,
  metodo_login: "claude.ai",
  erro: null,
  retorno_em: null,
  verificado_em: new Date().toISOString(),
};

/** PDF mínimo de uma página, para o visualizador ter o que mostrar. */
function pdfDemo(): ArrayBuffer {
  const stream = "BT /F1 22 Tf 72 720 Td (Edital de demonstracao) Tj ET";
  const objs = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    `<< /Length ${stream.length} >>\nstream\n${stream}\nendstream`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman >>",
  ];
  let out = "%PDF-1.4\n";
  const offs: number[] = [];
  objs.forEach((o, i) => {
    offs.push(out.length);
    out += `${i + 1} 0 obj\n${o}\nendobj\n`;
  });
  const xref = out.length;
  out += `xref\n0 ${objs.length + 1}\n0000000000 65535 f \n`;
  out += offs.map((o) => `${String(o).padStart(10, "0")} 00000 n \n`).join("");
  out += `trailer\n<< /Size ${objs.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF`;
  return new TextEncoder().encode(out).buffer as ArrayBuffer;
}

const materiasDemo: Materia[] = analises["demo-completo"].materias!;

/** Simula o worker: o job fica "executando" por alguns segundos e então grava o resultado. */
function simular(editalId: string, etapa: EtapaAnalise, aplicar: (a: Analise) => void) {
  const a = (analises[editalId] ??= { cargos: [], cargo_escolhido: null, materias: null, andamento: null });
  a.andamento = { etapa, status: "executando", desde: new Date().toISOString(), erro: null };
  void emit("claude-status", { ...pronto, estado: "ocupado" });
  const fim = setTimeout(() => {
    aplicar(a);
    a.andamento = { ...a.andamento!, status: "concluido" };
    void emit("claude-status", pronto);
    void emit("analise-atualizada", editalId);
  }, 4000);
  cancelamentos[editalId] = () => {
    clearTimeout(fim);
    a.andamento = { ...a.andamento!, status: "cancelado" };
    void emit("claude-status", pronto);
    void emit("analise-atualizada", editalId);
  };
}
const cancelamentos: Record<string, () => void> = {};

export function instalarMockIpc() {
  mockWindows("main");
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      const porId = () => editais.find((e) => e.id === a.id);
      switch (cmd) {
        case "app_bootstrap":
          return { versao_app: "0.1.0-dev", pasta_dados: "C:\\Users\\exemplo\\AppData\\Roaming\\com.vbo.editalkiller", health_ja_passou: true };
        case "claude_status":
        case "claude_verificar":
          setTimeout(() => void emit("claude-status", pronto), 400);
          return { ...pronto, estado: "verificando" };
        case "settings_obter":
          return settings;
        case "settings_salvar":
          settings = a.settings as Settings;
          return settings;
        case "editais_listar":
          return editais;
        case "edital_obter":
          return porId() ?? Promise.reject({ code: "EDITAL_NOT_FOUND", detail: String(a.id) });
        case "edital_atualizar": {
          const e = porId()!;
          Object.assign(e, a.dados, { metadados_editados: true });
          return e;
        }
        case "edital_remover":
          editais.splice(editais.indexOf(porId()!), 1);
          return null;
        case "edital_pdf":
          return pdfDemo();
        case "jobs_listar":
          return jobs;
        case "edital_analise":
          return analises[a.editalId as string] ?? { cargos: [], cargo_escolhido: null, materias: null, andamento: null };
        case "analise_iniciar": {
          const id = a.editalId as string;
          simular(id, "localizar", (an) => {
            an.cargos = cargosDemo;
            an.cargo_escolhido = null;
            an.materias = null;
            const e = editais.find((x) => x.id === id)!;
            if (!e.metadados_editados) {
              Object.assign(e, {
                nome: "Concurso Público nº 03/2026 — Prefeitura (exemplo)",
                orgao: "Prefeitura Municipal (exemplo)",
                banca: "Banca Exemplo",
                numero_edital: "03/2026",
                data_prova: "2027-02-07",
              });
            }
          });
          return null;
        }
        case "cargo_escolher": {
          const id = a.editalId as string;
          analises[id].cargo_escolhido = a.cargoId as string;
          analises[id].materias = null;
          simular(id, "extrair", (an) => (an.materias = materiasDemo));
          return null;
        }
        case "analise_cancelar":
          cancelamentos[a.editalId as string]?.();
          return null;
        case "limpar_workspaces_jobs":
          return 2;
        case "escolher_pdfs":
          return [];
        default:
          return Promise.reject({ code: "INTERNAL", detail: `comando ${cmd} não simulado` });
      }
    },
    { shouldMockEvents: true },
  );
}
