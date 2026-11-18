// Page Statistiques (V1.3) : séries, grille d'activité, temps par période, répartitions, habitudes, records,
// succès et badges. Les calculs sont faits côté Rust (`dashboard.rs`).
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { Award, CalendarDays, Clock3, Flame, Moon, Settings2, Sparkles, Trophy } from "lucide-react";
import { api, on } from "../api";
import { errorMessage, SOURCE_COLORS, SOURCE_LABELS } from "../i18n";
import { formatDate, formatMinutes, formatSeconds, percent } from "../lib/format";
import type { Dashboard, Game, Period, SourceId, StatsFilter } from "../types";
import { ActivityGrid } from "./ActivityGrid";
import { HabitsHeatmap, HBars, TimeBars } from "./charts";
import { Cover } from "./Cover";
import { useToasts } from "./Toasts";
import { ProgressBar, Tabs } from "./ui";

const PERIODS: { id: Period; label: string }[] = [
  { id: "7d", label: "7 jours" },
  { id: "30d", label: "30 jours" },
  { id: "year", label: "Cette année" },
  { id: "all", label: "Tout" },
];

function Tile({ icon, label, value, sub, accent }: { icon: ReactNode; label: string; value: ReactNode; sub?: ReactNode; accent?: boolean }) {
  return (
    <div className={`card rounded-xl border bg-panel px-4 py-3.5 ${accent ? "border-accent/60" : "border-line"}`}>
      <p className="flex items-center gap-1.5 text-xs uppercase tracking-wide text-muted">
        {icon}
        {label}
      </p>
      <p className="font-display mt-1 text-2xl font-semibold tabular-nums">{value}</p>
      {sub && <div className="mt-0.5 text-xs text-muted">{sub}</div>}
    </div>
  );
}

export function Card({ title, children, note, actions }: { title: string; children: ReactNode; note?: string; actions?: ReactNode }) {
  return (
    <section className="rounded-xl border border-line bg-panel p-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold">{title}</h2>
          {note && <p className="mt-0.5 text-xs text-muted">{note}</p>}
        </div>
        {actions}
      </div>
      <div className="mt-4">{children}</div>
    </section>
  );
}

function streakDates(start: string | null, end: string | null): string {
  if (!start) return "";
  return end && end !== start ? `du ${formatDate(start)} au ${formatDate(end)}` : `le ${formatDate(start)}`;
}

