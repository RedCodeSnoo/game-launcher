// Accueil (O-01) : bannière « Continuer à jouer », jeux récents, série, succès récents, nouveautés, backlog.
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { ArrowRight, Award, Dices, Play, Sparkles } from "lucide-react";
import { api, imageUrl, on } from "../api";
import { t } from "../i18n";
import { formatMinutes, formatRelative, formatSeconds, hueOf } from "../lib/format";
import { statusInfo } from "../lib/organize";
import type { Game, Stats } from "../types";
import { Cover, placeholderGradient, SourceBadge } from "./Cover";
import { recapYear } from "./Recap";
import { StreakWidget } from "./StreakWidget";

interface Props {
  games: Game[];
  running: Set<string>;
  onPlay: (g: Game) => void;
  onOpen: (id: string) => void;
  onStats: () => void;
  onBacklog: () => void;
  onRoulette: () => void;
  onRecap: (year: number) => void;
  /** Encarts ajoutés par les intégrations (jeux gratuits, actualités). */
  extra?: ReactNode;
}

function Section({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return (
    <section className="flex min-w-0 flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-base font-semibold">{title}</h2>
        {action}
      </div>
      {children}
    </section>
  );
}

function CoverRow({ games, onOpen, onPlay, running }: { games: Game[]; onOpen: (id: string) => void; onPlay: (g: Game) => void; running: Set<string> }) {
  if (games.length === 0) return <p className="text-sm text-muted">Rien pour l'instant.</p>;
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(120px,1fr))] gap-3">
      {games.map((g) => (
        <div key={g.id} className="group relative min-w-0">
          <button onClick={() => onOpen(g.id)} className="game-card block aspect-[2/3] w-full overflow-hidden rounded-lg bg-panel2 ring-1 ring-line" title={g.title}>
            <Cover title={g.title} path={g.coverPath} />
            {running.has(g.id) && <span className="absolute bottom-1.5 left-1.5 rounded bg-scrim/80 px-1.5 py-0.5 text-[10px] font-bold uppercase text-ok">{t("game.running")}</span>}
          </button>
          {g.installed && g.available && !running.has(g.id) && (
            <button
              onClick={() => onPlay(g)}
              aria-label={`Jouer à ${g.title}`}
              className="absolute bottom-[3.2rem] right-2 hidden h-9 w-9 items-center justify-center rounded-full bg-accent text-accent-fg shadow-lg group-hover:flex"
            >
              <Play size={16} className="ml-0.5 fill-current" />
            </button>
          )}
          <p className="mt-1.5 truncate text-[13px] font-medium">{g.title}</p>
          <p className="truncate text-[11px] text-muted">{g.lastPlayed ? formatRelative(g.lastPlayed) : g.playtimeMinutes > 0 ? formatMinutes(g.playtimeMinutes) : "Jamais lancé"}</p>
        </div>
      ))}
    </div>
  );
}

