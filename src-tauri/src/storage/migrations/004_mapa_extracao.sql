-- Item 4: mapa do edital (etapa 1), triagem (etapa 2) e estado da extração por bloco.
CREATE TABLE secao (
    edital_id  TEXT NOT NULL REFERENCES edital(id) ON DELETE CASCADE,
    ordem      INTEGER NOT NULL,
    titulo     TEXT NOT NULL,
    pagina_ini INTEGER NOT NULL,
    -- Bytes no texto original das páginas inicial e final.
    byte_ini   INTEGER NOT NULL,
    pagina_fim INTEGER NOT NULL,
    byte_fim   INTEGER NOT NULL,
    -- JSON com as categorias da triagem; NULL até a triagem terminar.
    categorias TEXT,
    PRIMARY KEY (edital_id, ordem)
);

CREATE TABLE extracao (
    id            TEXT PRIMARY KEY,
    edital_id     TEXT NOT NULL REFERENCES edital(id) ON DELETE CASCADE,
    bloco         TEXT NOT NULL,
    -- Blocos por cargo (provas, conteúdo).
    cargo_id      TEXT,
    job_id        TEXT REFERENCES job(id) ON DELETE SET NULL,
    -- Versão do papel (prompt + schema) que gerou o resultado.
    versao        INTEGER NOT NULL,
    status        TEXT NOT NULL,
    erro_code     TEXT,
    erro_detail   TEXT,
    criado_em     TEXT NOT NULL,
    atualizado_em TEXT NOT NULL
);
CREATE INDEX extracao_edital ON extracao(edital_id, bloco);
CREATE INDEX extracao_job ON extracao(job_id);

-- Resultado da triagem: NULL até ela terminar.
ALTER TABLE edital ADD COLUMN e_edital INTEGER;
ALTER TABLE edital ADD COLUMN triagem_motivo TEXT;
