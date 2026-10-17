-- Metadados lidos do edital pelo localizador. `metadados_editados` = o usuário editou
-- o cabeçalho; a partir daí a análise nunca mais sobrescreve esses campos.
ALTER TABLE edital ADD COLUMN numero_edital TEXT;
ALTER TABLE edital ADD COLUMN data_prova TEXT;
ALTER TABLE edital ADD COLUMN inscricoes_inicio TEXT;
ALTER TABLE edital ADD COLUMN inscricoes_fim TEXT;
ALTER TABLE edital ADD COLUMN metadados_editados INTEGER NOT NULL DEFAULT 0;
-- Sem FK aqui (ALTER TABLE não aceita); a consistência fica no repositório.
ALTER TABLE edital ADD COLUMN cargo_escolhido TEXT;

CREATE TABLE cargo (
    id           TEXT PRIMARY KEY,
    edital_id    TEXT NOT NULL REFERENCES edital(id) ON DELETE CASCADE,
    ordem        INTEGER NOT NULL,
    nome         TEXT NOT NULL,
    vagas        INTEGER,
    escolaridade TEXT,
    pagina       INTEGER,
    verificado   INTEGER NOT NULL
);
CREATE INDEX cargo_edital ON cargo(edital_id);

CREATE TABLE materia (
    id       TEXT PRIMARY KEY,
    cargo_id TEXT NOT NULL REFERENCES cargo(id) ON DELETE CASCADE,
    ordem    INTEGER NOT NULL,
    nome     TEXT NOT NULL,
    pagina   INTEGER
);
CREATE INDEX materia_cargo ON materia(cargo_id);

-- Tópicos em até dois níveis (`pai_id`). `inicio`/`fim`: offsets UTF-16 no texto
-- original da página, para o destaque no PDF.
CREATE TABLE topico (
    id         TEXT PRIMARY KEY,
    materia_id TEXT NOT NULL REFERENCES materia(id) ON DELETE CASCADE,
    pai_id     TEXT REFERENCES topico(id) ON DELETE CASCADE,
    ordem      INTEGER NOT NULL,
    texto      TEXT NOT NULL,
    pagina     INTEGER,
    inicio     INTEGER,
    fim        INTEGER,
    verificado INTEGER NOT NULL
);
CREATE INDEX topico_materia ON topico(materia_id);
