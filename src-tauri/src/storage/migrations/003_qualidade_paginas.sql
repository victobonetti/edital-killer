-- Qualidade do texto por página e OCR. A coluna `sem_texto` fica por compatibilidade
-- e passa a espelhar `qualidade = 'sem_texto'`. As páginas existentes são
-- reclassificadas em Rust logo após esta migração (`db::pos_migracao`).
ALTER TABLE pagina ADD COLUMN qualidade TEXT NOT NULL DEFAULT 'ok';
ALTER TABLE pagina ADD COLUMN origem TEXT NOT NULL DEFAULT 'pdf';
-- Confiança média do tesseract (0–100), só quando origem = 'ocr'.
ALTER TABLE pagina ADD COLUMN confianca REAL;
