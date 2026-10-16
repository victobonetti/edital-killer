// Copia os arquivos do tesseract.js para public/ocr/ e baixa o modelo de português,
// para o OCR rodar offline (por padrão o tesseract.js busca tudo em CDN).
// Roda antes de `vite` e `vite build`; não faz nada se já estiver tudo lá.
import { copyFileSync, existsSync, mkdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const raiz = join(dirname(fileURLToPath(import.meta.url)), "..");
const destino = join(raiz, "public", "ocr");
mkdirSync(destino, { recursive: true });

const copiar = [
  "node_modules/tesseract.js/dist/worker.min.js",
  "node_modules/tesseract.js-core/tesseract-core-lstm.wasm.js",
  "node_modules/tesseract.js-core/tesseract-core-simd-lstm.wasm.js",
  "node_modules/tesseract.js-core/tesseract-core-relaxedsimd-lstm.wasm.js",
];
for (const origem of copiar) {
  const de = join(raiz, origem);
  const para = join(destino, origem.split("/").pop());
  if (!existsSync(para) || statSync(para).mtimeMs < statSync(de).mtimeMs) copyFileSync(de, para);
}

// Modelo "fast" (LSTM, ~2 MB): bom para texto impresso e bem mais leve que o "best".
const MODELO = "https://github.com/tesseract-ocr/tessdata_fast/raw/main/por.traineddata";
const modelo = join(destino, "por.traineddata");
if (!existsSync(modelo)) {
  console.log("ocr-assets: baixando por.traineddata…");
  const r = await fetch(MODELO);
  if (!r.ok) throw new Error(`ocr-assets: download falhou (${r.status}) em ${MODELO}`);
  writeFileSync(modelo, Buffer.from(await r.arrayBuffer()));
}
