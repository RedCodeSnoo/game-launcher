// Recherche, filtres et tris de la bibliothèque (F-NAV-02/03/04). Fonctions pures, testées.
import type { Game, SourceId } from "../types";

export type Section =
  | { kind: "all" }
  | { kind: "installed" }
  | { kind: "favorites" }
  | { kind: "recent" }
  | { kind: "hidden" }
  | { kind: "source"; source: SourceId }
  | { kind: "collection"; id: string };

export type SortKey = "name" | "lastPlayed" | "playtime" | "added" | "achievements";

export interface Filters {
  search: string;
  installed: "all" | "installed" | "notInstalled";
  sources: SourceId[];
  drive: string | null;
  genre: string | null;
  sort: SortKey;
  sortDesc: boolean;
}

export const DEFAULT_FILTERS: Filters = {
  search: "",
  installed: "all",
  sources: [],
  drive: null,
  genre: null,
  sort: "name",
  sortDesc: false,
};

/** Minuscules, sans accents ni ponctuation : « Pokémon™ » correspond à « pokemon ». */
export function normalize(text: string): string {
  return text
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

export function matchesSearch(game: Game, search: string): boolean {
  const q = normalize(search);
  if (!q) return true;
  const title = normalize(game.title);
  return q.split(" ").every((word) => title.includes(word));
}

function inSection(game: Game, section: Section): boolean {
  if (section.kind === "hidden") return game.hidden;
  if (game.hidden) return false;
  switch (section.kind) {
    case "all":
      return true;
    case "installed":
      return game.installed;
    case "favorites":
      return game.favorite;
    case "recent":
      return game.lastPlayed !== null;
    case "source":
      return game.sources.some((s) => s.source === section.source);
    case "collection":
      return game.collections.includes(section.id);
  }
}

function achievementRatio(g: Game): number {
  return g.achTotal > 0 ? g.achUnlocked / g.achTotal : -1;
}

const comparators: Record<SortKey, (a: Game, b: Game) => number> = {
  name: (a, b) => a.sortTitle.localeCompare(b.sortTitle, "fr"),
  lastPlayed: (a, b) => (a.lastPlayed ?? "").localeCompare(b.lastPlayed ?? ""),
  playtime: (a, b) => a.playtimeMinutes - b.playtimeMinutes,
  added: (a, b) => a.addedAt.localeCompare(b.addedAt),
  achievements: (a, b) => achievementRatio(a) - achievementRatio(b),
};

export function applyFilters(games: Game[], section: Section, filters: Filters): Game[] {
  const out = games.filter((g) => {
    if (!inSection(g, section)) return false;
    if (!matchesSearch(g, filters.search)) return false;
    if (filters.installed === "installed" && !g.installed) return false;
    if (filters.installed === "notInstalled" && g.installed) return false;
    if (filters.sources.length > 0 && !g.sources.some((s) => filters.sources.includes(s.source))) return false;
    if (filters.drive && !g.sources.some((s) => s.installed && s.drive === filters.drive)) return false;
    if (filters.genre && !g.genres.includes(filters.genre)) return false;
    return true;
  });
  // « Récents » impose son propre ordre tant que l'utilisateur n'a pas choisi un tri.
  const key: SortKey = section.kind === "recent" && filters.sort === "name" ? "lastPlayed" : filters.sort;
  const desc = section.kind === "recent" && filters.sort === "name" ? true : filters.sortDesc;
  const cmp = comparators[key];
  out.sort((a, b) => {
    const primary = desc ? cmp(b, a) : cmp(a, b);
    return primary !== 0 ? primary : comparators.name(a, b);
  });
  return out;
}

export function listDrives(games: Game[]): string[] {
  const set = new Set<string>();
  for (const g of games) for (const s of g.sources) if (s.installed && s.drive) set.add(s.drive);
  return [...set].sort();
}

export function listGenres(games: Game[]): string[] {
  const set = new Set<string>();
  for (const g of games) for (const genre of g.genres) set.add(genre);
  return [...set].sort((a, b) => a.localeCompare(b, "fr"));
}

export function listSources(games: Game[]): SourceId[] {
  const set = new Set<SourceId>();
  for (const g of games) if (!g.hidden) for (const s of g.sources) set.add(s.source);
  return [...set];
}
