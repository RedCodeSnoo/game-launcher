// Backend simulé pour le développement dans un navigateur (sans Tauri) et les essais de charge.
// Nombre de jeux réglable : http://localhost:1420/?games=1000
import { mockEmit } from "./api";
import type { Game, GameSourceView, SourceId } from "./types";

const WORDS = ["Shadow", "Star", "Legends", "Empire", "Quest", "Odyssey", "Rising", "Frontier", "Chronicles", "Tactics", "Racing", "Simulator", "Kingdom", "Echoes", "Horizon", "Zero", "Infinite", "Lost", "Dark", "Neon"];
const SOURCES: SourceId[] = ["steam", "steam", "steam", "epic", "gog", "battlenet", "riot", "ubisoft", "ea", "xbox", "local"];
const GENRES = ["Action", "RPG", "Stratégie", "Course", "Simulation", "Aventure", "Indépendant", "FPS"];

function rng(seed: number) {
  let s = seed;
  return () => (s = (s * 1664525 + 1013904223) % 4294967296) / 4294967296;
}

function makeGames(count: number): Game[] {
  const rand = rng(42);
  const pick = <T,>(arr: T[]) => arr[Math.floor(rand() * arr.length)];
  const games: Game[] = [];
  for (let i = 0; i < count; i++) {
    const source = pick(SOURCES);
    const installed = source === "steam" || source === "gog" ? rand() > 0.45 : true;
    const available = installed ? rand() > 0.08 : true;
    const title = `${pick(WORDS)} ${pick(WORDS)}${rand() > 0.6 ? " " + (2 + Math.floor(rand() * 4)) : ""}`;
    const drive = installed ? pick(["C:", "D:", "E:"]) : null;
    const tracked = rand() > 0.6 ? Math.floor(rand() * 36000) : 0;
    const src: GameSourceView = {
      id: `s${i}`,
      gameId: `g${i}`,
      source,
      externalId: String(1000 + i),
      installed,
      installPath: installed ? `${drive}\\Jeux\\${title}` : null,
      volumeSerial: installed ? "ABCD1234" : null,
      executablePath: source === "local" ? `${drive}\\Jeux\\${title}\\game.exe` : null,
      launchMethod: source === "local" ? "exe" : "uri",
      launchTarget: source === "local" ? `${drive}\\Jeux\\${title}\\game.exe` : `steam://rungameid/${1000 + i}`,
      launchArgs: null,
      workingDir: null,
      sourcePlaytimeMinutes: source === "steam" || source === "gog" ? Math.floor(rand() * 6000) : null,
      lastPlayedSource: rand() > 0.4 ? new Date(Date.now() - rand() * 4e10).toISOString() : null,
      available,
      owned: source === "steam",
      trackedSeconds: tracked,
      drive,
    };
    const achTotal = source === "steam" && rand() > 0.3 ? 10 + Math.floor(rand() * 60) : 0;
    games.push({
      id: `g${i}`,
      title,
      sortTitle: title.toLowerCase(),
      coverPath: null,
      heroPath: null,
      logoPath: null,
      description: "Description de démonstration générée par le backend simulé.",
      genres: [pick(GENRES)],
      developer: "Studio Démo",
      releaseDate: "2021-03-04",
      favorite: rand() > 0.9,
      hidden: false,
      addedAt: new Date(Date.now() - rand() * 6e10).toISOString(),
      sources: [src],
      installed,
      available,
      playtimeMinutes: src.sourcePlaytimeMinutes ?? Math.floor(tracked / 60),
      trackedSeconds: tracked,
      manualPlaytimeMinutes: null,
      lastPlayed: src.lastPlayedSource,
      lastSessionSeconds: tracked ? Math.floor(tracked / 4) : null,
      achTotal,
      achUnlocked: Math.floor(achTotal * rand()),
      collections: [],
    });
  }
  return games.sort((a, b) => a.sortTitle.localeCompare(b.sortTitle));
}

const params = new URLSearchParams(typeof location !== "undefined" ? location.search : "");
export const mockState = {
  games: makeGames(Number(params.get("games") ?? 120)),
  settings: {} as Record<string, string>,
};

type Args = Record<string, unknown>;

export const mockHandlers: Record<string, (args: Args) => unknown> = {
  get_games: () => mockState.games,
  is_scanning: () => false,
  refresh_library: () => {
    mockEmit("scan-state", { running: true });
    setTimeout(() => {
      mockEmit("scan-state", { running: false, reports: [] });
      mockEmit("library-updated", null);
    }, 800);
  },
  launch_game: (a) => {
    const g = mockState.games.find((x) => x.id === a.gameId);
    if (g && !g.installed) throw { code: "NOT_INSTALLED", detail: "" };
    if (g && !g.available) throw { code: "DISK_UNPLUGGED", detail: g.sources[0].installPath ?? "" };
  },
};

const notify = () => mockEmit("library-updated", null);
const find = (id: unknown) => mockState.games.find((g) => g.id === id);
let running: string[] = [];
let folders = [{ id: "w1", path: "D:\Jeux", exists: true }];

