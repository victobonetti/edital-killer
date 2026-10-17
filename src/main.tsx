import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/newsreader/opsz.css";
import "@fontsource-variable/newsreader/opsz-italic.css";
import "@fontsource-variable/hanken-grotesk";
import "@fontsource-variable/jetbrains-mono";
import App from "./App";

async function iniciar() {
  // Fora do Tauri (navegador em `npm run dev`), simula o core para trabalhar a interface.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    (await import("./dev/mockIpc")).instalarMockIpc();
  }
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

void iniciar();
