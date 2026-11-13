// Types partagés avec le backend Rust (sérialisés en camelCase).

export type SourceId = "steam" | "epic" | "gog" | "battlenet" | "riot" | "ubisoft" | "ea" | "xbox" | "local";

export type LaunchMethod = "uri" | "launcher_cli" | "exe";

export interface GameSourceView {
  id: string;
  gameId: string;
  source: SourceId;
  externalId: string;
  installed: boolean;
  installPath: string | null;
  volumeSerial: string | null;
  executablePath: string | null;
  launchMethod: LaunchMethod;
  launchTarget: string;
  launchArgs: string | null;
  workingDir: string | null;
  sourcePlaytimeMinutes: number | null;
  lastPlayedSource: string | null;
  available: boolean;
  owned: boolean;
  /** Valeurs choisies par l'utilisateur ; elles priment sur les valeurs détectées (E2). */
  installPathOverride: string | null;
  executablePathOverride: string | null;
  /** "exe" = exécutable direct ; "launcher" ou null = launcher d'origine. */
  launchMode: "exe" | "launcher" | null;
  directArgs: string | null;
  directWorkingDir: string | null;
  trackedSeconds: number;
  /** Temps joué depuis la dernière lecture du temps de la source (secondes). */
  sinceSyncSeconds: number;
  playtimeMinutes: number;
  drive: string | null;
}

export interface Game {
  id: string;
  title: string;
  sortTitle: string;
  coverPath: string | null;
  heroPath: string | null;
  logoPath: string | null;
  description: string | null;
  genres: string[];
  developer: string | null;
  releaseDate: string | null;
  favorite: boolean;
  hidden: boolean;
  addedAt: string;
  sources: GameSourceView[];
  installed: boolean;
  available: boolean;
  playtimeMinutes: number;
  trackedSeconds: number;
  manualPlaytimeMinutes: number | null;
  sinceSyncSeconds: number;
  noTracking: boolean;
  lastPlayed: string | null;
  lastSessionSeconds: number | null;
  achTotal: number;
  achUnlocked: number;
  collections: string[];
}

export interface Session {
  id: string;
  source: SourceId;
  startedAt: string;
  endedAt: string;
  durationSeconds: number;
  active: boolean;
  origin: "app" | "detected";
}

export interface WatchedFolder {
  id: string;
  path: string;
  exists: boolean;
}

export interface Candidate {
  name: string;
  folder: string | null;
  exe: string;
  size: number;
  alternatives: string[];
}

export interface GamePatch {
  title?: string;
  favorite?: boolean;
  hidden?: boolean;
  manualPlaytimeMinutes?: number | null;
  noTracking?: boolean;
}

export interface SourcePatch {
  /** `null` = rétablir la valeur détectée. */
  installPathOverride?: string | null;
  executablePathOverride?: string | null;
  launchMode?: "exe" | "launcher";
  launchArgs?: string;
  workingDir?: string;
}

export interface Achievement {
  apiName: string;
  name: string;
  description: string;
  iconUnlocked: string | null;
  iconLocked: string | null;
  hidden: boolean;
  unlocked: boolean;
  unlockedAt: string | null;
  globalPercent: number | null;
}

export interface Accounts {
  steamId: string | null;
  steamKeySet: boolean;
  localSteamId: string | null;
  sgdbKeySet: boolean;
}

export interface StoreMatch {
  appid: number;
  name: string;
  image: string | null;
}

export interface SyncError {
  source: SourceId;
  code: string;
  detail: string;
}

export interface SyncState {
  running: boolean;
  errors?: SyncError[];
}

export interface SourceTime {
  source: SourceId;
  minutes: number;
  games: number;
}

export interface Bucket {
  start: string;
  seconds: number;
}

export interface Stats {
  totalMinutes: number;
  trackedSeconds: number;
  gamesTotal: number;
  gamesInstalled: number;
  gamesPlayed: number;
  achievementsUnlocked: number;
  achievementsTotal: number;
  bySource: SourceTime[];
  topGames: { gameId: string; title: string; coverPath: string | null; minutes: number }[];
  weekly: Bucket[];
  monthly: Bucket[];
  recentAchievements: { gameId: string; gameTitle: string; name: string; icon: string | null; unlockedAt: string }[];
}

export interface SourceStatus {
  source: SourceId;
  available: boolean;
  enabled: boolean;
  capabilities: { installed: boolean; owned: boolean; playtime: boolean; achievements: boolean };
  installedGames: number;
  totalGames: number;
}

export interface Collection {
  id: string;
  name: string;
  count: number;
}

export interface MergeSuggestion {
  title: string;
  gameIds: string[];
}

export interface AppError {
  code: string;
  detail: string;
}

export interface ScanStats {
  found: number;
  added: number;
  uninstalled: number;
  unavailable: number;
}

export interface SourceReport {
  source: SourceId;
  available: boolean;
  enabled: boolean;
  error: string | null;
  stats: ScanStats | null;
}

export interface ScanState {
  running: boolean;
  reports?: SourceReport[];
}
