// Backend simulé pour le développement dans un navigateur (sans Tauri) et les essais de charge.
// Nombre de jeux réglable : http://localhost:1420/?games=1000
import { mockEmit } from "./api";
import type { Game, GameSourceView, GameStatus, Period, SourceId, StatsFilter } from "./types";

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
      installPathOverride: null,
      executablePathOverride: null,
      launchMode: null,
      directArgs: null,
      directWorkingDir: null,
      trackedSeconds: tracked,
      sinceSyncSeconds: 0,
      playtimeMinutes: 0,
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
      manualBaselineMinutes: null,
      manualBaselineAt: null,
      sinceManualSeconds: 0,
      sinceSyncSeconds: 0,
      noTracking: false,
      status: ([null, "backlog", "playing", null, "finished", "backlog", "completed", "paused", null, "abandoned", "backlog"] as (GameStatus | null)[])[i % 11],
      statusChangedAt: null,
      rating: i % 4 === 0 ? 5 + (i % 6) : null,
      notes: null,
      timeToBeatMain: i % 3 === 0 ? 300 + (i * 97) % 2400 : null,
      timeToBeatFull: i % 3 === 0 ? 900 + (i * 131) % 6000 : null,
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
const mockNews = () =>
  mockState.games
    .filter((g) => g.sources[0].source === "steam")
    .slice(0, 5)
    .map((g, i) => ({
      id: `n${i}`,
      appid: g.sources[0].externalId,
      gameId: g.id,
      gameTitle: g.title,
      title: ["Mise à jour 1.4 : nouveau mode coopératif", "Patch notes : équilibrage et corrections", "Événement d'automne disponible", "Feuille de route 2027", "Soldes : -60 % ce week-end"][i],
      url: "https://store.steampowered.com/news/",
      date: new Date(Date.now() - i * 1.7e8).toISOString(),
      feed: "Community Announcements",
      excerpt: "Une nouvelle mise à jour arrive avec son lot de nouveautés, d'améliorations de performances et de corrections attendues par la communauté.",
    }));
