// Internationalisation minimale : toutes les chaînes de l'interface passent par `t()`.
// Pour ajouter une langue : créer un dictionnaire de même forme et l'enregistrer dans `dictionaries`.
import type { AppError, SourceId } from "./types";

const fr = {
  "app.name": "GameLauncher",
  "nav.all": "Tous les jeux",
  "nav.installed": "Installés",
  "nav.favorites": "Favoris",
  "nav.recent": "Récents",
  "nav.hidden": "Masqués",
  "nav.sources": "Sources",
  "nav.collections": "Collections",
  "nav.stats": "Statistiques",
  "nav.settings": "Paramètres",
  "lib.search": "Rechercher un jeu…",
  "lib.refresh": "Actualiser",
  "lib.refreshing": "Détection en cours…",
  "lib.grid": "Vue grille",
  "lib.list": "Vue liste",
  "lib.empty": "Aucun jeu à afficher",
  "lib.emptyHint": "Modifiez les filtres, ou lancez une détection avec « Actualiser ».",
  "lib.count": "{n} jeu(x)",
  "game.play": "Jouer",
  "game.install": "Installer",
  "game.unavailable": "Indisponible",
  "game.unavailableLong": "Indisponible – disque non connecté",
  "game.running": "En cours",
  "game.installed": "Installé",
  "game.notInstalled": "Non installé",
  "game.noAchievements": "Aucun succès",
  "common.back": "Retour",
  "common.cancel": "Annuler",
  "common.save": "Enregistrer",
  "common.add": "Ajouter",
  "common.remove": "Retirer",
  "common.close": "Fermer",
  "common.never": "Jamais",
  "err.GAME_NOT_FOUND": "Jeu introuvable dans la bibliothèque.",
  "err.NOT_INSTALLED": "Ce jeu n'est pas installé.",
  "err.DISK_UNPLUGGED": "Le disque de ce jeu n'est pas connecté.",
  "err.LAUNCHER_MISSING": "Le launcher nécessaire est introuvable ou n'est pas installé.",
  "err.FILE_NOT_FOUND": "Fichier introuvable : l'exécutable du jeu a été déplacé ou supprimé.",
  "err.LAUNCH_FAILED": "Le lancement a échoué.",
  "err.INTERNAL": "Une erreur interne est survenue.",
  "err.DATABASE": "Erreur de base de données.",
  "err.IO": "Erreur de lecture ou d'écriture de fichier.",
} as const;

export type Key = keyof typeof fr;
const dictionaries: Record<string, Record<string, string>> = { fr };
let current: Record<string, string> = fr;

export function setLanguage(lang: string) {
  current = dictionaries[lang] ?? fr;
}

export function t(key: Key, vars?: Record<string, string | number>): string {
  let s: string = current[key] ?? fr[key] ?? key;
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.replace(`{${k}}`, String(v));
  return s;
}

export function errorMessage(e: unknown): string {
  const err = e as Partial<AppError> | undefined;
  if (err && typeof err.code === "string") {
    const key = `err.${err.code}`;
    const base = current[key] ?? fr["err.INTERNAL"];
    return err.detail ? `${base} (${err.detail})` : base;
  }
  return typeof e === "string" ? e : fr["err.INTERNAL"];
}

export const SOURCE_LABELS: Record<SourceId, string> = {
  steam: "Steam",
  epic: "Epic Games",
  gog: "GOG",
  battlenet: "Battle.net",
  riot: "Riot Games",
  ubisoft: "Ubisoft Connect",
  ea: "EA app",
  xbox: "Xbox",
  local: "Jeux locaux",
};

export const SOURCE_COLORS: Record<SourceId, string> = {
  steam: "#1b6fb5",
  epic: "#4a4a4f",
  gog: "#8a3fc4",
  battlenet: "#0f8fd6",
  riot: "#d13639",
  ubisoft: "#2f6fe0",
  ea: "#e8503a",
  xbox: "#187c18",
  local: "#6b7287",
};