Object.assign(mockHandlers, {
  get_running: () => running,
  launch_game: (a: Args) => {
    const g = find(a.gameId);
    if (g && !g.installed) throw { code: "NOT_INSTALLED", detail: "" };
    if (g && !g.available) throw { code: "DISK_UNPLUGGED", detail: g.sources[0].installPath ?? "" };
    running = [...running, String(a.gameId)];
    mockEmit("running-changed", running);
    setTimeout(() => {
      running = running.filter((id) => id !== a.gameId);
      mockEmit("running-changed", running);
    }, 6000);
  },
  install_game: () => undefined,
  open_path: () => undefined,
  add_local_game: (a: Args) => {
    const exe = String(a.exePath);
    if (!exe.toLowerCase().endsWith(".exe")) throw { code: "NOT_AN_EXE", detail: exe };
    const base = mockState.games[0];
    const id = `g${Date.now()}`;
    const title = String(a.title ?? exe.split(/[\/]/).pop()?.replace(/\.exe$/i, "") ?? "Jeu");
    mockState.games = [
      ...mockState.games,
      { ...base, id, title, sortTitle: title.toLowerCase(), installed: true, available: true, favorite: false, playtimeMinutes: 0, trackedSeconds: 0, lastPlayed: null, lastSessionSeconds: null, achTotal: 0, achUnlocked: 0, addedAt: new Date().toISOString(), sources: [{ ...base.sources[0], id: `s${id}`, gameId: id, source: "local", installed: true, available: true, executablePath: exe, launchMethod: "exe", launchTarget: exe, sourcePlaytimeMinutes: null, trackedSeconds: 0, drive: exe.slice(0, 2) }] },
    ];
    notify();
  },
  get_watched_folders: () => folders,
  add_watched_folder: (a: Args) => void (folders = [...folders, { id: `w${Date.now()}`, path: String(a.path), exists: true }]),
  remove_watched_folder: (a: Args) => void (folders = folders.filter((f) => f.id !== a.id)),
  scan_watched_folders: () => [
    { name: "Hades", folder: "D:\Jeux\Hades", exe: "D:\Jeux\Hades\Hades.exe", size: 52428800, alternatives: [] },
    { name: "Astro Quest", folder: "D:\Jeux\Astro Quest", exe: "D:\Jeux\Astro Quest\AstroQuest.exe", size: 1048576, alternatives: ["D:\Jeux\Astro Quest\Binaries\Win64\AstroQuest-Win64-Shipping.exe"] },
    { name: "Portable", folder: null, exe: "D:\Jeux\Portable.exe", size: 7340032, alternatives: [] },
  ],
  add_candidates: (a: Args) => (a.candidates as unknown[]).length,
  ignore_candidates: () => undefined,
  get_sessions: (a: Args) => {
    const g = find(a.gameId);
    if (!g || g.trackedSeconds === 0) return [];
    return [0, 1, 2].map((i) => ({ id: `ps${i}`, source: g.sources[0].source, startedAt: new Date(Date.now() - (i + 1) * 9e7).toISOString(), endedAt: new Date(Date.now() - (i + 1) * 9e7 + 3.6e6).toISOString(), durationSeconds: Math.floor(g.trackedSeconds / 3), active: false }));
  },
  delete_session: () => undefined,
  update_game: (a: Args) => {
    const patch = a.patch as Record<string, unknown>;
    mockState.games = mockState.games.map((g) => {
      if (g.id !== a.gameId) return g;
      const next = { ...g, ...patch } as Game;
      if (typeof patch.manualPlaytimeMinutes === "number") next.playtimeMinutes = patch.manualPlaytimeMinutes;
      return next;
    });
    notify();
  },
  update_source: () => notify(),
  remove_game: (a: Args) => {
    mockState.games = mockState.games.filter((g) => g.id !== a.gameId);
    notify();
  },
});

let accounts = { steamId: null as string | null, steamKeySet: false, localSteamId: "76561198000000001", sgdbKeySet: false };

Object.assign(mockHandlers, {
  get_accounts: () => accounts,
  set_steam_account: (a: Args) => {
    if (!/^7656\d{13}$/.test(String(a.steamId))) throw { code: "STEAM_BAD_ID", detail: String(a.steamId) };
    accounts = { ...accounts, steamId: String(a.steamId), steamKeySet: true };
    return mockState.games.filter((g) => g.sources[0].source === "steam").length;
  },
  clear_steam_account: () => void (accounts = { ...accounts, steamId: null, steamKeySet: false }),
  set_sgdb_key: (a: Args) => void (accounts = { ...accounts, sgdbKeySet: Boolean(a.apiKey) }),
  sync_online: () => {
    mockEmit("sync-state", { running: true });
    setTimeout(() => mockEmit("sync-state", { running: false, errors: [] }), 1200);
  },
  get_achievements: (a: Args) => {
    const g = find(a.gameId);
    if (!g) return [];
    return Array.from({ length: g.achTotal }, (_, i) => ({
      apiName: `ACH_${i}`,
      name: i % 7 === 3 ? "Succès secret" : `Succès n° ${i + 1}`,
      description: "Accomplir un exploit de démonstration.",
      iconUnlocked: null,
      iconLocked: null,
      hidden: i % 7 === 3,
      unlocked: i < g.achUnlocked,
      unlockedAt: i < g.achUnlocked ? new Date(Date.now() - i * 8.64e7).toISOString() : null,
      globalPercent: Math.round((100 / (i + 1.5)) * 10) / 10,
    }));
  },
  refresh_game: () => false,
  search_metadata: (a: Args) => [
    { appid: 1, name: String(a.term), image: null },
    { appid: 2, name: `${String(a.term)} II`, image: null },
  ],
  apply_metadata: () => notify(),
  set_cover: () => notify(),
});

export async function mockCall(cmd: string, args: Args): Promise<unknown> {
  const handler = mockHandlers[cmd];
  if (!handler) throw { code: "INTERNAL", detail: `commande simulée absente : ${cmd}` };
  return handler(args);
}
