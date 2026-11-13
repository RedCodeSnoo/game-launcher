// Passerelle vers le backend. Hors Tauri (navigateur, `npm run dev`), un backend simulé prend le relais.
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Game, ScanState } from "./types";

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
};

export type { ScanState };
