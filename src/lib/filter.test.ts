import { describe, expect, it } from "vitest";
import type { Game, GameSourceView, SourceId } from "../types";
import { applyFilters, DEFAULT_FILTERS, listDrives, listGenres, matchesSearch, normalize } from "./filter";
import { formatMinutes, formatRelative, formatSeconds, percent } from "./format";

function source(source: SourceId, over: Partial<GameSourceView> = {}): GameSourceView {
  return {
    id: "s",
    gameId: "g",
    source,
    externalId: "1",
    installed: true,
    installPath: "C:\\Jeux\\X",
    volumeSerial: null,
    executablePath: null,
    launchMethod: "uri",
    launchTarget: "",
    launchArgs: null,
    workingDir: null,
    sourcePlaytimeMinutes: null,
    lastPlayedSource: null,
    available: true,
    owned: false,
    installPathOverride: null,
    executablePathOverride: null,
    launchMode: null,
    directArgs: null,
    directWorkingDir: null,
    trackedSeconds: 0,
    sinceSyncSeconds: 0,
    playtimeMinutes: 0,
    drive: "C:",
    ...over,
  };
}

function game(title: string, over: Partial<Game> = {}): Game {
  return {
    id: title,
    title,
    sortTitle: title.toLowerCase(),
    coverPath: null,
    heroPath: null,
    logoPath: null,
    description: null,
    genres: [],
    developer: null,
    releaseDate: null,
    favorite: false,
    hidden: false,
    addedAt: "2026-01-01T00:00:00Z",
    sources: [source("steam")],
    installed: true,
    available: true,
    playtimeMinutes: 0,
    trackedSeconds: 0,
    manualPlaytimeMinutes: null,
    sinceSyncSeconds: 0,
    noTracking: false,
    lastPlayed: null,
    lastSessionSeconds: null,
    achTotal: 0,
    achUnlocked: 0,
    collections: [],
    ...over,
  };
}

const games: Game[] = [
  game("Zelda Like", { playtimeMinutes: 10, genres: ["Aventure"], lastPlayed: "2026-05-01T10:00:00Z", achTotal: 10, achUnlocked: 5 }),
  game("Alpha Racing", { playtimeMinutes: 300, favorite: true, genres: ["Course"], addedAt: "2026-03-01T00:00:00Z" }),
  game("Pokémon™ Clone", { installed: false, sources: [source("gog", { installed: false, drive: null })], lastPlayed: "2026-06-01T10:00:00Z", achTotal: 4, achUnlocked: 4 }),
  game("Outil Caché", { hidden: true }),
  game("Disque Externe", { sources: [source("local", { drive: "E:" })], collections: ["c1"] }),
];

const titles = (list: Game[]) => list.map((g) => g.title);

describe("recherche", () => {
  it("ignore la casse, les accents et les symboles", () => {
    expect(normalize("Pokémon™: Écarlate")).toBe("pokemon ecarlate");
    expect(matchesSearch(games[2], "pokemon")).toBe(true);
    expect(matchesSearch(games[2], "clone poke")).toBe(true);
    expect(matchesSearch(games[2], "zelda")).toBe(false);
    expect(matchesSearch(games[2], "  ")).toBe(true);
  });
});

