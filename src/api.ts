// Passerelle vers le backend. Hors Tauri (navigateur, `npm run dev`), un backend simulé prend le relais.
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Candidate, Game, GamePatch, ScanState, Session, SourcePatch, WatchedFolder } from "./types";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Handler = (payload: unknown) => void;
const mockListeners = new Map<string, Set<Handler>>();

/** Émet un événement côté backend simulé. */
export function mockEmit(event: string, payload: unknown) {
  mockListeners.get(event)?.forEach((h) => h(payload));
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) return invoke<T>(cmd, args);
  const { mockCall } = await import("./mock");
  return mockCall(cmd, args ?? {}) as Promise<T>;
}

/** S'abonne à un événement du backend ; renvoie la fonction de désabonnement. */
export function on<T>(event: string, handler: (payload: T) => void): () => void {
  if (isTauri) {
    const p = listen<T>(event, (e) => handler(e.payload));
    return () => void p.then((un) => un());
  }
  const set = mockListeners.get(event) ?? new Set<Handler>();
  mockListeners.set(event, set);
  set.add(handler as Handler);
  return () => set.delete(handler as Handler);
}

/** URL affichable d'une image du cache local. */
export function imageUrl(path: string | null | undefined): string | undefined {
  if (!path) return undefined;
  if (!isTauri || /^https?:|^data:/.test(path)) return path;
  return convertFileSrc(path);
}

export const api = {
  getGames: () => call<Game[]>("get_games"),
  refreshLibrary: () => call<void>("refresh_library"),
  isScanning: () => call<boolean>("is_scanning"),
  launchGame: (gameId: string, sourceId?: string) => call<void>("launch_game", { gameId, sourceId }),
  getRunning: () => call<string[]>("get_running"),
  installGame: (gameId: string) => call<void>("install_game", { gameId }),
  openPath: (path: string) => call<void>("open_path", { path }),
  addLocalGame: (exePath: string, title?: string, args?: string, workingDir?: string) => call<void>("add_local_game", { exePath, title, args, workingDir }),
  getWatchedFolders: () => call<WatchedFolder[]>("get_watched_folders"),
  addWatchedFolder: (path: string) => call<void>("add_watched_folder", { path }),
  removeWatchedFolder: (id: string) => call<void>("remove_watched_folder", { id }),
  scanWatchedFolders: () => call<Candidate[]>("scan_watched_folders"),
  addCandidates: (candidates: { name: string; folder: string | null; exe: string }[]) => call<number>("add_candidates", { candidates }),
  ignoreCandidates: (exes: string[]) => call<void>("ignore_candidates", { exes }),
  getSessions: (gameId: string) => call<Session[]>("get_sessions", { gameId }),
  deleteSession: (sessionId: string) => call<void>("delete_session", { sessionId }),
  updateGame: (gameId: string, patch: GamePatch) => call<void>("update_game", { gameId, patch }),
  updateSource: (sourceId: string, patch: SourcePatch) => call<void>("update_source", { sourceId, patch }),
  removeGame: (gameId: string) => call<void>("remove_game", { gameId }),
};

/** Sélecteur de fichier natif (exécutable, image…). */
export async function pickFile(title: string, extensions: string[]): Promise<string | null> {
  if (!isTauri) return window.prompt(`${title} (chemin)`) || null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ title, multiple: false, directory: false, filters: [{ name: title, extensions }] });
  return typeof picked === "string" ? picked : null;
}

export async function pickFolder(title: string): Promise<string | null> {
  if (!isTauri) return window.prompt(`${title} (chemin)`) || null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ title, multiple: false, directory: true });
  return typeof picked === "string" ? picked : null;
}

export async function pickSavePath(title: string, defaultPath: string, extensions: string[]): Promise<string | null> {
  if (!isTauri) return window.prompt(`${title} (chemin)`, defaultPath) || null;
  const { save } = await import("@tauri-apps/plugin-dialog");
  return (await save({ title, defaultPath, filters: [{ name: title, extensions }] })) ?? null;
}

/** Fichiers déposés sur la fenêtre (glisser-déposer d'un .exe, F-LOC-01). */
export function onFileDrop(handler: (paths: string[]) => void): () => void {
  if (!isTauri) return () => {};
  const p = import("@tauri-apps/api/webview").then(({ getCurrentWebview }) =>
    getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === "drop") handler(e.payload.paths);
    }),
  );
  return () => void p.then((un) => un());
}

export type { ScanState };
