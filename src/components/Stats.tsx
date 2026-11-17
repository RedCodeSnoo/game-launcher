// Page Statistiques (F-HEU-06) : temps total, par source, top 10, par semaine et par mois, succès récents.
import { useEffect, useState } from "react";
import { Trophy } from "lucide-react";
import { api } from "../api";
import { errorMessage, SOURCE_COLORS, SOURCE_LABELS } from "../i18n";
import { formatMinutes, formatRelative, formatSeconds, percent } from "../lib/format";
import type { Bucket, Stats as StatsData } from "../types";
import { Cover } from "./Cover";
import { useToasts } from "./Toasts";
import { Tabs } from "./ui";

function Tile({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className="rounded-xl border border-line bg-panel px-4 py-3">
      <p className="text-xs uppercase tracking-wide text-muted">{label}</p>
      <p className="mt-0.5 text-2xl font-semibold tabular-nums">{value}</p>
      {sub && <p className="text-xs text-muted">{sub}</p>}
    </div>
  );
}

function Card({ title, children, note }: { title: string; children: React.ReactNode; note?: string }) {
  return (
    <section className="rounded-xl border border-line bg-panel p-5">
      <h2 className="text-sm font-semibold">{title}</h2>
      {note && <p className="mt-0.5 text-xs text-muted">{note}</p>}
      <div className="mt-4">{children}</div>
    </section>
  );
}

function bucketLabel(b: Bucket, monthly: boolean): string {
  const d = new Date(`${b.start}T12:00:00`);
  return monthly ? d.toLocaleDateString("fr-FR", { month: "short" }) : d.toLocaleDateString("fr-FR", { day: "numeric", month: "numeric" });
}

/** Histogramme d'une seule série : barres d'une couleur, valeur au survol et en libellé accessible. */
function BarChart({ buckets, monthly }: { buckets: Bucket[]; monthly: boolean }) {
  const max = Math.max(...buckets.map((b) => b.seconds), 1);
  const total = buckets.reduce((n, b) => n + b.seconds, 0);
  if (total === 0) return <p className="py-10 text-center text-sm text-muted">Aucune session suivie sur cette période.</p>;
  return (
    <div>
      <div className="flex h-44 items-end gap-2" role="img" aria-label={`Temps de jeu suivi ${monthly ? "par mois" : "par semaine"}, total ${formatSeconds(total)}`}>
        {buckets.map((b) => (
          <div key={b.start} className="group flex h-full min-w-0 flex-1 flex-col items-center justify-end gap-1" title={`${monthly ? "" : "Semaine du "}${bucketLabel(b, monthly)} : ${formatSeconds(b.seconds)}`}>
            <span className="text-[10px] tabular-nums text-muted opacity-0 group-hover:opacity-100">{b.seconds > 0 ? formatSeconds(b.seconds) : ""}</span>
            <div className="w-full rounded-t bg-accent/80 transition group-hover:bg-accent" style={{ height: `${Math.max((b.seconds / max) * 100, b.seconds > 0 ? 2 : 0)}%` }} />
          </div>
        ))}
      </div>
      <div className="mt-1.5 flex gap-2 border-t border-line pt-1.5">
        {buckets.map((b) => (
          <span key={b.start} className="min-w-0 flex-1 truncate text-center text-[10px] text-muted">
            {bucketLabel(b, monthly)}
          </span>
        ))}
      </div>
    </div>
  );
}