function StreakSettings() {
  const toasts = useToasts();
  const [prefs, setPrefs] = useState<Record<string, string>>({});
  useEffect(() => void api.getSettings().then(setPrefs).catch(() => {}), []);
  const save = async (key: string, value: string) => {
    try {
      await api.setSetting(key, value);
      setPrefs((p) => ({ ...p, [key]: value }));
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };
  const num = "h-8 w-20 rounded-md border border-line bg-bg px-2 text-sm";
  return (
    <div className="grid gap-4 text-sm sm:grid-cols-2">
      <label className="flex items-center justify-between gap-3">
        <span>
          Seuil d'un jour joué <span className="block text-xs text-muted">Temps minimum dans la journée, tous jeux confondus.</span>
        </span>
        <span className="flex items-center gap-1.5">
          <input type="number" min={1} max={600} className={num} value={prefs.streak_min_minutes ?? "15"} onChange={(e) => setPrefs((p) => ({ ...p, streak_min_minutes: e.target.value }))} onBlur={(e) => void save("streak_min_minutes", String(Math.max(1, Math.min(600, Number(e.target.value) || 15))))} />
          min
        </span>
      </label>
      <label className="flex items-center justify-between gap-3">
        <span>
          Heure de bascule <span className="block text-xs text-muted">Une partie à 1 h du matin compte pour la veille.</span>
        </span>
        <select className="h-8 rounded-md border border-line bg-bg px-2" value={prefs.streak_rollover_hour ?? "4"} onChange={(e) => void save("streak_rollover_hour", e.target.value)}>
          {Array.from({ length: 13 }, (_, h) => (
            <option key={h} value={h}>
              {h} h
            </option>
          ))}
        </select>
      </label>
      <label className="flex items-center justify-between gap-3">
        <span>
          Jokers de série <span className="block text-xs text-muted">1 joker tous les 7 jours d'affilée (2 au plus), utilisé pour sauver un jour manqué.</span>
        </span>
        <input type="checkbox" className="h-4 w-4 accent-[var(--accent)]" checked={prefs.streak_jokers === "1"} onChange={(e) => void save("streak_jokers", e.target.checked ? "1" : "0")} />
      </label>
      <label className="flex items-center justify-between gap-3">
        <span>
          Rappel du soir <span className="block text-xs text-muted">Notification si la série est en danger.</span>
        </span>
        <span className="flex items-center gap-2">
          <select className="h-8 rounded-md border border-line bg-bg px-2" value={prefs.streak_reminder_hour ?? "21"} disabled={prefs.streak_reminder !== "1"} onChange={(e) => void save("streak_reminder_hour", e.target.value)}>
            {Array.from({ length: 8 }, (_, i) => 16 + i).map((h) => (
              <option key={h} value={h}>
                {h} h
              </option>
            ))}
          </select>
          <input type="checkbox" className="h-4 w-4 accent-[var(--accent)]" checked={prefs.streak_reminder === "1"} onChange={(e) => void save("streak_reminder", e.target.checked ? "1" : "0")} />
        </span>
      </label>
    </div>
  );
}

export function Stats({ onOpenGame, refreshKey, games, onRecap }: { onOpenGame: (id: string) => void; refreshKey: number; games: Game[]; onRecap?: (year: number) => void }) {
  const toasts = useToasts();
  const [period, setPeriod] = useState<Period>("30d");
  const [filter, setFilter] = useState<StatsFilter>({});
  const [data, setData] = useState<Dashboard | null>(null);
  const [bars, setBars] = useState<"weekly" | "monthly">("weekly");
  const [showSettings, setShowSettings] = useState(false);

  useEffect(() => {
    const t0 = performance.now();
    api
      .getDashboard(period, filter)
      .then((d) => {
        setData(d);
        if (import.meta.env.DEV) console.debug(`statistiques en ${Math.round(performance.now() - t0)} ms`);
      })
      .catch((e) => toasts.error(errorMessage(e)));
  }, [period, filter, refreshKey, toasts]);
  useEffect(() => on<null>("library-updated", () => void api.getDashboard(period, filter).then(setData).catch(() => {})), [period, filter]);

  const played = useMemo(() => games.filter((g) => g.trackedSeconds > 0 || g.playtimeMinutes > 0).sort((a, b) => a.sortTitle.localeCompare(b.sortTitle)), [games]);
  const sources = useMemo(() => [...new Set(games.map((g) => g.sources[0]?.source).filter(Boolean))] as SourceId[], [games]);

  if (!data) {
    return (
      <div className="h-full overflow-y-auto px-6 py-5">
        <div className="mx-auto grid max-w-6xl grid-cols-2 gap-3 lg:grid-cols-4">
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className="skeleton h-24 rounded-xl" />
          ))}
        </div>
      </div>
    );
  }
  const s = data.streaks;
  const filterValue = filter.gameId ? `game:${filter.gameId}` : filter.source ? `source:${filter.source}` : "";
  const periodNote = data.period === "all" ? "Toute la bibliothèque : heures importées et saisies à la main comprises." : `Sessions jouées depuis le ${formatDate(data.since)}.`;
  const r = data.records;

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex max-w-6xl flex-col gap-5 px-6 py-5">
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="text-lg font-semibold">Statistiques</h1>
          <div className="ml-auto flex flex-wrap items-center gap-2">
            <div role="tablist" className="flex rounded-lg border border-line bg-panel p-0.5">
              {PERIODS.map((p) => (
                <button
                  key={p.id}
                  role="tab"
                  aria-selected={period === p.id}
                  onClick={() => setPeriod(p.id)}
                  className={`rounded-md px-3 py-1 text-sm transition ${period === p.id ? "bg-accent text-accent-fg" : "text-muted hover:text-fg"}`}
                >
                  {p.label}
                </button>
              ))}
            </div>
            <select
              aria-label="Filtrer par jeu ou par launcher"
              className="h-9 max-w-56 rounded-md border border-line bg-panel px-2 text-sm"
              value={filterValue}
              onChange={(e) => {
                const [kind, id] = e.target.value.split(":");
                setFilter(kind === "game" ? { gameId: id } : kind === "source" ? { source: id as SourceId } : {});
              }}
            >
              <option value="">Tous les jeux</option>
              <optgroup label="Launcher">
                {sources.map((src) => (
                  <option key={src} value={`source:${src}`}>
                    {SOURCE_LABELS[src]}
                  </option>
                ))}
              </optgroup>
              <optgroup label="Jeu">
                {played.map((g) => (
                  <option key={g.id} value={`game:${g.id}`}>
                    {g.title}
                  </option>
                ))}
              </optgroup>
            </select>
            {onRecap && (
              <button onClick={() => onRecap(new Date().getMonth() === 11 ? new Date().getFullYear() : new Date().getFullYear() - 1)} className="flex h-9 items-center gap-1.5 rounded-md border border-line bg-panel px-3 text-sm hover:border-accent" title="Récap de l'année">
                ✨ Récap {new Date().getMonth() === 11 ? new Date().getFullYear() : new Date().getFullYear() - 1}
              </button>
            )}
            <button onClick={() => setShowSettings((v) => !v)} className="rounded-md p-2 text-muted hover:bg-panel2 hover:text-fg" aria-label="Réglages des séries" title="Réglages des séries">
              <Settings2 size={17} />
            </button>
          </div>
        </div>

        {showSettings && (
          <Card title="Réglages des séries" note="Seules les sessions réelles comptent : les heures saisies à la main n'ont pas de date.">
            <StreakSettings />
          </Card>
        )}

        <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
          <Tile icon={<Clock3 size={13} />} label="Temps de jeu" value={formatMinutes(data.totalMinutes)} sub={`${data.gamesPlayed} jeu${data.gamesPlayed > 1 ? "x" : ""} · ${PERIODS.find((p) => p.id === period)?.label.toLowerCase()}`} />
          <Tile
            icon={<Flame size={13} />}
            label="Série actuelle"
            accent={s.current > 0 && !s.inDanger}
            value={
              <span className="flex items-center gap-2">
                {s.current} <span className="text-base font-normal text-muted">jour{s.current > 1 ? "s" : ""}</span>
                {s.current > 0 && <span aria-hidden>🔥</span>}
              </span>
            }
            sub={s.inDanger ? <span className="font-medium text-warn">En danger : jouez aujourd'hui pour la garder</span> : s.current > 0 ? `depuis le ${formatDate(s.currentStart)}${s.jokersAvailable ? ` · 🃏 ${s.jokersAvailable}` : ""}` : "Jouez aujourd'hui pour en démarrer une"}
          />
          <Tile icon={<Trophy size={13} />} label="Meilleure série" value={`${s.best} jour${s.best > 1 ? "s" : ""}`} sub={streakDates(s.bestStart, s.bestEnd)} />
          <Tile icon={<CalendarDays size={13} />} label="Jours joués cette année" value={String(data.daysPlayedThisYear)} sub={`sur ${Math.ceil((Date.now() - new Date(new Date().getFullYear(), 0, 1).getTime()) / 864e5)} jours`} />
        </div>

        <Card title="Activité" note="Un jour compte dès que le seuil est atteint. Survolez une case pour le détail, cliquez pour les sessions.">
          <ActivityGrid gameId={filter.gameId} />
        </Card>

        <Card
          title="Temps de jeu"
          note="Sessions enregistrées uniquement : les heures saisies à la main et celles importées d'un launcher n'ont pas de date."
          actions={
            <Tabs
              tabs={[
                { id: "weekly", label: "Par semaine" },
                { id: "monthly", label: "Par mois" },
              ]}
              value={bars}
              onChange={setBars}
            />
          }
        >
          <TimeBars buckets={bars === "weekly" ? data.weekly : data.monthly} monthly={bars === "monthly"} />
        </Card>

        <div className="grid grid-cols-1 gap-5 lg:grid-cols-2">
          <Card title="Par launcher" note={periodNote}>
            <HBars items={data.bySource.map((x) => ({ key: x.source, label: SOURCE_LABELS[x.source], minutes: x.minutes, sub: `${x.games} jeu${x.games > 1 ? "x" : ""}`, color: SOURCE_COLORS[x.source] }))} />
          </Card>
          <Card title="Par genre" note="Le temps d'un jeu est partagé entre ses genres.">
            <HBars items={data.byGenre.map((g) => ({ key: g.genre, label: g.genre, minutes: g.minutes, sub: `${g.games} jeu${g.games > 1 ? "x" : ""}` }))} empty="Les genres apparaissent quand les fiches des jeux sont complétées." />
          </Card>
        </div>

        <div className="grid grid-cols-1 gap-5 lg:grid-cols-[1.1fr_1fr]">
          <Card title="Top jeux" note={periodNote}>
            {data.topGames.length === 0 ? (
              <p className="py-6 text-center text-sm text-muted">Aucune partie sur cette période.</p>
            ) : (
              <ol className="flex flex-col gap-2">
                {data.topGames.map((g, i) => (
                  <li key={g.gameId}>
                    <button onClick={() => onOpenGame(g.gameId)} className="flex w-full items-center gap-3 rounded-md p-1 text-left hover:bg-panel2">
                      <span className="w-5 text-right text-sm tabular-nums text-muted">{i + 1}</span>
                      <span className="h-12 w-8 shrink-0 overflow-hidden rounded">
                        <Cover title={g.title} path={g.coverPath} mini />
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="block truncate text-sm font-medium">{g.title}</span>
                        <ProgressBar value={percent(g.minutes, data.topGames[0].minutes)} className="mt-1.5" />
                      </span>
                      <span className="text-sm tabular-nums">{formatMinutes(g.minutes)}</span>
                    </button>
                  </li>
                ))}
              </ol>
            )}
          </Card>
          <div className="flex flex-col gap-5">
            <Card title="Habitudes" note="À quels moments de la semaine vous jouez.">
              <HabitsHeatmap habits={data.habits} />
            </Card>
          <Card title="Records" note={period === "all" ? "Depuis le début du suivi." : periodNote}>
            <div className="grid grid-cols-2 gap-3">
              {[
                { icon: <Clock3 size={14} />, rec: r.longestSession, value: r.longestSession ? formatSeconds(r.longestSession.value) : "–" },
                { icon: <CalendarDays size={14} />, rec: r.biggestDay, value: r.biggestDay ? formatSeconds(r.biggestDay.value) : "–" },
                { icon: <Sparkles size={14} />, rec: r.bestWeek, value: r.bestWeek ? formatSeconds(r.bestWeek.value) : "–" },
                {
                  icon: <Moon size={14} />,
                  rec: r.latestHour,
                  value: r.latestHour ? `${Math.floor(r.latestHour.value / 60)} h ${String(r.latestHour.value % 60).padStart(2, "0")}` : "–",
                },
              ].map(({ icon, rec, value }, i) => (
                <div key={i} className="rounded-lg border border-line bg-panel2/60 p-3">
                  <p className="flex items-center gap-1.5 text-xs text-muted">
                    {icon}
                    {rec?.label ?? ["Plus longue session", "Plus grosse journée", "Jeu le plus joué en une semaine", "Heure de jeu la plus tardive"][i]}
                  </p>
                  <p className="font-display mt-1 text-xl font-semibold tabular-nums">{value}</p>
                  <p className="truncate text-xs text-muted">{[rec?.gameTitle, rec?.date ? formatDate(rec.date) : null].filter(Boolean).join(" · ") || " "}</p>
                </div>
              ))}
            </div>
          </Card>
          </div>
        </div>


        <div className="grid grid-cols-1 gap-5 lg:grid-cols-2">
          <Card title="Succès" note={`${data.achievements.unlocked} débloqués sur ${data.achievements.total} · ${data.achievements.perfectGames} jeu(x) à 100 %`}>
            <div className="mb-4 flex items-center gap-3">
              <span className="font-display text-3xl font-semibold tabular-nums">{Math.round(data.achievements.completion)} %</span>
              <span className="text-xs text-muted">de complétion moyenne
                <br />
                des jeux qui ont des succès</span>
            </div>
            {data.achievements.rarest.length > 0 && (
              <>
                <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted">Les plus rares obtenus</p>
                <ul className="mb-4 flex flex-col gap-1.5">
                  {data.achievements.rarest.map((a) => (
                    <li key={`${a.gameId}-${a.name}`} className="flex items-center gap-2 text-sm">
                      <Award size={14} className="shrink-0 text-accent" />
                      <span className="min-w-0 flex-1 truncate">
                        {a.name} <span className="text-muted">· {a.gameTitle}</span>
                      </span>
                      <span className="tabular-nums text-xs text-muted">{a.percent.toFixed(1)} %</span>
                    </li>
                  ))}
                </ul>
              </>
            )}
            {data.achievements.almostDone.length > 0 && (
              <>
                <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted">Presque à 100 %</p>
                <ul className="flex flex-col gap-2">
                  {data.achievements.almostDone.map((g) => (
                    <li key={g.gameId}>
                      <button onClick={() => onOpenGame(g.gameId)} className="flex w-full items-center gap-2 rounded-md p-1 text-left text-sm hover:bg-panel2">
                        <span className="h-9 w-6 shrink-0 overflow-hidden rounded-sm">
                          <Cover title={g.title} path={g.coverPath} mini />
                        </span>
                        <span className="min-w-0 flex-1">
                          <span className="block truncate">{g.title}</span>
                          <ProgressBar value={percent(g.unlocked, g.total)} className="mt-1" />
                        </span>
                        <span className="text-xs tabular-nums text-muted">
                          {g.unlocked}/{g.total}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              </>
            )}
            {data.achievements.total === 0 && <p className="py-4 text-center text-sm text-muted">Aucun succès synchronisé.</p>}
          </Card>
          <Card title="Badges" note="Méta-succès de GameLauncher : une notification s'affiche à l'obtention.">
            <ul className="grid grid-cols-2 gap-2.5 sm:grid-cols-3">
              {data.badges.map((b) => (
                <li
                  key={b.id}
                  className={`flex flex-col items-center gap-1 rounded-lg border p-3 text-center ${b.earned ? "border-accent/60 bg-accent/10" : "border-line bg-panel2/40 opacity-60 grayscale"}`}
                  title={b.description}
                >
                  <span className="text-2xl" aria-hidden>
                    {b.icon}
                  </span>
                  <span className="text-sm font-semibold">{b.name}</span>
                  <span className="text-[11px] leading-tight text-muted">{b.description}</span>
                  <span className={`text-[10px] font-semibold uppercase tracking-wide ${b.earned ? "text-accent" : "text-muted"}`}>{b.earned ? "Obtenu" : "À débloquer"}</span>
                </li>
              ))}
            </ul>
          </Card>
        </div>
      </div>
    </div>
  );
}