let mockUpdateState = "idle";
const mockUpdate = () => ({
  currentVersion: "1.1.1",
  available: params.get("update") ? { version: "1.2.0", notes: "- Nouveautés de démonstration\n- Corrections", date: null } : null,
  autoCheck: mockState.settings.auto_update_check !== "0",
  state: mockUpdateState,
  lastCheck: null,
  lastCheckFailed: false,
});
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
    return [0, 1, 2].map((i) => ({ id: `ps${i}`, source: g.sources[0].source, startedAt: new Date(Date.now() - (i + 1) * 9e7).toISOString(), endedAt: new Date(Date.now() - (i + 1) * 9e7 + 3.6e6).toISOString(), durationSeconds: Math.floor(g.trackedSeconds / 3), active: false, origin: i === 1 ? "app" : "detected", note: i === 1 ? "Boss du chapitre 3 battu" : null }));
  },
  delete_session: () => undefined,
  open_url: () => undefined,
  get_free_games: () => ({
    games: [
      { id: "1", title: "Lanternes du Nord", url: "https://store.epicgames.com/fr/free-games", image: null, start: new Date(Date.now() - 3 * 864e5).toISOString(), end: new Date(Date.now() + 4 * 864e5).toISOString(), current: true },
      { id: "2", title: "Cités de Verre", url: "https://store.epicgames.com/fr/free-games", image: null, start: new Date(Date.now() + 4 * 864e5).toISOString(), end: new Date(Date.now() + 11 * 864e5).toISOString(), current: false },
    ],
    checkedAt: new Date().toISOString(),
  }),
  get_game_news: (a: Args) => mockNews().filter((_, i) => i < 4).map((n) => ({ ...n, gameId: String(a.gameId) })),
  get_news_feed: () => mockNews(),
  get_screenshots: () => [],
  set_igdb_credentials: () => undefined,
  get_sync_info: () => ({ folder: null, auto: false, last: null, remote: null }),
  sync_now: () => ({ appVersion: "1.5.0", exportedAt: new Date().toISOString(), machine: "PC-DEMO", games: mockState.games.length, sessions: 420 }),
  restore_from_folder: () => ({ appVersion: "1.5.0", exportedAt: new Date().toISOString(), machine: "PC-DEMO", games: mockState.games.length, sessions: 420 }),
  set_session_note: () => notify(),
  read_image_data_url: () => "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
  read_text_file: () => { throw { code: "FILE_NOT_FOUND", detail: "" }; },
  write_text_file: () => undefined,
  // Démonstration : `?update=1` simule une version disponible.
  get_update_info: () => mockUpdate(),
  check_for_updates: () => ({ ...mockUpdate(), lastCheck: new Date().toISOString() }),
  install_update: () => {
    mockUpdateState = running.length > 0 ? "waiting-game" : "downloading";
    mockEmit("update-state", mockUpdateState);
  },
  update_session: () => notify(),
  stop_tracking: (a: Args) => {
    running = running.filter((id) => id !== a.gameId);
    mockEmit("running-changed", running);
  },
  force_close_game: (a: Args) => {
    // Démonstration : un jeu sur deux « résiste » pour montrer le message d'erreur.
    if (String(a.gameId).length % 2 === 0) throw { code: "KILL_FAILED", detail: "vgc.exe" };
    running = running.filter((id) => id !== a.gameId);
    mockEmit("running-changed", running);
    return 2;
  },
  update_game: (a: Args) => {
    const patch = a.patch as Record<string, unknown>;
    mockState.games = mockState.games.map((g) => {
      if (g.id !== a.gameId) return g;
      const next = { ...g, ...patch } as Game;
      if (typeof patch.manualPlaytimeMinutes === "number") Object.assign(next, { playtimeMinutes: patch.manualPlaytimeMinutes, manualBaselineMinutes: patch.manualPlaytimeMinutes, manualBaselineAt: new Date().toISOString(), sinceManualSeconds: 0 });
      if (patch.manualPlaytimeMinutes === null) Object.assign(next, { manualBaselineMinutes: null, manualBaselineAt: null, sinceManualSeconds: 0 });
      return next;
    });
    notify();
  },
  update_source: (a: Args) => {
    const patch = a.patch as Record<string, unknown>;
    if (typeof patch.executablePathOverride === "string" && !patch.executablePathOverride.toLowerCase().endsWith(".exe")) throw { code: "NOT_AN_EXE", detail: patch.executablePathOverride };
    let warning: string | null = null;
    mockState.games = mockState.games.map((g) => ({
      ...g,
      sources: g.sources.map((s) => {
        if (s.id !== a.sourceId) return s;
        const next = { ...s, installPathOverride: (patch.installPathOverride as string | null) ?? null, executablePathOverride: (patch.executablePathOverride as string | null) ?? null };
        if (s.source === "local") Object.assign(next, { launchArgs: patch.launchArgs || null, workingDir: patch.workingDir || null });
        else Object.assign(next, { launchMode: patch.launchMode === "exe" ? "exe" : null, directArgs: patch.launchArgs || null, directWorkingDir: patch.workingDir || null });
        const dir = (next.installPathOverride ?? next.installPath ?? "").toLowerCase();
        if (next.executablePathOverride && dir && !next.executablePathOverride.toLowerCase().startsWith(dir)) warning = "EXE_OUTSIDE_DIR";
        return next;
      }),
    }));
    notify();
    return { warning };
  },
  remove_game: (a: Args) => {
    mockState.games = mockState.games.filter((g) => g.id !== a.gameId);
    notify();
  },
});

let accounts = { steamId: null as string | null, steamKeySet: false, localSteamId: "76561198000000001", sgdbKeySet: false, igdbSet: false };

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

let collections = [{ id: "c1", name: "Coop", count: 0 }];
let sourcesEnabled: Record<string, boolean> = {};
let autostart = false;

