CREATE TABLE edital (
    id               TEXT PRIMARY KEY,
    nome             TEXT NOT NULL,
    orgao            TEXT,
    banca            TEXT,
    arquivo_original TEXT NOT NULL,
    sha256           TEXT NOT NULL UNIQUE,
    total_paginas    INTEGER,
    status           TEXT NOT NULL,
    importado_em     TEXT NOT NULL
);

CREATE TABLE pagina (
    edital_id         TEXT NOT NULL REFERENCES edital(id) ON DELETE CASCADE,
    numero            INTEGER NOT NULL,
    -- Texto como o pdf.js entregou, sem limpeza.
    texto             TEXT NOT NULL,
    -- JSON: [{"t": "...", "i": offset_utf16}]
    itens             TEXT NOT NULL,
    texto_normalizado TEXT NOT NULL,
    -- u32 little-endian por char do normalizado (+1 sentinela): offset UTF-16 no original.
    mapa_normalizado  BLOB NOT NULL,
    sem_texto         INTEGER NOT NULL,
    PRIMARY KEY (edital_id, numero)
);

CREATE TABLE job (
    id            TEXT PRIMARY KEY,
    edital_id     TEXT REFERENCES edital(id) ON DELETE CASCADE,
    tipo          TEXT NOT NULL,
    status        TEXT NOT NULL,
    criado_em     TEXT NOT NULL,
    iniciado_em   TEXT,
    finalizado_em TEXT,
    erro_code     TEXT,
    erro_detail   TEXT
);
CREATE INDEX job_edital ON job(edital_id);

CREATE TABLE config (
    chave TEXT PRIMARY KEY,
    valor TEXT NOT NULL
);