export function Home({ games, running, onPlay, onOpen, onStats, onBacklog, onRoulette, onRecap, extra }: Props) {
  const recap = recapYear();
  const [stats, setStats] = useState<Stats | null>(null);
  useEffect(() => {
    const load = () => void api.getStats().then(setStats).catch(() => {});
    load();
    return on<null>("library-updated", load);
  }, []);

  const visible = useMemo(() => games.filter((g) => !g.hidden), [games]);
  const recent = useMemo(() => visible.filter((g) => g.lastPlayed).sort((a, b) => (b.lastPlayed ?? "").localeCompare(a.lastPlayed ?? "")), [visible]);
  const hero = recent.find((g) => g.installed) ?? recent[0] ?? visible.find((g) => g.installed) ?? null;
  const added = useMemo(() => [...visible].sort((a, b) => b.addedAt.localeCompare(a.addedAt)).slice(0, 8), [visible]);
  const backlog = useMemo(() => visible.filter((g) => g.status === "backlog" || g.status === "playing").slice(0, 8), [visible]);
  const heroImage = imageUrl(hero?.heroPath);
  const heroLogo = imageUrl(hero?.logoPath);
  const status = statusInfo(hero?.status);
  const isRunning = hero ? running.has(hero.id) : false;

  return (
    <div className="h-full overflow-y-auto">
      {hero && (
        <div className="relative h-[22rem] w-full overflow-hidden">
          <div
            className="absolute inset-0 bg-cover bg-center"
            style={{ backgroundImage: heroImage ? `url("${heroImage}")` : placeholderGradient(hueOf(hero.title), 120) }}
          />
          <div className="absolute inset-0 bg-gradient-to-r from-scrim/85 via-scrim/40 to-transparent" />
          <div className="absolute inset-0 bg-gradient-to-t from-bg via-transparent to-transparent" />
          <div className="relative flex h-full flex-col justify-end gap-4 px-8 pb-9">
            <p className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.2em] text-on-scrim/80">
              <Sparkles size={14} /> Continuer à jouer
            </p>
            {heroLogo ? (
              <img src={heroLogo} alt={hero.title} className="max-h-24 max-w-md object-contain object-left drop-shadow-xl" draggable={false} />
            ) : (
              <h1 className="max-w-2xl text-4xl font-bold leading-tight text-on-scrim drop-shadow-lg">{hero.title}</h1>
            )}
            <div className="flex flex-wrap items-center gap-2 text-sm text-on-scrim/85">
              {hero.sources.slice(0, 1).map((s) => (
                <SourceBadge key={s.id} source={s.source} />
              ))}
              {hero.playtimeMinutes > 0 && <span>{formatMinutes(hero.playtimeMinutes)} de jeu</span>}
              {hero.lastSessionSeconds !== null && <span>· dernière session {formatSeconds(hero.lastSessionSeconds)}</span>}
              {hero.lastPlayed && <span>· {formatRelative(hero.lastPlayed).toLowerCase()}</span>}
              {status && <span className="rounded-full bg-scrim/60 px-2 py-0.5 text-xs">{status.emoji} {status.label}</span>}
            </div>
            <div className="flex flex-wrap items-center gap-3">
              {isRunning ? (
                <span className="inline-flex h-14 items-center gap-3 rounded-xl bg-ok/25 px-8 text-lg font-semibold text-on-scrim">
                  <span className="h-3 w-3 animate-pulse rounded-full bg-ok" /> {t("game.running")}
                </span>
              ) : (
                <button
                  onClick={() => onPlay(hero)}
                  disabled={!hero.available}
                  className="inline-flex h-14 items-center gap-3 rounded-xl bg-accent px-10 text-lg font-semibold text-accent-fg shadow-2xl transition hover:scale-[1.03] hover:brightness-110 disabled:opacity-50"
                  autoFocus
                >
                  <Play size={22} className="fill-current" /> {hero.installed ? t("game.play") : t("game.install")}
                </button>
              )}
              <button onClick={() => onOpen(hero.id)} className="inline-flex h-14 items-center gap-2 rounded-xl bg-scrim/50 px-5 text-sm font-medium text-on-scrim backdrop-blur hover:bg-scrim/70">
                Voir la fiche <ArrowRight size={16} />
              </button>
              <button onClick={onRoulette} className="inline-flex h-14 items-center gap-2 rounded-xl bg-scrim/50 px-5 text-sm font-medium text-on-scrim backdrop-blur hover:bg-scrim/70" title="Je sais pas à quoi jouer">
                <Dices size={18} /> Je sais pas à quoi jouer
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="flex flex-col gap-8 px-8 py-6">
        {recap !== null && (
          <button onClick={() => onRecap(recap)} className="group relative overflow-hidden rounded-2xl border border-accent/50 bg-panel p-6 text-left">
            <span className="pointer-events-none absolute -right-10 -top-16 h-56 w-56 rounded-full bg-accent/25 blur-3xl transition group-hover:bg-accent/40" />
            <span className="relative flex items-center gap-5">
              <Sparkles size={36} className="text-accent" />
              <span>
                <span className="font-display block text-2xl font-bold">Votre récap {recap} est prêt</span>
                <span className="text-sm text-muted">Temps de jeu, top 5, meilleure série, jour record et votre profil de joueur, à partager en image.</span>
              </span>
              <ArrowRight size={22} className="ml-auto text-accent transition group-hover:translate-x-1" />
            </span>
          </button>
        )}
        <div className="grid grid-cols-1 gap-8 xl:grid-cols-[minmax(0,1fr)_20rem]">
          <Section title="Jeux récents">
            <CoverRow games={recent.slice(hero ? 1 : 0, (hero ? 1 : 0) + 8)} onOpen={onOpen} onPlay={onPlay} running={running} />
          </Section>
          <div className="flex flex-col gap-6">
            <Section title="Série" action={<button onClick={onStats} className="text-xs text-accent hover:underline">Statistiques</button>}>
              <StreakWidget onOpen={onStats} large />
            </Section>
            <Section title="Succès récents">
              {stats && stats.recentAchievements.length > 0 ? (
                <ul className="flex flex-col gap-2">
                  {stats.recentAchievements.slice(0, 5).map((a, i) => (
                    <li key={i}>
                      <button onClick={() => onOpen(a.gameId)} className="flex w-full items-center gap-2.5 rounded-md p-1 text-left hover:bg-panel2">
                        {a.icon ? <img src={a.icon} alt="" className="h-9 w-9 rounded" /> : <span className="flex h-9 w-9 items-center justify-center rounded bg-panel2 text-accent"><Award size={16} /></span>}
                        <span className="min-w-0">
                          <span className="block truncate text-sm">{a.name}</span>
                          <span className="block truncate text-xs text-muted">
                            {a.gameTitle} · {formatRelative(a.unlockedAt).toLowerCase()}
                          </span>
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-sm text-muted">Les succès Steam apparaissent ici une fois le compte relié.</p>
              )}
            </Section>
          </div>
        </div>
        {extra}
        <Section title="À jouer ensuite" action={<button onClick={onBacklog} className="text-xs text-accent hover:underline">Ouvrir le backlog</button>}>
          {backlog.length > 0 ? (
            <CoverRow games={backlog} onOpen={onOpen} onPlay={onPlay} running={running} />
          ) : (
            <p className="text-sm text-muted">Donnez le statut « À jouer » à des jeux depuis leur fiche ou le backlog.</p>
          )}
        </Section>
        <Section title="Nouveautés de la bibliothèque">
          <CoverRow games={added} onOpen={onOpen} onPlay={onPlay} running={running} />
        </Section>
      </div>
    </div>
  );
}