Object.assign(mockHandlers, {
  get_stats: () => {
    const visible = mockState.games.filter((g) => !g.hidden);
    const bySource = new Map<string, { source: string; minutes: number; games: number }>();
    for (const g of visible) {
      const s = g.sources[0].source;
      const e = bySource.get(s) ?? { source: s, minutes: 0, games: 0 };
      e.minutes += g.playtimeMinutes;
      e.games += 1;
      bySource.set(s, e);
    }
    const weeks = Array.from({ length: 12 }, (_, i) => ({ start: new Date(Date.now() - (11 - i) * 6.048e8).toISOString().slice(0, 10), seconds: Math.round(Math.abs(Math.sin(i * 1.7)) * 40000) }));
    const months = Array.from({ length: 12 }, (_, i) => ({ start: new Date(2026, i, 1, 12).toISOString().slice(0, 10), seconds: Math.round(Math.abs(Math.cos(i)) * 150000) }));
    return {
      totalMinutes: visible.reduce((n, g) => n + g.playtimeMinutes, 0),
      trackedSeconds: visible.reduce((n, g) => n + g.trackedSeconds, 0),
      gamesTotal: visible.length,
      gamesInstalled: visible.filter((g) => g.installed).length,
      gamesPlayed: visible.filter((g) => g.playtimeMinutes > 0).length,
      achievementsUnlocked: visible.reduce((n, g) => n + g.achUnlocked, 0),
      achievementsTotal: visible.reduce((n, g) => n + g.achTotal, 0),
      bySource: [...bySource.values()].sort((x, y) => y.minutes - x.minutes),
      topGames: [...visible].sort((x, y) => y.playtimeMinutes - x.playtimeMinutes).slice(0, 10).map((g) => ({ gameId: g.id, title: g.title, coverPath: null, minutes: g.playtimeMinutes })),
      weekly: weeks,
      monthly: months,
      recentAchievements: visible.filter((g) => g.achUnlocked > 0).slice(0, 6).map((g, i) => ({ gameId: g.id, gameTitle: g.title, name: `Succès n° ${i + 1}`, icon: null, unlockedAt: new Date(Date.now() - i * 2e8).toISOString() })),
    };
  },
  get_dashboard: (a: Args) => mockDashboard(a.period as Period, (a.filter ?? {}) as StatsFilter),
  get_activity: (a: Args) => mockActivity(a.year as number | undefined, a.gameId as string | undefined),
  get_activity_day: (a: Args) => mockDay(String(a.day), a.gameId as string | undefined),
  get_streaks: () => mockStreaks(),
  get_sources: () =>
    (["steam", "local", "epic", "riot", "battlenet", "gog", "ubisoft", "ea", "xbox"] as SourceId[]).map((source) => ({
      source,
      available: !["gog", "ea"].includes(source),
      enabled: sourcesEnabled[source] ?? true,
      capabilities: { installed: true, owned: source === "steam" || source === "epic", playtime: source === "steam", achievements: source === "steam" },
      installedGames: mockState.games.filter((g) => g.installed && g.sources[0].source === source).length,
      totalGames: mockState.games.filter((g) => g.sources[0].source === source).length,
    })),
  set_source_enabled: (a: Args) => void (sourcesEnabled = { ...sourcesEnabled, [String(a.source)]: Boolean(a.enabled) }),
  get_settings: () => mockState.settings,
  set_setting: (a: Args) => void (mockState.settings = { ...mockState.settings, [String(a.key)]: String(a.value) }),
  get_autostart: () => autostart,
  set_autostart: (a: Args) => void (autostart = Boolean(a.enabled)),
  export_backup: () => undefined,
  import_backup: () => notify(),
  get_logs: () => "2026-11-13 15:00:00 [INFO] démarrage (backend simulé)\n2026-11-13 15:00:00 [INFO] steam : 19 jeu(x) installé(s), 0 nouveau(x)",
  open_data_folder: () => undefined,
  get_collections: () => collections.map((c) => ({ ...c, count: mockState.games.filter((g) => g.collections.includes(c.id)).length })),
  create_collection: (a: Args) => {
    const id = `c${Date.now()}`;
    collections = [...collections, { id, name: String(a.name), count: 0 }];
    notify();
    return id;
  },
  rename_collection: (a: Args) => {
    collections = collections.map((c) => (c.id === a.id ? { ...c, name: String(a.name) } : c));
    notify();
  },
  delete_collection: (a: Args) => {
    collections = collections.filter((c) => c.id !== a.id);
    notify();
  },
  set_game_collection: (a: Args) => {
    mockState.games = mockState.games.map((g) =>
      g.id !== a.gameId ? g : { ...g, collections: a.member ? [...g.collections, String(a.collectionId)] : g.collections.filter((c) => c !== a.collectionId) },
    );
    notify();
  },
  get_merge_suggestions: () => [],
  merge_games: () => notify(),
  dismiss_merge: () => undefined,
});

