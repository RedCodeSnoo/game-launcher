import { describe, expect, it } from "vitest";
import { moveInTop, parseTop, placeInTop, removeFromTop } from "./profile";

const known = new Set(["a", "b", "c", "d"]);

describe("top 10 du profil", () => {
  it("toujours dix emplacements, sans inconnu ni doublon", () => {
    expect(parseTop(undefined, known)).toEqual(Array(10).fill(null));
    expect(parseTop("pas du json", known)).toHaveLength(10);
    const t = parseTop(JSON.stringify(["a", "zz", "a", null, "b"]), known);
    expect(t.slice(0, 5)).toEqual(["a", null, null, null, "b"]);
  });
  it("placer, déplacer, retirer", () => {
    let t = parseTop("[]", known);
    t = placeInTop(t, "a", 0);
    t = placeInTop(t, "b", 1);
    t = placeInTop(t, "c", 2);
    expect(t.slice(0, 3)).toEqual(["a", "b", "c"]);
    // Le n° 3 passe en tête, les autres descendent.
    expect(moveInTop(t, 2, 0).slice(0, 3)).toEqual(["c", "a", "b"]);
    // Placer un jeu déjà présent le déplace.
    expect(placeInTop(t, "a", 5).slice(0, 6)).toEqual([null, "b", "c", null, null, "a"]);
    expect(removeFromTop(t, 1).slice(0, 3)).toEqual(["a", null, "c"]);
    expect(moveInTop(t, 0, 9)).toHaveLength(10);
  });
});
