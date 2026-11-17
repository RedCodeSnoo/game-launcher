// Fiche d'un jeu : bannière, bouton Jouer, temps de jeu, onglets Succès / Sessions / Infos / Paramètres.
import { useCallback, useEffect, useState, type ReactNode } from "react";
import { ArrowLeft, Check, Download, Eye, EyeOff, FolderOpen, HardDrive, Heart, Pencil, Play, Square, Trash2, X, XOctagon } from "lucide-react";
import { api, imageUrl } from "../api";
import { errorMessage, SOURCE_LABELS, t } from "../i18n";
import { formatDate, formatDateTime, formatLooseDate, formatMinutes, formatRelative, formatSeconds, hueOf, percent } from "../lib/format";
import type { Game, GameSourceView, Session } from "../types";
import { Cover, placeholderGradient, SourceBadge } from "./Cover";
import { LocationSettings } from "./LocationSettings";
import { useToasts } from "./Toasts";
import { Button, Field, ProgressBar, Tabs, TextInput } from "./ui";

export type DetailTab = "achievements" | "sessions" | "info" | "settings";

interface Props {
  game: Game;
  isRunning: boolean;
  onBack: () => void;
  onPlay: (game: Game) => void;
  /** « Arrêter le suivi » : clôt la session sans fermer le jeu (E1-01). */
  onStop: (game: Game) => void;
  /** « Forcer la fermeture du jeu » (E1-02). */
  onForceClose: (game: Game) => void;
  /** Contenu de l'onglet Succès (fourni par l'application). */
  achievements?: ReactNode;
  /** Réglages supplémentaires de l'onglet Paramètres (jaquette, fiche…). */
  extraSettings?: ReactNode;
  headerExtra?: ReactNode;
}

function Stat({ label, value, sub, title }: { label: string; value: ReactNode; sub?: ReactNode; title?: string }) {
  return (
    <div className="min-w-0 rounded-lg border border-line bg-panel px-4 py-3" title={title}>
      <p className="text-xs uppercase tracking-wide text-muted">{label}</p>
      <p className="mt-0.5 truncate text-lg font-semibold">{value}</p>
      {sub && <div className="mt-0.5 text-xs text-muted">{sub}</div>}
    </div>
  );
}