export async function mockCall(cmd: string, args: Args): Promise<unknown> {
  const handler = mockHandlers[cmd];
  if (!handler) throw { code: "INTERNAL", detail: `commande simulée absente : ${cmd}` };
  return handler(args);
}

// --- Statistiques simulées (V1.3) ---

const iso = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
const dayOffset = (n: number) => {
  const d = new Date();
  d.setDate(d.getDate() - n);
  return iso(d);
};
/** Historique fictif : jour → [identifiant du jeu, secondes][]. */
const mockHistory: Map<string, [string, number][]> = (() => {
  const map = new Map<string, [string, number][]>();
  let seed = 7;
  const r = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  const played = mockState.games.filter((g) => g.trackedSeconds > 0).slice(0, 14);
  for (let n = 420; n >= 0; n--) {
    const weekend = [0, 6].includes(new Date(Date.now() - n * 864e5).getDay());
    if (n > 3 && r() < (weekend ? 0.18 : 0.38)) continue;
    const count = 1 + Math.floor(r() * (weekend ? 3 : 2));
    map.set(
      dayOffset(n),
      Array.from({ length: count }, () => [played[Math.floor(r() * played.length)]?.id ?? "g0", Math.round(900 + r() * (weekend ? 14000 : 7000))] as [string, number]),
    );
  }
  return map;
})();

function mockStreaks() {
  let current = 0;
  const todayPlayed = mockHistory.has(dayOffset(0));
  for (let n = todayPlayed ? 0 : 1; mockHistory.has(dayOffset(n)); n++) current++;
  let best = 0;
  let run = 0;
  let bestEnd = 0;
  for (let n = 420; n >= 0; n--) {
    run = mockHistory.has(dayOffset(n)) ? run + 1 : 0;
    if (run > best) {
      best = run;
      bestEnd = n;
    }
  }
  return { current, currentStart: dayOffset(current - (todayPlayed ? 1 : 0)), inDanger: !todayPlayed && current > 0, todayPlayed, best, bestStart: dayOffset(bestEnd + best - 1), bestEnd: dayOffset(bestEnd), jokersAvailable: 1, jokerDays: [] };
}

function mockActivity(year?: number, gameId?: string) {
  const today = dayOffset(0);
  const start = year ? `${year}-01-01` : dayOffset(364);
  const end = year ? `${year}-12-31` : today;
  const days = [...mockHistory.entries()]
    .filter(([d]) => d >= start && d <= end)
    .map(([day, list]) => ({ day, seconds: list.filter(([g]) => !gameId || g === gameId).reduce((n, [, s]) => n + s, 0) }))
    .filter((d) => d.seconds > 0);
  const years = [...new Set([...mockHistory.keys()].map((d) => Number(d.slice(0, 4))))].sort((a, b) => b - a);
  return { start, end, today, days, years, minMinutes: 15 };
}

