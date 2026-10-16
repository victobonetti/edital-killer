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
};
