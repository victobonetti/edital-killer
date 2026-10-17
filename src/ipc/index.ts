// Wrappers tipados dos commands do core. Toda chamada IPC passa por aqui.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Analise } from "../bindings/Analise";
import type { AppError } from "../bindings/AppError";
import type { BackendStatus } from "../bindings/BackendStatus";
import type { Bootstrap } from "../bindings/Bootstrap";
import type { Edital } from "../bindings/Edital";
import type { EditalUpdate } from "../bindings/EditalUpdate";
import type { ErrorCode } from "../bindings/ErrorCode";
import type { ImportStart } from "../bindings/ImportStart";
import type { Job } from "../bindings/Job";
import type { PaginaInput } from "../bindings/PaginaInput";
import type { Settings } from "../bindings/Settings";

export function isAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && "code" in e && "detail" in e;
}

/** Garante que todo erro vire `AppError`, inclusive falhas do próprio IPC. */
export function toAppError(e: unknown): AppError {
  if (isAppError(e)) return e;
  return { code: "INTERNAL", detail: e instanceof Error ? e.message : String(e) };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toAppError(e);
  }
}

export const ipc = {
  appBootstrap: () => call<Bootstrap>("app_bootstrap"),
  settingsObter: () => call<Settings>("settings_obter"),
  settingsSalvar: (settings: Settings) => call<Settings>("settings_salvar", { settings }),
  abrirPastaDados: () => call<void>("abrir_pasta_dados"),
  abrirPastaLogs: () => call<void>("abrir_pasta_logs"),
  limparWorkspacesJobs: () => call<number>("limpar_workspaces_jobs"),

  claudeStatus: () => call<BackendStatus>("claude_status"),
  claudeVerificar: () => call<BackendStatus>("claude_verificar"),

  escolherPdfs: () => call<string[]>("escolher_pdfs"),
  editaisListar: () => call<Edital[]>("editais_listar"),
  editalObter: (id: string) => call<Edital>("edital_obter", { id }),
  editalAtualizar: (id: string, dados: EditalUpdate) =>
    call<Edital>("edital_atualizar", { id, dados }),
  editalRemover: (id: string) => call<void>("edital_remover", { id }),
  editalPdf: (id: string) => call<ArrayBuffer>("edital_pdf", { id }),

  importarIniciar: (caminho: string) => call<ImportStart>("importar_iniciar", { caminho }),
  importarRetomar: (id: string) => call<Edital>("importar_retomar", { id }),
  importarPaginas: (id: string, paginas: PaginaInput[]) =>
    call<void>("importar_paginas", { id, paginas }),
  importarFinalizar: (id: string, totalPaginas: number) =>
    call<Edital>("importar_finalizar", { id, totalPaginas }),
  importarAbortar: (id: string, code: Extract<ErrorCode, "IMPORT_ENCRYPTED" | "IMPORT_CORRUPTED">) =>
    call<void>("importar_abortar", { id, code }),

  jobsListar: () => call<Job[]>("jobs_listar"),

  editalAnalise: (editalId: string) => call<Analise>("edital_analise", { editalId }),
  analiseIniciar: (editalId: string) => call<void>("analise_iniciar", { editalId }),
  cargoEscolher: (editalId: string, cargoId: string) => call<void>("cargo_escolher", { editalId, cargoId }),
  analiseCancelar: (editalId: string) => call<void>("analise_cancelar", { editalId }),
};

export function onClaudeStatus(cb: (s: BackendStatus) => void): Promise<UnlistenFn> {
  return listen<BackendStatus>("claude-status", (ev) => cb(ev.payload));
}

/** A análise de um edital mudou (começou, terminou, falhou). Payload: id do edital. */
export function onAnalise(cb: (editalId: string) => void): Promise<UnlistenFn> {
  return listen<string>("analise-atualizada", (ev) => cb(ev.payload));
}