export function Stats({ onOpenGame, refreshKey }: { onOpenGame: (id: string) => void; refreshKey: number }) {
  const toasts = useToasts();
  const [stats, setStats] = useState<StatsData | null>(null);
  const [period, setPeriod] = useState<"weekly" | "monthly">("weekly");

  useEffect(() => {
    api
      .getStats()
      .then(setStats)
      .catch((e) => toasts.error(errorMessage(e)));
  }, [refreshKey, toasts]);

  if (!stats) {
    return (
      <div className="grid grid-cols-4 gap-4 p-6">
        {Array.from({ length: 4 }, (_, i) => (
          <div key={i} className="skeleton h-20 rounded-xl" />
        ))}
      </div>
    );
  }

  const maxSource = Math.max(...stats.bySource.map((s) => s.minutes), 1);
  const maxTop = Math.max(...stats.topGames.map((g) => g.minutes), 1);
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex max-w-5xl flex-col gap-5 px-6 py-5">
        <h1 className="text-lg font-semibold">Statistiques</h1>
        <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
          <Tile label="Temps de jeu total" value={formatMinutes(stats.totalMinutes)} sub={`dont ${formatSeconds(stats.trackedSeconds)} suivis par l'application`} />
          <Tile label="Jeux" value={String(stats.gamesTotal)} sub={`${stats.gamesInstalled} installés`} />
          <Tile label="Jeux joués" value={String(stats.gamesPlayed)} sub={`${percent(stats.gamesPlayed, stats.gamesTotal)} % de la bibliothèque`} />
          <Tile
            label="Succès débloqués"
            value={String(stats.achievementsUnlocked)}
            sub={stats.achievementsTotal > 0 ? `sur ${stats.achievementsTotal} · ${percent(stats.achievementsUnlocked, stats.achievementsTotal)} %` : "Aucun succès"}
          />
        </div>

        <Card title="Temps de jeu suivi">
          <div className="mb-4">
            <Tabs
              tabs={[
                { id: "weekly", label: "Par semaine" },
                { id: "monthly", label: "Par mois" },
              ]}
              value={period}
              onChange={setPeriod}
            />
          </div>
          <BarChart buckets={period === "weekly" ? stats.weekly : stats.monthly} monthly={period === "monthly"} />
          <p className="mt-3 text-[11px] text-muted">
            Sessions enregistrées uniquement : les heures saisies à la main et celles importées d'un launcher n'ont pas de date, elles comptent dans les totaux mais pas dans ce graphique.
          </p>
        </Card>

        <div className="grid grid-cols-1 gap-5 lg:grid-cols-2">
          <Card title="Temps par source">
            {stats.bySource.length === 0 ? (
              <p className="py-6 text-center text-sm text-muted">Aucune donnée.</p>
            ) : (
              <ul className="flex flex-col gap-3">
                {stats.bySource.map((s) => (
                  <li key={s.source}>
                    <div className="mb-1 flex items-baseline justify-between gap-3 text-sm">
                      <span className="flex items-center gap-2">
                        <span className="h-2.5 w-2.5 rounded-full" style={{ background: SOURCE_COLORS[s.source] }} />
                        {SOURCE_LABELS[s.source]}
                        <span className="text-xs text-muted">{s.games} jeux</span>
                      </span>
                      <span className="tabular-nums">{formatMinutes(s.minutes)}</span>
                    </div>
                    <div className="h-2 overflow-hidden rounded-full bg-panel2">
                      <div className="h-full rounded-full" style={{ width: `${(s.minutes / maxSource) * 100}%`, background: SOURCE_COLORS[s.source] }} />
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </Card>

          <Card title="Top 10 des jeux">
            {stats.topGames.length === 0 ? (
              <p className="py-6 text-center text-sm text-muted">Aucun temps de jeu enregistré pour l'instant.</p>
            ) : (
              <ol className="flex flex-col gap-1">
                {stats.topGames.map((g, i) => (
                  <li key={g.gameId}>
                    <button onClick={() => onOpenGame(g.gameId)} className="flex w-full items-center gap-3 rounded-md px-1.5 py-1 text-left hover:bg-panel2">
                      <span className="w-5 text-right text-xs tabular-nums text-muted">{i + 1}</span>
                      <span className="h-9 w-6 shrink-0 overflow-hidden rounded bg-panel2 text-[0px]">
                        <Cover title={g.title} path={g.coverPath} />
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="block truncate text-sm">{g.title}</span>
                        <span className="mt-1 block h-1 overflow-hidden rounded-full bg-panel2">
                          <span className="block h-full rounded-full bg-accent" style={{ width: `${(g.minutes / maxTop) * 100}%` }} />
                        </span>
                      </span>
                      <span className="text-sm tabular-nums">{formatMinutes(g.minutes)}</span>
                    </button>
                  </li>
                ))}
              </ol>
            )}
          </Card>
        </div>

        <Card title="Succès débloqués récemment">
          {stats.recentAchievements.length === 0 ? (
            <p className="py-6 text-center text-sm text-muted">Aucun succès débloqué pour l'instant.</p>
          ) : (
            <ul className="grid grid-cols-1 gap-2 md:grid-cols-2">
              {stats.recentAchievements.map((a, i) => (
                <li key={`${a.gameId}-${a.name}-${i}`}>
                  <button onClick={() => onOpenGame(a.gameId)} className="flex w-full items-center gap-3 rounded-md border border-line bg-bg p-2 text-left hover:border-accent">
                    {a.icon ? (
                      <img src={a.icon} alt="" loading="lazy" className="h-10 w-10 rounded" />
                    ) : (
                      <span className="flex h-10 w-10 items-center justify-center rounded bg-accent/25 text-accent">
                        <Trophy size={18} />
                      </span>
                    )}
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-medium">{a.name}</span>
                      <span className="block truncate text-xs text-muted">{a.gameTitle}</span>
                    </span>
                    <span className="shrink-0 text-xs text-muted">{formatRelative(a.unlockedAt)}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>
    </div>
  );
}
