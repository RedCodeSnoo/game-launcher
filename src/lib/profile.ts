// Top 10 des jeux préférés du profil : dix emplacements numérotés, vides au départ.
import { reorder } from "./drag";

export const TOP_SIZE = 10;
export type Top = (string | null)[];

/** Lit le réglage `profile_top10` : toujours dix emplacements, sans jeu inconnu ni doublon. */
export function parseTop(raw: string | undefined, known: Set<string>): Top {
  let list: unknown = [];
  try {
    list = JSON.parse(raw ?? "[]");
  } catch {
    list = [];
  }
  const seen = new Set<string>();
  const out: Top = [];
  for (let i = 0; i < TOP_SIZE; i++) {
    const v = Array.isArray(list) ? list[i] : null;
    if (typeof v === "string" && known.has(v) && !seen.has(v)) {
      seen.add(v);
      out.push(v);
    } else {
      out.push(null);
    }
  }
  return out;
}

/** Place un jeu dans un emplacement (s'il était déjà dans le top, il quitte son ancienne place). */
export function placeInTop(top: Top, gameId: string, slot: number): Top {
  const out = top.map((v) => (v === gameId ? null : v));
  out[slot] = gameId;
  return out;
}

/** Déplace le jeu de l'emplacement `from` vers `to` ; les autres se décalent pour lui faire place. */
export function moveInTop(top: Top, from: number, to: number): Top {
  return reorder(top, from, to);
}

export function removeFromTop(top: Top, slot: number): Top {
  return top.map((v, i) => (i === slot ? null : v));
}
