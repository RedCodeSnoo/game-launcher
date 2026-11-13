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
  trackedSeconds: number;
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
}

export interface SourcePatch {
  executablePath?: string;
  launchArgs?: string;
  workingDir?: string;
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
