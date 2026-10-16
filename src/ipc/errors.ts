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
  IMPORT_OCR_FAILED: {
    titulo: "OCR falhou em algumas páginas",
    explicacao: "A leitura por OCR não funcionou nessas páginas; elas ficaram como estavam no PDF.",
    acao: "Se o conteúdo programático estiver nelas, procure uma versão do edital com texto selecionável.",
  },
  IMPORT_CANCELADO: {
    titulo: "Importação cancelada",
    explicacao: "A importação foi interrompida e nada foi salvo.",
  },
  EDITAL_NOT_FOUND: {
    titulo: "Edital não encontrado",
    explicacao: "Este edital não existe mais na biblioteca.",
  },
  EDITAL_SEM_TEXTO: {
    titulo: "Edital sem texto",
    explicacao:
      "Nenhuma página deste PDF tem texto legível, nem depois da leitura por OCR. O processamento fica bloqueado.",
    acao: "Procure uma versão do edital com texto selecionável ou uma digitalização de melhor qualidade.",
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
    explicacao: "O Claude Code demorou demais para responder e foi encerrado.",
    acao: "Verifique sua conexão. Para editais grandes, aumente o tempo limite em Configurações.",
  },
  CLAUDE_LIMITE_USO: {
    titulo: "Limite de uso do Claude atingido",
    explicacao: "Seu plano do Claude chegou ao limite de uso. A análise ficou guardada.",
    acao: "Ela continua sozinha quando o limite renovar. Não é preciso fazer nada.",
  },
  CLAUDE_USO_ALTO: {
    titulo: "Uso do Claude está alto",
    explicacao: "Sua janela de uso passou de 80%. Um job grande pode esgotá-la no meio.",
    acao: "Confirme para iniciar mesmo assim, ou espere a janela renovar.",
  },
  CLAUDE_SAIDA_INVALIDA: {
    titulo: "Resposta do Claude fora do formato",
    explicacao: "O Claude respondeu, mas não no formato que o app exige, mesmo depois de uma nova tentativa.",
    acao: "Tente de novo. Se repetir, abra os registros técnicos em Ajustes.",
  },
  CLAUDE_ERRO_API: {
    titulo: "Serviço do Claude com problema",
    explicacao: "O serviço do Claude devolveu erro (por exemplo, sobrecarga) em três tentativas seguidas.",
    acao: "Espere alguns minutos e tente de novo.",
  },
  CLAUDE_PROCESSO_FALHOU: {
    titulo: "Claude Code parou no meio",
    explicacao: "O processo do Claude Code terminou sem entregar resposta.",
    acao: "Rode `claude` num terminal para ver se ele abre normalmente. Os detalhes técnicos trazem a saída de erro.",
  },
  JOB_NOT_FOUND: {
    titulo: "Job não encontrado",
    explicacao: "Esta análise não existe mais ou já terminou.",
  },
  JOB_CANCELADO: {
    titulo: "Cancelado",
    explicacao: "A análise foi interrompida antes de terminar.",
    acao: "Processe de novo quando quiser.",
  },
  JOB_INTERROMPIDO: {
    titulo: "Interrompido",
    explicacao: "O app foi fechado durante a análise.",
    acao: "Processe de novo.",
  },
  EDITAL_CONFIRMAR_PAGINAS: {
    titulo: "Muitas páginas com problema",
    explicacao: "Mais de 20% das páginas deste edital estão sem texto ou ilegíveis, mesmo depois do OCR.",
    acao: "Confirme para processar mesmo assim.",
  },
  EDITAL_NAO_E_EDITAL: {
    titulo: "Não parece um edital de abertura",
    explicacao: "A triagem concluiu que este PDF não é um edital de abertura de concurso (o motivo está nos detalhes).",
    acao: "Confira se importou o arquivo certo. Se for mesmo um edital, processe de novo.",
  },
  EXTRACAO_SEM_SECOES: {
    titulo: "Nada para extrair",
    explicacao: "A triagem não encontrou nenhuma seção deste tipo no edital.",
  },
  EXTRACAO_INCOMPLETA: {
    titulo: "Extração com itens não conferidos",
    explicacao: "Parte dos itens extraídos não foi encontrada literalmente no texto do edital.",
    acao: "Revise os itens marcados ou extraia este bloco de novo.",
  },
};
