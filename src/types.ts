// Types partagés avec le backend Rust (sérialisés en camelCase).

export type SourceId = "steam" | "epic" | "gog" | "battlenet" | "riot" | "ubisoft" | "ea" | "xbox" | "local";

export type GameStatus = "backlog" | "playing" | "finished" | "completed" | "abandoned" | "paused";

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
  /** Base manuelle saisie (minutes) et date de la saisie ; les sessions suivantes s'y ajoutent. */
  manualBaselineMinutes: number | null;
  manualBaselineAt: string | null;
  /** Sessions qui commencent après la saisie manuelle (secondes). */
  sinceManualSeconds: number;
  sinceSyncSeconds: number;
  noTracking: boolean;
  status: GameStatus | null;
  statusChangedAt: string | null;
  rating: number | null;
  notes: string | null;
  /** Durée de vie (minutes) : histoire principale et complétion, si connue (V1.5). */
  timeToBeatMain?: number | null;
  timeToBeatFull?: number | null;
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
  note: string | null;
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
  status?: GameStatus | null;
  rating?: number | null;
  notes?: string | null;
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

export interface UpdateAvailable {
  version: string;
  notes: string | null;
  date: string | null;
}

export interface UpdateInfo {
  currentVersion: string;
  available: UpdateAvailable | null;
  autoCheck: boolean;
  /** idle, waiting-game, downloading, installing, error */
  state: string;
  lastCheck: string | null;
  lastCheckFailed: boolean;
}

// --- Statistiques et séries (V1.3) ---

export type Period = "7d" | "30d" | "year" | "all";

export interface StatsFilter {
  gameId?: string;
  source?: SourceId;
}

export interface Streaks {
  current: number;
  currentStart: string | null;
  inDanger: boolean;
  todayPlayed: boolean;
  best: number;
  bestStart: string | null;
  bestEnd: string | null;
  jokersAvailable: number;
  jokerDays: string[];
}

export interface StatRecord {
  value: number;
  label: string;
  date: string | null;
  gameId: string | null;
  gameTitle: string | null;
}

export interface Badge {
  id: string;
  name: string;
  description: string;
  icon: string;
  earned: boolean;
}

export interface Dashboard {
  period: Period;
  since: string | null;
  totalMinutes: number;
  sessionSeconds: number;
  gamesPlayed: number;
  streaks: Streaks;
  daysPlayedThisYear: number;
  weekly: Bucket[];
  monthly: Bucket[];
  bySource: SourceTime[];
  byGenre: { genre: string; minutes: number; games: number }[];
  topGames: { gameId: string; title: string; coverPath: string | null; minutes: number }[];
  habits: number[][];
  records: { longestSession: StatRecord | null; biggestDay: StatRecord | null; bestWeek: StatRecord | null; latestHour: StatRecord | null };
  achievements: {
    unlocked: number;
    total: number;
    completion: number;
    perfectGames: number;
    rarest: { gameId: string; gameTitle: string; name: string; icon: string | null; percent: number }[];
    almostDone: { gameId: string; title: string; coverPath: string | null; unlocked: number; total: number }[];
  };
  badges: Badge[];
}

export interface ActivityGrid {
  start: string;
  end: string;
  today: string;
  days: { day: string; seconds: number }[];
  years: number[];
  minMinutes: number;
}

export interface DayDetail {
  day: string;
  seconds: number;
  games: { gameId: string; title: string; coverPath: string | null; seconds: number }[];
  sessions: { id: string; gameId: string; gameTitle: string; startedAt: string; endedAt: string; durationSeconds: number; origin: "app" | "detected"; note: string | null }[];
}
