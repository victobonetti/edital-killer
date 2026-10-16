-- Execução de jobs pelo runner: tentativas, consumo e resultado.
ALTER TABLE job ADD COLUMN tentativas INTEGER NOT NULL DEFAULT 0;
ALTER TABLE job ADD COLUMN modelo TEXT;
ALTER TABLE job ADD COLUMN tokens_entrada INTEGER;
ALTER TABLE job ADD COLUMN tokens_saida INTEGER;
ALTER TABLE job ADD COLUMN tokens_cache_escrita INTEGER;
ALTER TABLE job ADD COLUMN tokens_cache_leitura INTEGER;
ALTER TABLE job ADD COLUMN tokens_raciocinio INTEGER;
ALTER TABLE job ADD COLUMN custo_estimado_usd REAL;
ALTER TABLE job ADD COLUMN duracao_ms INTEGER;
ALTER TABLE job ADD COLUMN utilizacao_5h REAL;
ALTER TABLE job ADD COLUMN utilizacao_7d REAL;
-- `structured_output` validado, como JSON.
ALTER TABLE job ADD COLUMN resultado TEXT;
CREATE INDEX job_status ON job(status, criado_em);
