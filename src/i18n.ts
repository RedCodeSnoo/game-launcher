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
  "game.favorite": "Favori",
  "game.hide": "Masquer",
  "game.unhide": "Ne plus masquer",
  "game.playtime": "Temps de jeu",
  "game.tracked": "dont {t} suivi par l'application",
  "game.trackedOnly": "suivi par l'application",
  "game.lastSession": "Dernière session",
  "game.lastPlayed": "Dernier lancement",
  "game.achievements": "Succès",
  "tab.achievements": "Succès",
  "tab.sessions": "Sessions",
  "tab.info": "Infos",
  "tab.settings": "Paramètres du jeu",
  "add.title": "Ajouter un jeu",
  "add.button": "Ajouter",
  "err.NOT_AN_EXE": "Ce fichier n'est pas un exécutable (.exe).",
  "err.ALREADY_IN_LIBRARY": "Ce jeu est déjà dans la bibliothèque.",
  "err.FOLDER_NOT_FOUND": "Dossier introuvable.",
  "err.INSTALL_UNSUPPORTED": "L'installation depuis l'application n'est pas disponible pour cette source : ouvrez son launcher.",
  "err.NOT_EDITABLE": "Seuls les jeux locaux ont des paramètres de lancement modifiables.",
  "err.REMOVE_LAUNCHER_GAME": "Ce jeu vient d'un launcher : il réapparaîtrait à la prochaine détection. Masquez-le plutôt.",
  "err.OFFLINE": "Pas de connexion Internet : les données locales restent disponibles.",
  "err.STEAM_PRIVATE": "Profil Steam privé : rendez publics votre profil et les « Détails des jeux » (Steam > Profil > Modifier le profil > Confidentialité).",
  "err.STEAM_BAD_KEY": "Clé API Steam refusée. Vérifiez-la sur steamcommunity.com/dev/apikey.",
  "err.STEAM_BAD_ID": "SteamID introuvable. Saisissez votre SteamID64 (17 chiffres) ou l'adresse de votre profil.",
  "err.STEAM_KEY_REQUIRED": "Saisissez votre clé API Steam.",
  "err.STEAM_NOT_CONFIGURED": "Le compte Steam n'est pas configuré (Paramètres > Comptes).",
  "err.SGDB_BAD_KEY": "Clé SteamGridDB refusée.",
  "err.RATE_LIMITED": "Trop de requêtes : le service demande de patienter.",
  "err.SERVICE_UNAVAILABLE": "Le service est momentanément indisponible.",
  "err.BAD_RESPONSE": "Réponse inattendue du service.",
  "err.NOT_AN_IMAGE": "Ce fichier n'est pas une image PNG, JPEG ou WebP (20 Mo maximum).",
  "err.BACKUP_INVALID": "Ce fichier n'est pas une sauvegarde de GameLauncher.",
  "err.BACKUP_WHILE_PLAYING": "Restauration impossible pendant qu'un jeu est en cours.",
  "err.BAD_NAME": "Nom invalide (1 à 60 caractères).",
  "err.NAME_TAKEN": "Ce nom est déjà utilisé.",
  "err.GAME_NOT_FOUND": "Jeu introuvable dans la bibliothèque.",
  "err.NOT_INSTALLED": "Ce jeu n'est pas installé.",
  "err.DISK_UNPLUGGED": "Le disque de ce jeu n'est pas connecté.",
  "err.LAUNCHER_MISSING": "Le launcher nécessaire est introuvable ou n'est pas installé.",
  "err.FILE_NOT_FOUND": "Fichier introuvable : l'exécutable du jeu a été déplacé ou supprimé.",
  "err.LAUNCH_FAILED": "Le lancement a échoué.",
  "err.KILL_FAILED": "Certains processus du jeu n'ont pas pu être fermés (droits insuffisants ou protection anti-triche).",
  "err.BAD_SESSION_END": "Heure de fin invalide : elle doit être après le début de la session et ne pas être dans le futur.",
  "err.SESSION_NOT_FOUND": "Session introuvable.",
  "err.AUTOSTART_FAILED": "Impossible de modifier le démarrage avec Windows.",
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