describe("sections et filtres", () => {
  it("masque les jeux cachés partout sauf dans « Masqués »", () => {
    expect(titles(applyFilters(games, { kind: "all" }, DEFAULT_FILTERS))).not.toContain("Outil Caché");
    expect(titles(applyFilters(games, { kind: "hidden" }, DEFAULT_FILTERS))).toEqual(["Outil Caché"]);
  });

  it("filtre par section", () => {
    expect(titles(applyFilters(games, { kind: "favorites" }, DEFAULT_FILTERS))).toEqual(["Alpha Racing"]);
    expect(titles(applyFilters(games, { kind: "installed" }, DEFAULT_FILTERS))).not.toContain("Pokémon™ Clone");
    expect(titles(applyFilters(games, { kind: "source", source: "gog" }, DEFAULT_FILTERS))).toEqual(["Pokémon™ Clone"]);
    expect(titles(applyFilters(games, { kind: "collection", id: "c1" }, DEFAULT_FILTERS))).toEqual(["Disque Externe"]);
  });

  it("classe les récents du plus récent au plus ancien", () => {
    expect(titles(applyFilters(games, { kind: "recent" }, DEFAULT_FILTERS))).toEqual(["Pokémon™ Clone", "Zelda Like"]);
  });

  it("filtre par source, état, disque et genre", () => {
    const f = { ...DEFAULT_FILTERS };
    expect(titles(applyFilters(games, { kind: "all" }, { ...f, sources: ["local"] }))).toEqual(["Disque Externe"]);
    expect(titles(applyFilters(games, { kind: "all" }, { ...f, installed: "notInstalled" }))).toEqual(["Pokémon™ Clone"]);
    expect(titles(applyFilters(games, { kind: "all" }, { ...f, drive: "E:" }))).toEqual(["Disque Externe"]);
    expect(titles(applyFilters(games, { kind: "all" }, { ...f, genre: "Course" }))).toEqual(["Alpha Racing"]);
    expect(titles(applyFilters(games, { kind: "all" }, { ...f, search: "a", genre: "Aventure" }))).toEqual(["Zelda Like"]);
  });
});

describe("tris", () => {
  const sorted = (sort: typeof DEFAULT_FILTERS.sort, sortDesc: boolean) =>
    titles(applyFilters(games, { kind: "all" }, { ...DEFAULT_FILTERS, sort, sortDesc }));

  it("trie par nom", () => {
    expect(sorted("name", false)).toEqual(["Alpha Racing", "Disque Externe", "Pokémon™ Clone", "Zelda Like"]);
  });
  it("trie par temps de jeu décroissant", () => {
    expect(sorted("playtime", true).slice(0, 2)).toEqual(["Alpha Racing", "Zelda Like"]);
  });
  it("trie par dernier lancement et date d'ajout", () => {
    expect(sorted("lastPlayed", true)[0]).toBe("Pokémon™ Clone");
    expect(sorted("added", true)[0]).toBe("Alpha Racing");
  });
  it("trie par progression des succès, les jeux sans succès en dernier", () => {
    expect(sorted("achievements", true)).toEqual(["Pokémon™ Clone", "Zelda Like", "Alpha Racing", "Disque Externe"]);
  });
});

describe("listes de valeurs", () => {
  it("liste les disques et les genres", () => {
    expect(listDrives(games)).toEqual(["C:", "E:"]);
    expect(listGenres(games)).toEqual(["Aventure", "Course"]);
  });
});

describe("mise en forme", () => {
  it("formate les durées", () => {
    expect(formatMinutes(0)).toBe("0 min");
    expect(formatMinutes(45)).toBe("45 min");
    expect(formatMinutes(120)).toBe("2 h");
    expect(formatMinutes(135)).toBe("2 h 15");
    expect(formatMinutes(6005)).toBe("100 h 05");
    expect(formatSeconds(30)).toBe("30 s");
    expect(formatSeconds(3600)).toBe("1 h");
  });
  it("formate les dates relatives", () => {
    const now = new Date(2026, 10, 13, 15, 0, 0);
    expect(formatRelative(null, now)).toBe("Jamais");
    expect(formatRelative(new Date(2026, 10, 13, 1, 0, 0).toISOString(), now)).toBe("Aujourd'hui");
    expect(formatRelative(new Date(2026, 10, 12, 23, 0, 0).toISOString(), now)).toBe("Hier");
    expect(formatRelative(new Date(2026, 10, 3, 12, 0, 0).toISOString(), now)).toBe("Il y a 10 jours");
  });
  it("calcule les pourcentages", () => {
    expect(percent(1, 3)).toBe(33);
    expect(percent(0, 0)).toBe(0);
  });
});
