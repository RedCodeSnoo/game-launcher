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

export async function mockCall(cmd: string, args: Args): Promise<unknown> {
  const handler = mockHandlers[cmd];
  if (!handler) throw { code: "INTERNAL", detail: `commande simulée absente : ${cmd}` };
  return handler(args);
}
