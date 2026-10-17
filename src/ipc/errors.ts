// Catálogo de erros visto pela UI: cada `code` vira título, explicação e o que fazer.
// O `detail` do core aparece só como informação técnica secundária.
import type { ErrorCode } from "../bindings/ErrorCode";

export type ErroExplicado = { titulo: string; explicacao: string; acao?: string };

export const CATALOGO: Record<ErrorCode, ErroExplicado> = {
  INTERNAL: {
    titulo: "Erro inesperado",
    explicacao: "Algo deu errado dentro do app.",
    acao: "Tente de novo. Se repetir, abra os logs em Configurações.",
  },
  DATABASE: {
    titulo: "Erro no banco de dados",
    explicacao: "O app não conseguiu ler ou gravar no banco local.",
    acao: "Verifique o espaço em disco e abra os logs em Configurações.",
  },
  IO: {
    titulo: "Erro de arquivo",
    explicacao: "O app não conseguiu ler ou gravar um arquivo.",
    acao: "Confira se o arquivo existe e se você tem permissão de acesso.",
  },
  INVALID_INPUT: {
    titulo: "Operação inválida",
    explicacao: "O app recebeu um pedido que não faz sentido no estado atual.",
  },
  IMPORT_NOT_PDF: {
    titulo: "Não é um PDF",
    explicacao: "O arquivo escolhido não tem cabeçalho de PDF. Nada foi importado.",
    acao: "Escolha o arquivo .pdf do edital.",
  },
  IMPORT_ENCRYPTED: {
    titulo: "PDF protegido por senha",
    explicacao: "Este PDF só abre com senha, e o app ainda não aceita PDFs protegidos.",
    acao: "Baixe uma versão sem senha do edital ou remova a proteção antes de importar.",
  },
  IMPORT_CORRUPTED: {
    titulo: "PDF corrompido",
    explicacao: "O arquivo parece ser um PDF, mas não pôde ser aberto. A cópia foi descartada.",
    acao: "Baixe o edital de novo da página oficial e tente outra vez.",
  },
  EDITAL_NOT_FOUND: {
    titulo: "Edital não encontrado",
    explicacao: "Este edital não existe mais na biblioteca.",
  },
  EDITAL_SEM_TEXTO: {
    titulo: "Edital sem texto",
    explicacao:
      "Nenhuma página deste PDF tem texto extraível; ele provavelmente é uma digitalização. Leitura por OCR ainda não é suportada.",
    acao: "Procure uma versão do edital com texto selecionável.",
  },
  CLAUDE_NOT_FOUND: {
    titulo: "Claude Code não encontrado",
    explicacao: "O app usa o Claude Code CLI instalado nesta máquina, e ele não foi encontrado.",
    acao: "Instale o Claude Code ou informe o caminho do executável em Configurações.",
  },
  CLAUDE_NOT_RUNNABLE: {
    titulo: "Claude Code não executou",
    explicacao: "O executável foi encontrado, mas não respondeu como esperado.",
    acao: "Rode `claude --version` num terminal para ver o problema, ou reinstale o Claude Code.",
  },
  CLAUDE_NOT_LOGGED_IN: {
    titulo: "Claude Code sem login",
    explicacao: "O Claude Code está instalado, mas não há conta logada.",
    acao: "Abra um terminal, rode `claude` e faça login. Depois clique em Verificar de novo.",
  },
  CLAUDE_TIMEOUT: {
    titulo: "Claude Code não respondeu",
    explicacao: "O Claude Code demorou demais para responder.",
    acao: "Verifique sua conexão e tente de novo.",
  },
  CLAUDE_EXEC_FAILED: {
    titulo: "O Claude não conseguiu terminar",
    explicacao: "A análise parou no meio por um erro do Claude ou da conexão.",
    acao: "Tente de novo em alguns minutos. Se repetir, abra os logs em Ajustes › Avançado.",
  },
  CLAUDE_USAGE_LIMIT: {
    titulo: "Limite de uso do Claude atingido",
    explicacao: "Sua conta do Claude chegou ao limite de uso por agora. Nada foi perdido.",
    acao: "Espere o limite renovar e tente de novo.",
  },
  CLAUDE_MODEL_UNAVAILABLE: {
    titulo: "Modelo indisponível na sua conta",
    explicacao: "O modelo configurado para a análise não está liberado no seu plano do Claude.",
    acao: "Em Ajustes › Avançado, deixe o modelo em branco (usa o padrão) ou escolha outro.",
  },
  CLAUDE_OUTPUT_INVALID: {
    titulo: "Resposta incompleta",
    explicacao: "O Claude respondeu, mas a resposta não veio no formato esperado. Nada do que já existia foi alterado.",
    acao: "Tente de novo; costuma funcionar na segunda vez.",
  },
  JOB_CANCELLED: {
    titulo: "Análise cancelada",
    explicacao: "A análise foi interrompida a pedido.",
  },
  ANALISE_EM_ANDAMENTO: {
    titulo: "Análise em andamento",
    explicacao: "Este edital já está sendo analisado.",
    acao: "Espere terminar ou cancele a análise atual.",
  },
  CARGOS_NAO_ENCONTRADOS: {
    titulo: "Nenhum cargo encontrado",
    explicacao: "O Claude leu o edital, mas não encontrou um quadro de cargos.",
    acao: "Confira se este PDF é o edital de abertura (e não uma retificação ou um anexo) e tente de novo.",
  },
};