/** Valeur d'un champ `datetime-local` (heure locale) pour une date ISO. */
function toLocalInput(iso: string): string {
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function SessionsTab({ game }: { game: Game }) {
  const toasts = useToasts();
  const [sessions, setSessions] = useState<Session[] | null>(null);
  const [editing, setEditing] = useState<{ id: string; value: string } | null>(null);
  const saveEnd = async () => {
    if (!editing) return;
    const date = new Date(editing.value);
    if (Number.isNaN(date.getTime())) return toasts.error("Date invalide.");
    try {
      await api.updateSession(editing.id, date.toISOString());
      setEditing(null);
      toasts.success("Heure de fin corrigée");
      await load();
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };
  const load = useCallback(() => api.getSessions(game.id).then(setSessions).catch((e) => toasts.error(errorMessage(e))), [game.id, toasts]);
  // `trackedSeconds` change quand une session se termine : on recharge alors l'historique.
  useEffect(() => void load(), [load, game.trackedSeconds]);

  if (!sessions) return <div className="skeleton h-24 rounded-lg" />;
  if (sessions.length === 0)
    return <p className="py-8 text-center text-sm text-muted">Aucune session enregistrée. Les sessions sont suivies tant que GameLauncher tourne, même si le jeu est lancé depuis son launcher ou un raccourci.</p>;
  return (
    <table className="w-full text-sm">
      <thead className="text-left text-xs uppercase tracking-wide text-muted">
        <tr>
          <th className="py-2 font-medium">Début</th>
          <th className="py-2 font-medium">Fin</th>
          <th className="py-2 font-medium">Durée</th>
          <th className="py-2 font-medium">Source</th>
          <th className="py-2 font-medium">Origine</th>
          <th />
        </tr>
      </thead>
      <tbody>
        {sessions.map((s) => (
          <tr key={s.id} className="border-t border-line">
            <td className="py-2">{formatDateTime(s.startedAt)}</td>
            <td className="py-2">
              {s.active ? (
                <span className="font-medium text-ok">{t("game.running")}</span>
              ) : editing?.id === s.id ? (
                <span className="flex items-center gap-1.5">
                  <input
                    type="datetime-local"
                    aria-label="Heure de fin de la session"
                    className="h-8 rounded-md border border-line bg-bg px-2 text-sm"
                    value={editing.value}
                    min={toLocalInput(s.startedAt)}
                    onChange={(e) => setEditing({ id: s.id, value: e.target.value })}
                    onKeyDown={(e) => e.key === "Enter" && void saveEnd()}
                    autoFocus
                  />
                  <button onClick={() => void saveEnd()} className="text-ok hover:brightness-125" aria-label="Enregistrer l'heure de fin" title="Enregistrer">
                    <Check size={16} />
                  </button>
                  <button onClick={() => setEditing(null)} className="text-muted hover:text-fg" aria-label="Annuler" title="Annuler">
                    <X size={16} />
                  </button>
                </span>
              ) : (
                formatDateTime(s.endedAt)
              )}
            </td>
            <td className="py-2 tabular-nums">{formatSeconds(s.durationSeconds)}</td>
            <td className="py-2 text-muted">{SOURCE_LABELS[s.source]}</td>
            <td className="py-2 text-muted">{s.origin === "detected" ? "Détectée" : "Lancée depuis l'app"}</td>
            <td className="py-2 text-right">
              {!s.active && editing?.id !== s.id && (
                <button onClick={() => setEditing({ id: s.id, value: toLocalInput(s.endedAt) })} className="mr-3 text-muted hover:text-fg" aria-label="Modifier l'heure de fin" title="Modifier l'heure de fin">
                  <Pencil size={14} />
                </button>
              )}
              {!s.active && (
                <button
                  onClick={async () => {
                    if (!window.confirm("Supprimer cette session de l'historique ?")) return;
                    await api.deleteSession(s.id);
                    await load();
                  }}
                  className="text-muted hover:text-danger"
                  aria-label="Supprimer cette session"
                  title="Supprimer cette session"
                >
                  <Trash2 size={14} />
                </button>
              )}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function InfoRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid grid-cols-[10rem_minmax(0,1fr)] gap-3 border-t border-line py-2 text-sm first:border-t-0">
      <dt className="text-muted">{label}</dt>
      <dd className="selectable min-w-0 break-words">{children}</dd>
    </div>
  );
}

function sourceState(s: GameSourceView): string {
  if (!s.installed) return t("game.notInstalled");
  if (!s.available) return t("game.unavailableLong");
  return t("game.installed");
}

function InfoTab({ game }: { game: Game }) {
  const toasts = useToasts();
  return (
    <div className="flex flex-col gap-5">
      {game.description && <p className="selectable max-w-3xl whitespace-pre-line text-sm leading-relaxed text-fg/90">{game.description}</p>}
      <dl>
        {game.genres.length > 0 && <InfoRow label="Genres">{game.genres.join(", ")}</InfoRow>}
        {game.developer && <InfoRow label="Développeur">{game.developer}</InfoRow>}
        {game.releaseDate && <InfoRow label="Date de sortie">{formatLooseDate(game.releaseDate)}</InfoRow>}
        <InfoRow label="Ajouté le">{formatDate(game.addedAt)}</InfoRow>
      </dl>
      {game.sources.map((s) => (
        <div key={s.id} className="rounded-lg border border-line bg-panel p-4">
          <div className="mb-1 flex items-center gap-2">
            <SourceBadge source={s.source} />
            <span className="text-sm text-muted">{sourceState(s)}</span>
            {s.installPath && s.available && (
              <button onClick={() => api.openPath(s.installPath!).catch((e) => toasts.error(errorMessage(e)))} className="ml-auto flex items-center gap-1.5 text-xs text-accent hover:underline">
                <FolderOpen size={13} /> Ouvrir le dossier
              </button>
            )}
          </div>
          <dl>
            <InfoRow label="Identifiant">{s.externalId}</InfoRow>
            {(s.installPathOverride ?? s.installPath) && <InfoRow label="Dossier">{`${s.installPathOverride ?? s.installPath}${s.installPathOverride ? " (personnalisé)" : ""}`}</InfoRow>}
            {(s.executablePathOverride ?? s.executablePath) && <InfoRow label="Exécutable">{`${s.executablePathOverride ?? s.executablePath}${s.executablePathOverride ? " (personnalisé)" : ""}`}</InfoRow>}
            {s.drive && (
              <InfoRow label="Disque">
                {s.drive}
                {s.volumeSerial ? ` (volume ${s.volumeSerial})` : ""}
              </InfoRow>
            )}
            <InfoRow label="Lancement">{s.launchMethod === "exe" || s.launchMode === "exe" ? "Exécutable direct" : s.launchMethod === "uri" ? `Via le launcher (${s.launchTarget})` : "Ligne de commande du launcher"}</InfoRow>
            {s.sourcePlaytimeMinutes !== null && <InfoRow label="Temps (source)">{formatMinutes(s.sourcePlaytimeMinutes)}</InfoRow>}
            {s.sinceSyncSeconds >= 60 && <InfoRow label="Depuis la synchro">{`+ ${formatSeconds(s.sinceSyncSeconds)} (ajouté au total jusqu'à la prochaine synchronisation)`}</InfoRow>}
            <InfoRow label="Temps suivi">{formatSeconds(s.trackedSeconds)}</InfoRow>
          </dl>
        </div>
      ))}
    </div>
  );
}

function SettingsTab({ game, extra, onBack }: { game: Game; extra?: ReactNode; onBack: () => void }) {
  const toasts = useToasts();
  const [title, setTitle] = useState(game.title);
  const [manual, setManual] = useState(game.manualBaselineMinutes !== null ? String(Math.round((game.manualBaselineMinutes / 60) * 100) / 100) : "");
  useEffect(() => setTitle(game.title), [game.title]);

  const run = async (action: () => Promise<void>, ok: string) => {
    try {
      await action();
      toasts.success(ok);
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  return (
    <div className="flex max-w-2xl flex-col gap-6">
      <section className="flex flex-col gap-3">
        <h3 className="text-sm font-semibold">Général</h3>
        <Field label="Nom affiché">
          <div className="flex gap-2">
            <TextInput value={title} onChange={(e) => setTitle(e.target.value)} />
            <Button disabled={!title.trim() || title === game.title} onClick={() => run(() => api.updateGame(game.id, { title }), "Jeu renommé")}>
              Renommer
            </Button>
          </div>
        </Field>
        <Field
          label="Temps de jeu de départ (heures)"
          hint="Sert de base : les sessions jouées après la saisie s'y ajoutent. Une nouvelle saisie remplace la base. Laissez vide pour revenir au temps calculé."
        >
          <div className="flex gap-2">
            <TextInput value={manual} onChange={(e) => setManual(e.target.value)} inputMode="decimal" placeholder="ex. 12,5" className="max-w-40" />
            <Button
              onClick={() => {
                const hours = Number(manual.replace(",", "."));
                const value = manual.trim() === "" ? null : Math.round(hours * 60);
                if (value !== null && !Number.isFinite(hours)) return toasts.error("Nombre d'heures invalide.");
                void run(() => api.updateGame(game.id, { manualPlaytimeMinutes: value }), value === null ? "Base supprimée" : "Temps de départ enregistré");
              }}
            >
              Appliquer
            </Button>
          </div>
        </Field>
      </section>

      {extra}

      {game.sources
        .filter((src) => src.installed)
        .map((src) => (
          <LocationSettings key={src.id} source={src} />
        ))}

      <section className="flex flex-col gap-3">
        <h3 className="text-sm font-semibold">Suivi du temps de jeu</h3>
        <div className="flex items-center justify-between gap-4">
          <p className="text-sm text-muted">
            {game.noTracking
              ? "Le temps de ce jeu n'est pas suivi : aucune session n'est ouverte quand il tourne."
              : "Une session est ouverte dès que ce jeu tourne. À désactiver pour un logiciel qui reste ouvert en permanence."}
          </p>
          <Button onClick={() => run(() => api.updateGame(game.id, { noTracking: !game.noTracking }), game.noTracking ? "Suivi réactivé" : "Suivi désactivé pour ce jeu")}>
            {game.noTracking ? "Réactiver le suivi" : "Ne plus suivre"}
          </Button>
        </div>
      </section>

      <section className="flex flex-col gap-3">
        <h3 className="text-sm font-semibold">Bibliothèque</h3>
        <div className="flex gap-2">
          <Button onClick={() => run(() => api.updateGame(game.id, { hidden: !game.hidden }), game.hidden ? "Jeu de nouveau visible" : "Jeu masqué")}>
            {game.hidden ? <Eye size={15} /> : <EyeOff size={15} />}
            {game.hidden ? t("game.unhide") : t("game.hide")}
          </Button>
          {game.sources.every((s) => s.source === "local" || (!s.installed && !s.owned)) && (
            <Button
              variant="danger"
              onClick={async () => {
                if (!window.confirm(`Retirer « ${game.title} » de la bibliothèque ? Son historique de sessions sera supprimé. Les fichiers du jeu ne sont pas touchés.`)) return;
                await run(() => api.removeGame(game.id), "Jeu retiré de la bibliothèque");
                onBack();
              }}
            >
              <Trash2 size={15} /> Retirer de la bibliothèque
            </Button>
          )}
        </div>
      </section>
    </div>
  );
}

/** Détail du temps total, affiché au survol : « 2300 h saisies le 15/11 + 3 h 10 suivies depuis ». */
export function playtimeDetail(game: Game): string {
  if (game.manualBaselineMinutes !== null) {
    const day = game.manualBaselineAt ? new Date(game.manualBaselineAt).toLocaleDateString("fr-FR", { day: "2-digit", month: "2-digit" }) : "?";
    return `${formatMinutes(game.manualBaselineMinutes)} saisies le ${day} + ${formatSeconds(game.sinceManualSeconds)} suivies depuis`;
  }
  const fromSource = game.sources.filter((s) => s.sourcePlaytimeMinutes !== null);
  if (fromSource.length > 0) {
    const imported = fromSource.reduce((n, s) => n + (s.sourcePlaytimeMinutes ?? 0), 0);
    return `${formatMinutes(imported)} d'après ${fromSource.map((s) => SOURCE_LABELS[s.source]).join(", ")} + ${formatSeconds(game.sinceSyncSeconds)} suivies depuis la dernière synchronisation`;
  }
  return `${formatSeconds(game.trackedSeconds)} suivies par l'application`;
}

export function GameDetail({ game, isRunning, onBack, onPlay, onStop, onForceClose, achievements, extraSettings, headerExtra }: Props) {
  const toasts = useToasts();
  const [tab, setTab] = useState<DetailTab>("achievements");
  const unavailable = game.installed && !game.available;
  const hero = imageUrl(game.heroPath);
  const logo = imageUrl(game.logoPath);
  const hue = hueOf(game.title);
  const hasSourceTime = game.sources.some((s) => s.sourcePlaytimeMinutes !== null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement;
      if (e.key === "Escape" || (e.key === "Backspace" && !typing)) onBack();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onBack]);

  return (
    <div className="h-full overflow-y-auto">
      <div
        className="relative h-72 w-full bg-cover bg-center"
        style={{ backgroundImage: hero ? `url("${hero}")` : placeholderGradient(hue, 120) }}
      >
        <div className="absolute inset-0 bg-gradient-to-t from-bg via-bg/50 to-transparent" />
        <button onClick={onBack} className="absolute left-5 top-4 flex items-center gap-1.5 rounded-md bg-scrim/45 px-2.5 py-1.5 text-sm text-on-scrim backdrop-blur hover:bg-scrim/65">
          <ArrowLeft size={16} /> {t("common.back")}
        </button>
        <div className="absolute inset-x-6 bottom-4 flex items-end gap-5">
          <div className={`h-44 w-[7.35rem] shrink-0 overflow-hidden rounded-lg shadow-xl ring-1 ring-on-scrim/10 ${unavailable ? "grayscale" : ""}`}>
            <Cover title={game.title} path={game.coverPath} />
          </div>
          <div className="min-w-0 flex-1 pb-1">
            {logo ? <img src={logo} alt={game.title} className="mb-2 max-h-20 max-w-80 object-contain object-left drop-shadow-lg" draggable={false} /> : <h1 className="truncate text-3xl font-bold drop-shadow">{game.title}</h1>}
            <div className="mt-2 flex flex-wrap items-center gap-2">
              {game.sources.map((s) => (
                <SourceBadge key={s.id} source={s.source} />
              ))}
              {game.developer && <span className="text-sm text-muted">{game.developer}</span>}
            </div>
          </div>
        </div>
      </div>

      <div className="flex flex-col gap-5 px-6 pb-8 pt-3">
        <div className="flex flex-wrap items-center gap-3">
          {isRunning ? (
            <span className="inline-flex h-11 items-center gap-2 rounded-md bg-ok/20 px-6 text-base font-semibold text-ok">
              <span className="h-2.5 w-2.5 animate-pulse rounded-full bg-ok" /> {t("game.running")}
            </span>
          ) : null}
          {isRunning ? (
            <>
              <Button onClick={() => onStop(game)} title="Clôt la session maintenant, sans fermer le jeu">
                <Square size={14} className="fill-current" /> Arrêter le suivi
              </Button>
              <Button variant="danger" onClick={() => onForceClose(game)} title="Termine les processus du jeu puis clôt la session">
                <XOctagon size={16} /> Forcer la fermeture du jeu
              </Button>
            </>
          ) : unavailable ? (
            <span className="inline-flex h-11 items-center gap-2 rounded-md border border-line bg-panel2 px-5 text-base font-semibold text-muted" title="Rebranchez le disque puis cliquez sur Actualiser">
              <HardDrive size={18} /> {t("game.unavailableLong")}
            </span>
          ) : game.installed ? (
            <Button variant="primary" className="!h-11 !px-8 !text-base" onClick={() => onPlay(game)} autoFocus>
              <Play size={18} className="fill-current" /> {t("game.play")}
            </Button>
          ) : (
            <Button variant="secondary" className="!h-11 !px-6 !text-base" onClick={() => onPlay(game)} autoFocus>
              <Download size={18} /> {t("game.install")}
            </Button>
          )}
          <Button
            variant="ghost"
            aria-pressed={game.favorite}
            onClick={() => api.updateGame(game.id, { favorite: !game.favorite }).catch((e) => toasts.error(errorMessage(e)))}
            title={t("game.favorite")}
          >
            <Heart size={17} className={game.favorite ? "fill-danger text-danger" : ""} /> {t("game.favorite")}
          </Button>
          {headerExtra}
        </div>

        <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
          <Stat
            label={t("game.playtime")}
            value={formatMinutes(game.playtimeMinutes)}
            title={playtimeDetail(game)}
            sub={
              game.manualBaselineMinutes !== null
                ? t("game.manualSince", { t: formatSeconds(game.sinceManualSeconds) })
                : game.sinceSyncSeconds >= 60
                  ? t("game.sinceSync", { t: formatSeconds(game.sinceSyncSeconds) })
                  : hasSourceTime
                    ? t("game.tracked", { t: formatSeconds(game.trackedSeconds) })
                  : t("game.trackedOnly")
            }
          />
          <Stat label={t("game.lastSession")} value={game.lastSessionSeconds !== null ? formatSeconds(game.lastSessionSeconds) : "–"} />
          <Stat label={t("game.lastPlayed")} value={formatRelative(game.lastPlayed)} sub={game.lastPlayed ? formatDateTime(game.lastPlayed) : undefined} />
          <Stat
            label={t("game.achievements")}
            value={game.achTotal > 0 ? `${game.achUnlocked} / ${game.achTotal}` : t("game.noAchievements")}
            sub={game.achTotal > 0 ? <ProgressBar value={percent(game.achUnlocked, game.achTotal)} className="mt-1.5" /> : undefined}
          />
        </div>

        <Tabs
          tabs={[
            { id: "achievements", label: game.achTotal > 0 ? `${t("tab.achievements")} · ${percent(game.achUnlocked, game.achTotal)} %` : t("tab.achievements") },
            { id: "sessions", label: t("tab.sessions") },
            { id: "info", label: t("tab.info") },
            { id: "settings", label: t("tab.settings") },
          ]}
          value={tab}
          onChange={setTab}
        />
        <div>
          {tab === "achievements" && (achievements ?? <p className="py-8 text-center text-sm text-muted">{t("game.noAchievements")}</p>)}
          {tab === "sessions" && <SessionsTab game={game} />}
          {tab === "info" && <InfoTab game={game} />}
          {tab === "settings" && <SettingsTab key={game.id} game={game} extra={extraSettings} onBack={onBack} />}
        </div>
      </div>
    </div>
  );
}
