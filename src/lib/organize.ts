// Organisation de la bibliothèque (V1.4) : statuts, collections automatiques, roulette, recherche rapide.
// Fonctions pures, testées.
import type { Game, GameStatus } from "../types";
import { normalize } from "./filter";

export const STATUSES: { id: GameStatus; label: string; short: string; color: string; emoji: string }[] = [
  { id: "backlog", label: "À jouer", short: "À jouer", color: "var(--accent)", emoji: "📋" },
  { id: "playing", label: "En cours", short: "En cours", color: "var(--ok)", emoji: "🎮" },
  { id: "paused", label: "En pause", short: "Pause", color: "var(--warn)", emoji: "⏸️" },
  { id: "finished", label: "Terminé", short: "Terminé", color: "color-mix(in srgb, var(--ok) 60%, var(--accent))", emoji: "🏁" },
  { id: "completed", label: "100 %", short: "100 %", color: "var(--warn)", emoji: "🏆" },
  { id: "abandoned", label: "Abandonné", short: "Abandonné", color: "var(--muted)", emoji: "🪦" },
];

export function statusInfo(id: GameStatus | null | undefined) {
  return STATUSES.find((s) => s.id === id) ?? null;
}

export type AutoCollection = "stale" | "almost" | "new";

export const AUTO_COLLECTIONS: { id: AutoCollection; label: string; hint: string }[] = [
  { id: "stale", label: "Pas joué depuis 6 mois", hint: "Joués un jour, oubliés depuis." },
  { id: "almost", label: "Presque 100 %", hint: "80 % des succès ou plus." },
  { id: "new", label: "Ajoutés récemment", hint: "Arrivés dans la bibliothèque ces 30 derniers jours." },
];

const DAY = 86_400_000;

export function inAutoCollection(g: Game, id: AutoCollection, now = Date.now()): boolean {
  if (g.hidden) return false;
  switch (id) {
    case "stale":
      return g.lastPlayed !== null && now - new Date(g.lastPlayed).getTime() > 182 * DAY;
    case "almost":
      return g.achTotal > 0 && g.achUnlocked / g.achTotal >= 0.8 && g.achUnlocked < g.achTotal;
    case "new":
      return now - new Date(g.addedAt).getTime() <= 30 * DAY;
  }
}

// ---------------------------------------------------------------------------
// Roulette « Je sais pas à quoi jouer »
// ---------------------------------------------------------------------------

export interface RouletteOptions {
  backlogOnly: boolean;
  /** Jeux courts : durée de vie connue de 15 h au plus (histoire principale). */
  shortOnly: boolean;
  genre: string | null;
  notPlayed30: boolean;
}

export const DEFAULT_ROULETTE: RouletteOptions = { backlogOnly: false, shortOnly: false, genre: null, notPlayed30: false };

export function rouletteCandidates(games: Game[], o: RouletteOptions, now = Date.now()): Game[] {
  return games.filter((g) => {
    if (g.hidden || !g.installed || !g.available) return false;
    if (o.backlogOnly && g.status !== "backlog") return false;
    if (o.genre && !g.genres.includes(o.genre)) return false;
    if (o.notPlayed30 && g.lastPlayed && now - new Date(g.lastPlayed).getTime() < 30 * DAY) return false;
    if (o.shortOnly && !(g.timeToBeatMain !== null && g.timeToBeatMain !== undefined && g.timeToBeatMain <= 15 * 60)) return false;
    return true;
  });
}

/** Tirage reproductible : `random` dans [0, 1[. */
export function pick<T>(items: T[], random: number): T | null {
  return items.length ? items[Math.min(items.length - 1, Math.floor(random * items.length))] : null;
}

// ---------------------------------------------------------------------------
// Recherche rapide (Ctrl+K)
// ---------------------------------------------------------------------------

/**
 * Score d'un texte pour une recherche : 0 si pas de correspondance. Favorise le début du titre, le début des mots,
 * puis les lettres dans l'ordre (« ffvii » trouve « Final Fantasy VII »).
 */
export function fuzzyScore(query: string, text: string): number {
  const q = normalize(query).replace(/ /g, "");
  const t = normalize(text);
  if (!q) return 1;
  if (!t) return 0;
  const compact = t.replace(/ /g, "");
  if (compact.startsWith(q)) return 1000 - t.length;
  const words = t.split(" ");
  const initials = words.map((w) => w[0]).join("");
  if (initials.startsWith(q)) return 800 - t.length;
  if (words.some((w) => w.startsWith(q))) return 700 - t.length;
  if (compact.includes(q)) return 500 - t.length;
  // Lettres dans l'ordre, bonus quand elles se suivent ou commencent un mot.
  let score = 0;
  let ti = 0;
  let streak = 0;
  for (const ch of q) {
    let found = false;
    while (ti < t.length) {
      const c = t[ti];
      const wordStart = ti === 0 || t[ti - 1] === " ";
      ti++;
      if (c === ch) {
        streak++;
        score += 2 + streak + (wordStart ? 6 : 0);
        found = true;
        break;
      }
      if (c !== " ") streak = 0;
    }
    if (!found) return 0;
  }
  return Math.max(1, score);
}

export function searchGames(games: Game[], query: string, limit = 8): Game[] {
  return games
    .filter((g) => !g.hidden)
    .map((g) => ({ g, base: fuzzyScore(query, g.title) }))
    .filter((x) => x.base > 0 && (query.trim() === "" ? x.g.lastPlayed !== null : true))
    .map((x) => ({ g: x.g, s: x.base + (x.g.installed ? 5 : 0) + (x.g.favorite ? 3 : 0) }))
    .sort((a, b) => b.s - a.s || (b.g.lastPlayed ?? "").localeCompare(a.g.lastPlayed ?? ""))
    .slice(0, limit)
    .map((x) => x.g);
}
