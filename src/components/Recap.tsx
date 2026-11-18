// « Récap de l'année » (B-03, B-04) : une suite d'écrans animés, puis l'image à partager en PNG.
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { ChevronLeft, ChevronRight, Download, X } from "lucide-react";
import { api, pickSavePath } from "../api";
import { errorMessage, SOURCE_COLORS, SOURCE_LABELS } from "../i18n";
import { formatDate, formatSeconds } from "../lib/format";
import { renderCard } from "../lib/sharecard";
import { useTheme } from "../themes/ThemeContext";
import type { Recap as RecapData } from "../types";
import { Cover } from "./Cover";
import { useToasts } from "./Toasts";

const SLIDE_MS = 7000;
const MONTHS = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];

/** Nombre qui monte de 0 à sa valeur (sans animation si les animations sont réduites). */
function CountUp({ value, format }: { value: number; format: (n: number) => string }) {
  const { reduced } = useTheme();
  const [shown, setShown] = useState(reduced ? value : 0);
  useEffect(() => {
    if (reduced) return setShown(value);
    let raf = 0;
    const start = performance.now();
    const tick = (t: number) => {
      const k = Math.min(1, (t - start) / 1400);
      setShown(value * (1 - Math.pow(1 - k, 3)));
      if (k < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [value, reduced]);
  return <>{format(shown)}</>;
}

function hours(s: number): string {
  return `${Math.round(s / 3600).toLocaleString("fr-FR")} h`;
}

function Slide({ kicker, title, children }: { kicker: string; title?: ReactNode; children?: ReactNode }) {
  return (
    <div className="recap-slide flex h-full flex-col items-center justify-center gap-6 px-10 text-center">
      <p className="text-sm font-semibold uppercase tracking-[0.3em] text-accent">{kicker}</p>
      {title && <h2 className="font-display max-w-4xl text-5xl font-bold leading-tight">{title}</h2>}
      {children}
    </div>
  );
}

export function RecapView({ year, onClose }: { year: number; onClose: () => void }) {
  const toasts = useToasts();
  const [data, setData] = useState<RecapData | null>(null);
  const [index, setIndex] = useState(0);
  const [paused, setPaused] = useState(false);
  const [card, setCard] = useState<string | null>(null);
  useEffect(() => void api.getRecap(year).then(setData).catch((e) => toasts.error(errorMessage(e))), [year, toasts]);

  const slides = useMemo<{ id: string; node: ReactNode }[]>(() => {
    if (!data) return [];
    const top = data.topGames;
    const maxMonth = Math.max(1, ...data.monthly);
    const fav = data.favoriteSource;
    return [
      {
        id: "intro",
        node: (
          <Slide kicker={`Récap ${data.year}`} title={<>Votre année de jeu</>}>
            <p className="max-w-xl text-lg text-muted">Ce que GameLauncher a retenu de vos parties en {data.year}, des marathons du week-end aux parties de minuit.</p>
          </Slide>
        ),
      },
      {
        id: "time",
        node: (
          <Slide kicker="Temps de jeu">
            <p className="font-display text-[7rem] font-extrabold leading-none text-accent">
              <CountUp value={data.totalSeconds} format={(n) => hours(n)} />
            </p>
            <p className="text-xl">
              sur <span className="font-semibold">{data.daysPlayed}</span> jours joués, en <span className="font-semibold">{data.sessions}</span> sessions et <span className="font-semibold">{data.gamesPlayed}</span> jeux
            </p>
            <div className="mt-4 flex h-32 items-end gap-2">
              {data.monthly.map((m, i) => (
                <div key={i} className="flex flex-col items-center gap-1">
                  <div className="recap-bar w-9 rounded-t-md bg-accent" style={{ height: `${Math.max(3, (m / maxMonth) * 110)}px`, animationDelay: `${i * 60}ms` }} title={`${MONTHS[i]} : ${formatSeconds(m)}`} />
                  <span className="text-[11px] text-muted">{MONTHS[i]}</span>
                </div>
              ))}
            </div>
          </Slide>
        ),
      },
      {
        id: "top",
        node: (
          <Slide kicker="Vos jeux de l'année" title={top[0] ? <>« {top[0].title} » en tête</> : "Pas encore de partie cette année"}>
            <div className="flex items-end justify-center gap-5">
              {top.map((g, i) => (
                <div key={g.gameId} className={`recap-pop flex flex-col items-center gap-2 ${i === 0 ? "w-44" : "w-36"}`} style={{ animationDelay: `${i * 140}ms` }}>
                  <span className="font-display text-2xl font-bold text-accent">#{i + 1}</span>
                  <div className={`w-full overflow-hidden rounded-xl shadow-2xl ring-2 ${i === 0 ? "aspect-[2/3] ring-accent" : "aspect-[2/3] ring-line"}`}>
                    <Cover title={g.title} path={g.coverPath} />
                  </div>
                  <span className="mt-1 line-clamp-2 text-sm font-semibold">{g.title}</span>
                  <span className="text-xs text-muted">{hours(g.seconds)}</span>
                </div>
              ))}
            </div>
          </Slide>
        ),
      },
      {
        id: "streak",
        node: (
          <Slide kicker="Meilleure série">
            <p className="font-display text-[7rem] font-extrabold leading-none">
              <CountUp value={data.bestStreak} format={(n) => String(Math.round(n))} /> <span className="text-5xl">🔥</span>
            </p>
            <p className="text-xl">jours joués d'affilée{data.bestStreakStart ? `, du ${formatDate(data.bestStreakStart)} au ${formatDate(data.bestStreakEnd)}` : ""}</p>
          </Slide>
        ),
      },
      {
        id: "record",
        node: (
          <Slide kicker="Jour record" title={data.recordDay ? new Date(`${data.recordDay[0]}T12:00:00`).toLocaleDateString("fr-FR", { weekday: "long", day: "numeric", month: "long" }) : "—"}>
            {data.recordDay && (
              <>
                <p className="font-display text-7xl font-extrabold text-accent">{formatSeconds(data.recordDay[1])}</p>
                <p className="text-lg text-muted">{data.recordDayGames.length ? `de ${data.recordDayGames.join(", ")}` : ""}</p>
              </>
            )}
            {data.longestSession && <p className="text-base text-muted">Plus longue session : {formatSeconds(data.longestSession[1])} sur {data.longestSession[0]}</p>}
          </Slide>
        ),
      },
      {
        id: "launcher",
        node: (
          <Slide kicker="Launcher préféré" title={fav ? SOURCE_LABELS[fav[0]] : "—"}>
            {fav && (
              <>
                <div className="relative h-56 w-56">
                  <svg viewBox="0 0 100 100" className="h-full w-full -rotate-90">
                    <circle cx="50" cy="50" r="42" fill="none" stroke="var(--panel2)" strokeWidth="12" />
                    <circle className="recap-ring" cx="50" cy="50" r="42" fill="none" stroke={SOURCE_COLORS[fav[0]]} strokeWidth="12" strokeLinecap="round" strokeDasharray={`${data.sourceShare * 264} 264`} />
                  </svg>
                  <span className="font-display absolute inset-0 flex items-center justify-center text-5xl font-bold">{Math.round(data.sourceShare * 100)} %</span>
                </div>
                <p className="text-lg text-muted">de votre temps de jeu, soit {hours(fav[1])}</p>
              </>
            )}
          </Slide>
        ),
      },
      {
        id: "profile",
        node: (
          <Slide kicker="Votre profil de joueur">
            <span className="recap-pop text-[8rem] leading-none">{data.profile.emoji}</span>
            <h2 className="font-display text-6xl font-bold">{data.profile.title}</h2>
            <p className="max-w-xl text-xl text-muted">{data.profile.description}</p>
            {data.profile.traits.length > 0 && (
              <p className="flex flex-wrap justify-center gap-2">
                {data.profile.traits.map((t) => (
                  <span key={t} className="rounded-full border border-accent/60 px-3 py-1 text-sm">
                    aussi {t}
                  </span>
                ))}
              </p>
            )}
            {data.badges.length > 0 && <p className="text-sm text-muted">Badges : {data.badges.map((b) => `${b.icon} ${b.name}`).join(" · ")}</p>}
          </Slide>
        ),
      },
      {
        id: "share",
        node: (
          <Slide kicker="À partager">
            {card ? <img src={card} alt={`Récap ${data.year}`} className="max-h-[52vh] rounded-xl shadow-2xl ring-1 ring-line" /> : <div className="skeleton h-[52vh] w-[42vh] rounded-xl" />}
            <button
              disabled={!card}
              onClick={async () => {
                if (!card) return;
                const path = await pickSavePath("Image du récap", `recap-${data.year}.png`, ["png"]);
                if (!path) return;
                try {
                  await api.savePng(path, card);
                  toasts.success("Image enregistrée");
                } catch (e) {
                  toasts.error(errorMessage(e));
                }
              }}
              className="inline-flex h-12 items-center gap-2 rounded-xl bg-accent px-6 font-semibold text-accent-fg disabled:opacity-50"
            >
              <Download size={18} /> Enregistrer l'image (PNG)
            </button>
          </Slide>
        ),
      },
    ];
  }, [data, card, toasts]);

  // Image à partager, préparée en fond.
  useEffect(() => {
    if (!data) return;
    void renderCard({
      kind: "recap",
      heading: `Mon année ${data.year}`,
      subheading: "Récap GameLauncher",
      bigValue: hours(data.totalSeconds),
      bigLabel: `de jeu sur ${data.daysPlayed} jours · ${data.gamesPlayed} jeux`,
      stats: [
        { label: "meilleure série", value: `${data.bestStreak} j 🔥` },
        { label: "jour record", value: data.recordDay ? formatSeconds(data.recordDay[1]) : "—" },
        { label: "launcher préféré", value: data.favoriteSource ? SOURCE_LABELS[data.favoriteSource[0]] : "—" },
      ],
      games: data.topGames.map((g) => ({ title: g.title, coverPath: g.coverPath, value: hours(g.seconds) })),
      profile: data.profile,
      badges: data.badges.map((b) => b.name),
      footer: "GameLauncher",
    }).then(setCard);
  }, [data]);

  // Défilement automatique, flèches, Échap.
  useEffect(() => {
    if (paused || slides.length === 0 || index >= slides.length - 1) return;
    const t = setTimeout(() => setIndex((i) => Math.min(slides.length - 1, i + 1)), SLIDE_MS);
    return () => clearTimeout(t);
  }, [index, paused, slides.length]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight" || e.key === " ") setIndex((i) => Math.min(slides.length - 1, i + 1));
      if (e.key === "ArrowLeft") setIndex((i) => Math.max(0, i - 1));
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [slides.length, onClose]);

  return (
    <div className="fixed inset-0 z-[80] flex flex-col bg-bg/95 backdrop-blur-xl" role="dialog" aria-label={`Récap ${year}`} onMouseDown={() => setPaused(true)} onMouseUp={() => setPaused(false)}>
      <div className="flex items-center gap-3 px-6 pt-5">
        <div className="flex flex-1 gap-1.5">
          {slides.map((s, i) => (
            <button key={s.id} onClick={() => setIndex(i)} className="h-1.5 flex-1 overflow-hidden rounded-full bg-line" aria-label={`Écran ${i + 1}`}>
              <span
                className={`block h-full bg-accent ${i === index && !paused ? "recap-progress" : ""}`}
                style={{ width: i < index ? "100%" : i === index ? undefined : "0%", animationDuration: `${SLIDE_MS}ms` }}
              />
            </button>
          ))}
        </div>
        <button onClick={onClose} className="rounded-md p-1.5 text-muted hover:text-fg" aria-label="Fermer le récap">
          <X size={22} />
        </button>
      </div>
      <div className="relative min-h-0 flex-1">
        {data ? <div key={slides[index]?.id} className="h-full">{slides[index]?.node}</div> : <div className="flex h-full items-center justify-center text-muted">Préparation de votre récap…</div>}
        <button onClick={() => setIndex((i) => Math.max(0, i - 1))} className="absolute left-4 top-1/2 -translate-y-1/2 rounded-full p-2 text-muted hover:bg-panel2 hover:text-fg" aria-label="Écran précédent">
          <ChevronLeft size={30} />
        </button>
        <button onClick={() => setIndex((i) => Math.min(slides.length - 1, i + 1))} className="absolute right-4 top-1/2 -translate-y-1/2 rounded-full p-2 text-muted hover:bg-panel2 hover:text-fg" aria-label="Écran suivant">
          <ChevronRight size={30} />
        </button>
      </div>
    </div>
  );
}

/** Année proposée pour le récap : l'année en cours à partir du 1er décembre, sinon l'année précédente en janvier. */
export function recapYear(now = new Date()): number | null {
  if (now.getMonth() === 11) return now.getFullYear();
  if (now.getMonth() === 0) return now.getFullYear() - 1;
  return null;
}
