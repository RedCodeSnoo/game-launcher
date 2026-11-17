import { describe, expect, it } from "vitest";
import type { Game } from "../types";
import { fuzzyScore, inAutoCollection, pick, rouletteCandidates, searchGames, DEFAULT_ROULETTE } from "./organize";

const NOW = new Date("2026-11-18T12:00:00Z").getTime();
const daysAgo = (n: number) => new Date(NOW - n * 86_400_000).toISOString();

function game(title: string, over: Partial<Game> = {}): Game {
  return {
    id: title, title, sortTitle: title.toLowerCase(), coverPath: null, heroPath: null, logoPath: null, description: null, genres: [], developer: null,
    releaseDate: null, favorite: false, hidden: false, addedAt: daysAgo(400), sources: [], installed: true, available: true, playtimeMinutes: 0,
    trackedSeconds: 0, manualBaselineMinutes: null, manualBaselineAt: null, sinceManualSeconds: 0, sinceSyncSeconds: 0, noTracking: false,
    status: null, statusChangedAt: null, rating: null, notes: null, lastPlayed: null, lastSessionSeconds: null, achTotal: 0, achUnlocked: 0, collections: [],
    ...over,
  };
}

describe("collections automatiques", () => {
  it("pas joué depuis 6 mois, presque 100 %, ajoutés récemment", () => {
    expect(inAutoCollection(game("A", { lastPlayed: daysAgo(200) }), "stale", NOW)).toBe(true);
    expect(inAutoCollection(game("B", { lastPlayed: daysAgo(20) }), "stale", NOW)).toBe(false);
    expect(inAutoCollection(game("C"), "stale", NOW)).toBe(false);
    expect(inAutoCollection(game("D", { achTotal: 50, achUnlocked: 42 }), "almost", NOW)).toBe(true);
    expect(inAutoCollection(game("E", { achTotal: 50, achUnlocked: 50 }), "almost", NOW)).toBe(false);
    expect(inAutoCollection(game("F", { addedAt: daysAgo(3) }), "new", NOW)).toBe(true);
    expect(inAutoCollection(game("G", { addedAt: daysAgo(3), hidden: true }), "new", NOW)).toBe(false);
  });
});

describe("roulette", () => {
  const games = [
    game("Installé backlog", { status: "backlog", genres: ["RPG"], timeToBeatMain: 600 }),
    game("Pas installé", { installed: false, status: "backlog" }),
    game("Disque absent", { available: false }),
    game("Joué hier", { lastPlayed: daysAgo(1), genres: ["RPG"] }),
    game("Long", { timeToBeatMain: 3000 }),
  ];
  it("ne tire que des jeux installés et disponibles, avec les filtres", () => {
    expect(rouletteCandidates(games, DEFAULT_ROULETTE, NOW).map((g) => g.title)).toEqual(["Installé backlog", "Joué hier", "Long"]);
    expect(rouletteCandidates(games, { ...DEFAULT_ROULETTE, backlogOnly: true }, NOW).map((g) => g.title)).toEqual(["Installé backlog"]);
    expect(rouletteCandidates(games, { ...DEFAULT_ROULETTE, genre: "RPG", notPlayed30: true }, NOW).map((g) => g.title)).toEqual(["Installé backlog"]);
    expect(rouletteCandidates(games, { ...DEFAULT_ROULETTE, shortOnly: true }, NOW).map((g) => g.title)).toEqual(["Installé backlog"]);
  });
  it("tirage", () => {
    expect(pick(["a", "b", "c"], 0)).toBe("a");
    expect(pick(["a", "b", "c"], 0.99)).toBe("c");
    expect(pick([], 0.5)).toBeNull();
  });
});

describe("recherche rapide", () => {
  it("préfère le début du titre, les initiales, puis les lettres dans l'ordre", () => {
    expect(fuzzyScore("hol", "Hollow Knight")).toBeGreaterThan(fuzzyScore("hol", "Death Hollow"));
    expect(fuzzyScore("ffvii", "Final Fantasy VII")).toBeGreaterThan(0);
    expect(fuzzyScore("lol", "League of Legends")).toBeGreaterThan(fuzzyScore("lol", "Lollipop Chainsaw") - 1000);
    expect(fuzzyScore("xyz", "Hades")).toBe(0);
    expect(fuzzyScore("pokemon", "Pokémon™ Violet")).toBeGreaterThan(0);
  });
  it("classe les jeux et ignore les jeux masqués", () => {
    const games = [game("Hades"), game("Hades II", { hidden: true }), game("Shadow Tactics"), game("Hollow Knight")];
    expect(searchGames(games, "had").map((g) => g.title)).toEqual(["Hades", "Shadow Tactics"]);
    expect(searchGames(games, "hk").map((g) => g.title)[0]).toBe("Hollow Knight");
  });
});
