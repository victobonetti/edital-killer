// Wrappers tipados dos commands do core. Toda chamada IPC passa por aqui.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppError } from "../bindings/AppError";
import type { BackendStatus } from "../bindings/BackendStatus";
import type { Bootstrap } from "../bindings/Bootstrap";
import type { Edital } from "../bindings/Edital";
import type { EstadoExtracao } from "../bindings/EstadoExtracao";
import type { EstadoFila } from "../bindings/EstadoFila";
import type { EditalUpdate } from "../bindings/EditalUpdate";
import type { ErrorCode } from "../bindings/ErrorCode";
import type { ImportStart } from "../bindings/ImportStart";
import type { Job } from "../bindings/Job";
import type { PaginaInput } from "../bindings/PaginaInput";
import type { Settings } from "../bindings/Settings";
import type { UsoClaude } from "../bindings/UsoClaude";

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
  /** Devolve as páginas do lote que precisam de OCR. */
  importarPaginas: (id: string, paginas: PaginaInput[]) =>
    call<number[]>("importar_paginas", { id, paginas }),
  importarFinalizar: (id: string, totalPaginas: number) =>
    call<Edital>("importar_finalizar", { id, totalPaginas }),
  importarAbortar: (id: string, code: Extract<ErrorCode, "IMPORT_ENCRYPTED" | "IMPORT_CORRUPTED" | "IMPORT_CANCELADO">) =>
    call<void>("importar_abortar", { id, code }),

  /** Mapa + triagem. Pode falhar com `EDITAL_CONFIRMAR_PAGINAS` ou `CLAUDE_USO_ALTO`, para a UI confirmar. */
  editalProcessar: (id: string, confirmarPaginas: boolean, confirmarUso: boolean) =>
    call<EstadoExtracao>("edital_processar", { id, confirmarPaginas, confirmarUso }),
  extracaoEstado: (id: string) => call<EstadoExtracao>("extracao_estado", { id }),

  jobsListar: () => call<Job[]>("jobs_listar"),
  filaEstado: () => call<EstadoFila>("fila_estado"),
  jobCancelar: (id: string) => call<void>("job_cancelar", { id }),
  filaRetomar: () => call<void>("fila_retomar"),
  claudeUso: () => call<UsoClaude | null>("claude_uso"),
  /** Com uso alto e `confirmado = false`, falha com `CLAUDE_USO_ALTO`. */
  claudeTestarExecucao: (confirmado: boolean) => call<Job>("claude_testar_execucao", { confirmado }),
  appSair: () => call<void>("app_sair"),
};

export function onClaudeStatus(cb: (s: BackendStatus) => void): Promise<UnlistenFn> {
  return listen<BackendStatus>("claude-status", (ev) => cb(ev.payload));
}

export function onJobsAtualizados(cb: (e: EstadoFila) => void): Promise<UnlistenFn> {
  return listen<EstadoFila>("jobs-atualizados", (ev) => cb(ev.payload));
}

export function onClaudeUso(cb: (u: UsoClaude) => void): Promise<UnlistenFn> {
  return listen<UsoClaude>("claude-uso", (ev) => cb(ev.payload));
}

export function onJobTerminou(cb: (j: Job) => void): Promise<UnlistenFn> {
  return listen<Job>("job-terminou", (ev) => cb(ev.payload));
}

/** A janela pediu para fechar com job em execução. */
export function onFecharPedido(cb: (e: EstadoFila) => void): Promise<UnlistenFn> {
  return listen<EstadoFila>("fechar-pedido", (ev) => cb(ev.payload));
}

/** O processamento de um edital mudou. Payload: id do edital. */
export function onExtracaoAtualizada(cb: (editalId: string) => void): Promise<UnlistenFn> {
  return listen<string>("extracao-atualizada", (ev) => cb(ev.payload));
}
