-- Quadro de provas do cargo, lido pelo extrator junto com o conteúdo programático.
-- `*_verificado` = o trecho que sustenta o dado foi achado literalmente no edital.
ALTER TABLE materia ADD COLUMN bloco TEXT;
ALTER TABLE materia ADD COLUMN questoes INTEGER;
ALTER TABLE materia ADD COLUMN peso REAL;
ALTER TABLE materia ADD COLUMN quadro_verificado INTEGER NOT NULL DEFAULT 0;

CREATE TABLE estrutura_prova (
    cargo_id          TEXT PRIMARY KEY REFERENCES cargo(id) ON DELETE CASCADE,
    duracao_minutos   INTEGER,
    duracao_verificada INTEGER NOT NULL,
    -- Percentual (0–100) do total de pontos exigido para não ser eliminado.
    minimo_geral      REAL,
    minimo_verificado INTEGER NOT NULL
);

CREATE TABLE bloco_prova (
    cargo_id   TEXT NOT NULL REFERENCES cargo(id) ON DELETE CASCADE,
    ordem      INTEGER NOT NULL,
    nome       TEXT NOT NULL,
    -- Questões do bloco, quando o edital informa por bloco e não por matéria.
    questoes   INTEGER,
    -- Percentual (0–100) dos pontos do bloco exigido.
    minimo     REAL,
    verificado INTEGER NOT NULL,
    PRIMARY KEY (cargo_id, ordem)
);