function mockDay(day: string, gameId?: string) {
  const list = (mockHistory.get(day) ?? []).filter(([g]) => !gameId || g === gameId);
  const games = list.map(([id, seconds]) => {
    const g = mockState.games.find((x) => x.id === id);
    return { gameId: id, title: g?.title ?? "Jeu", coverPath: g?.coverPath ?? null, seconds };
  });
  let t = new Date(`${day}T19:00:00`).getTime();
  const sessions = games.map((g, i) => {
    const s = { id: `${day}-${i}`, gameId: g.gameId, gameTitle: g.title, startedAt: new Date(t).toISOString(), endedAt: new Date(t + g.seconds * 1000).toISOString(), durationSeconds: g.seconds, origin: i % 2 ? "app" : "detected", note: null };
    t += g.seconds * 1000 + 900_000;
    return s;
  });
  return { day, seconds: games.reduce((n, g) => n + g.seconds, 0), games, sessions };
}

function mockDashboard(period: Period, filter: StatsFilter) {
  const since = period === "7d" ? dayOffset(6) : period === "30d" ? dayOffset(29) : period === "year" ? `${new Date().getFullYear()}-01-01` : null;
  const keep = (id: string) => {
    const g = mockState.games.find((x) => x.id === id);
    return !!g && (!filter.gameId || filter.gameId === id) && (!filter.source || g.sources[0].source === filter.source);
  };
  const perGame = new Map<string, number>();
  const habits = Array.from({ length: 7 }, () => Array(24).fill(0) as number[]);
  for (const [day, list] of mockHistory) {
    if (since && day < since) continue;
    for (const [id, secs] of list) {
      if (!keep(id)) continue;
      perGame.set(id, (perGame.get(id) ?? 0) + secs);
      const wd = (new Date(`${day}T12:00:00`).getDay() + 6) % 7;
      const h = 17 + (secs % 6);
      habits[wd][h] += secs * 0.6;
      habits[wd][(h + 1) % 24] += secs * 0.4;
    }
  }
  const sessionSeconds = [...perGame.values()].reduce((a, b) => a + b, 0);
  const visible = mockState.games.filter((g) => !g.hidden && keep(g.id));
  const top = period === "all"
    ? [...visible].sort((x, y) => y.playtimeMinutes - x.playtimeMinutes).slice(0, 10).map((g) => ({ gameId: g.id, title: g.title, coverPath: g.coverPath, minutes: g.playtimeMinutes }))
    : [...perGame.entries()].sort((a, b) => b[1] - a[1]).slice(0, 10).map(([id, s]) => { const g = mockState.games.find((x) => x.id === id)!; return { gameId: id, title: g.title, coverPath: g.coverPath, minutes: Math.round(s / 60) }; });
  const bySourceMap = new Map<SourceId, { source: SourceId; minutes: number; games: number }>();
  const genreMap = new Map<string, { genre: string; minutes: number; games: number }>();
  for (const t of top) {
    const g = mockState.games.find((x) => x.id === t.gameId)!;
    const s = bySourceMap.get(g.sources[0].source) ?? { source: g.sources[0].source, minutes: 0, games: 0 };
    s.minutes += t.minutes;
    s.games += 1;
    bySourceMap.set(s.source, s);
    for (const genre of g.genres) {
      const e = genreMap.get(genre) ?? { genre, minutes: 0, games: 0 };
      e.minutes += t.minutes;
      e.games += 1;
      genreMap.set(genre, e);
    }
  }
  const weekly = Array.from({ length: 12 }, (_, i) => {
    let seconds = 0;
    for (let k = 0; k < 7; k++) for (const [id, s] of mockHistory.get(dayOffset((11 - i) * 7 + k)) ?? []) if (keep(id)) seconds += s;
    return { start: dayOffset((11 - i) * 7 + 6), seconds };
  });
  const monthly = Array.from({ length: 12 }, (_, i) => {
    const d = new Date();
    d.setMonth(d.getMonth() - (11 - i), 1);
    const prefix = iso(d).slice(0, 7);
    let seconds = 0;
    for (const [day, list] of mockHistory) if (day.startsWith(prefix)) for (const [id, s] of list) if (keep(id)) seconds += s;
    return { start: `${prefix}-01`, seconds };
  });
  const withAch = visible.filter((g) => g.achTotal > 0);
  const st = mockStreaks();
  return {
    period,
    since,
    totalMinutes: period === "all" ? visible.reduce((n, g) => n + g.playtimeMinutes, 0) : Math.round(sessionSeconds / 60),
    sessionSeconds,
    gamesPlayed: period === "all" ? visible.filter((g) => g.playtimeMinutes > 0).length : perGame.size,
    streaks: st,
    daysPlayedThisYear: [...mockHistory.keys()].filter((d) => d.startsWith(String(new Date().getFullYear()))).length,
    weekly,
    monthly,
    bySource: [...bySourceMap.values()].sort((a, b) => b.minutes - a.minutes),
    byGenre: [...genreMap.values()].sort((a, b) => b.minutes - a.minutes),
    topGames: top,
    habits: habits.map((row) => row.map(Math.round)),
    records: {
      longestSession: top[0] ? { value: 19800, label: "Plus longue session", date: dayOffset(40), gameId: top[0].gameId, gameTitle: top[0].title } : null,
      biggestDay: { value: 28800, label: "Plus grosse journée", date: dayOffset(12), gameId: null, gameTitle: null },
      bestWeek: top[1] ? { value: 61200, label: "Jeu le plus joué en une semaine", date: dayOffset(60), gameId: top[1].gameId, gameTitle: top[1].title } : null,
      latestHour: top[2] ? { value: 3 * 60 + 12, label: "Heure de jeu la plus tardive", date: dayOffset(90), gameId: top[2].gameId, gameTitle: top[2].title } : null,
    },
    achievements: {
      unlocked: withAch.reduce((n, g) => n + g.achUnlocked, 0),
      total: withAch.reduce((n, g) => n + g.achTotal, 0),
      completion: withAch.length ? (withAch.reduce((n, g) => n + g.achUnlocked / g.achTotal, 0) / withAch.length) * 100 : 0,
      perfectGames: withAch.filter((g) => g.achUnlocked === g.achTotal).length,
      rarest: withAch.slice(0, 5).map((g, i) => ({ gameId: g.id, gameTitle: g.title, name: ["Sans une égratignure", "Collectionneur fou", "Speedrun", "Pacifiste", "Perfection"][i], icon: null, percent: [0.4, 1.2, 2.8, 4.1, 6.5][i] })),
      almostDone: withAch.filter((g) => g.achUnlocked / g.achTotal >= 0.8 && g.achUnlocked < g.achTotal).slice(0, 6).map((g) => ({ gameId: g.id, title: g.title, coverPath: g.coverPath, unlocked: g.achUnlocked, total: g.achTotal })),
    },
    badges: [
      { id: "night-owl", name: "Noctambule", description: "Jouer après 3 h du matin.", icon: "🦉", earned: true },
      { id: "marathon", name: "Marathonien", description: "Une session de plus de 6 heures.", icon: "🏃", earned: false },
      { id: "loyal", name: "Fidèle", description: "Une série de 30 jours joués d'affilée.", icon: "🔥", earned: st.best >= 30 },
      { id: "collector", name: "Collectionneur", description: "100 jeux dans la bibliothèque.", icon: "📚", earned: mockState.games.length >= 100 },
      { id: "explorer", name: "Touche-à-tout", description: "Jouer sur 5 launchers différents.", icon: "🧭", earned: true },
      { id: "completionist", name: "Complétionniste", description: "Tous les succès d'un jeu.", icon: "🏆", earned: withAch.some((g) => g.achUnlocked === g.achTotal) },
    ],
  };
}

