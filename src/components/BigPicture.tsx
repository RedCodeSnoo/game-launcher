// Mode Big Picture (B-01, B-02) : plein écran pour la télé, navigable à la manette (Gamepad API), au clavier ou
// à la souris. Grandes jaquettes, fond animé du thème, sons de navigation.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Gamepad2, Heart, Play, X } from "lucide-react";
import { api, imageUrl, setFullscreen } from "../api";
import { t } from "../i18n";
import { formatMinutes, formatRelative, hueOf } from "../lib/format";
import { moveFocus, newMemory, readPad, type PadAction, type PadState } from "../lib/gamepad";
import { statusInfo } from "../lib/organize";
import { playUiSound } from "../themes/sounds";
import { useTheme } from "../themes/ThemeContext";
import type { Game } from "../types";
import { Cover, placeholderGradient, SourceBadge } from "./Cover";

interface Props {
  games: Game[];
  running: Set<string>;
  onPlay: (g: Game) => void;
  onExit: () => void;
}

interface Row {
  id: string;
  title: string;
  games: Game[];
}

function Clock() {
  const [now, setNow] = useState(new Date());
  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), 15_000);
    return () => clearInterval(t);
  }, []);
  return <span className="tabular-nums">{now.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}</span>;
}

function Hint({ btn, label }: { btn: string; label: string }) {
  return (
    <span className="flex items-center gap-2">
      <span className="flex h-7 min-w-7 items-center justify-center rounded-full border-2 border-on-scrim/60 px-1.5 text-xs font-bold">{btn}</span>
      {label}
    </span>
  );
}

export function BigPicture({ games, running, onPlay, onExit }: Props) {
  const { active } = useTheme();
  const [sounds, setSounds] = useState(true);
  useEffect(() => void api.getSettings().then((s) => setSounds(s.bigpicture_sounds !== "0")).catch(() => {}), []);
  const [pos, setPos] = useState({ row: 0, col: 0 });
  const [details, setDetails] = useState<Game | null>(null);
  const [detailBtn, setDetailBtn] = useState(0);
  const [pad, setPad] = useState<string | null>(null);
  const shelves = useRef<(HTMLDivElement | null)[]>([]);
  const soundSet = active.sounds === "none" ? "soft" : active.sounds;
  const playing = running.size > 0;

  const rows = useMemo<Row[]>(() => {
    const visible = games.filter((g) => !g.hidden);
    const recent = visible.filter((g) => g.lastPlayed && g.installed).sort((a, b) => (b.lastPlayed ?? "").localeCompare(a.lastPlayed ?? "")).slice(0, 16);
    const fav = visible.filter((g) => g.favorite);
    const backlog = visible.filter((g) => g.status === "backlog" || g.status === "playing");
    const installed = visible.filter((g) => g.installed).sort((a, b) => a.sortTitle.localeCompare(b.sortTitle));
    return [
      { id: "recent", title: "Continuer à jouer", games: recent },
      { id: "fav", title: "Favoris", games: fav },
      { id: "backlog", title: "À jouer", games: backlog },
      { id: "installed", title: "Tous les jeux installés", games: installed },
    ].filter((r) => r.games.length > 0);
  }, [games]);
  const lengths = rows.map((r) => r.games.length);
  const focused = rows[pos.row]?.games[pos.col] ?? null;

  // Plein écran à l'entrée, fenêtre normale à la sortie.
  useEffect(() => {
    void setFullscreen(true).catch(() => {});
    return () => void setFullscreen(false).catch(() => {});
  }, []);

  // La rangée et la jaquette sélectionnées restent visibles.
  useEffect(() => {
    const shelf = shelves.current[pos.row];
    shelf?.scrollIntoView({ block: "center", behavior: "smooth" });
    shelf?.querySelector<HTMLElement>(`[data-col="${pos.col}"]`)?.scrollIntoView({ inline: "center", block: "nearest", behavior: "smooth" });
  }, [pos]);

  const sound = useCallback((kind: "nav" | "click") => sounds && playUiSound(soundSet, kind), [sounds, soundSet]);

  const act = useCallback(
    (a: PadAction) => {
      if (a === "toggle") return onExit();
      if (details) {
        if (a === "left" || a === "right") {
          setDetailBtn((b) => (b === 0 ? 1 : 0));
          sound("nav");
        } else if (a === "select") {
          sound("click");
          if (detailBtn === 0) {
            if (details.installed && details.available) onPlay(details);
            setDetails(null);
          } else {
            void api.updateGame(details.id, { favorite: !details.favorite });
          }
        } else if (a === "back" || a === "details") {
          sound("nav");
          setDetails(null);
        }
        return;
      }
      if (a === "back" || a === "menu") return onExit();
      if (a === "select" && focused) {
        sound("click");
        if (focused.installed && focused.available && !running.has(focused.id)) onPlay(focused);
        else setDetails(focused);
        return;
      }
      if (a === "details" && focused) {
        sound("click");
        setDetailBtn(0);
        setDetails(focused);
        return;
      }
      const next = moveFocus(lengths, pos, a);
      if (next.row !== pos.row || next.col !== pos.col) {
        sound("nav");
        setPos(next);
      }
    },
    [details, detailBtn, focused, lengths, pos, onExit, onPlay, running, sound],
  );

  // Clavier.
  useEffect(() => {
    const map: Record<string, PadAction> = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right", Enter: "select", Escape: "back", Backspace: "back", y: "details", Y: "details" };
    const onKey = (e: KeyboardEvent) => {
      const a = map[e.key];
      if (!a) return;
      e.preventDefault();
      e.stopPropagation();
      act(a);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [act]);

  // Manette : lecture à chaque image, seulement fenêtre visible et aucun jeu en cours (la manette sert au jeu).
  const actRef = useRef(act);
  actRef.current = act;
  useEffect(() => {
    if (playing) return;
    const mem = newMemory();
    let raf = 0;
    const loop = () => {
      raf = requestAnimationFrame(loop);
      if (document.hidden) return;
      const gp = [...(navigator.getGamepads?.() ?? [])].find((g) => g && g.connected);
      if (!gp) return setPad((p) => (p ? null : p));
      setPad((p) => (p === gp.id ? p : gp.id));
      const state: PadState = { buttons: gp.buttons.map((b) => b.pressed), axes: [...gp.axes] };
      for (const a of readPad(mem, state, performance.now())) actRef.current(a);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [playing]);

  const hero = focused ? (imageUrl(focused.heroPath) ?? imageUrl(focused.coverPath)) : undefined;
  const runningGame = games.find((g) => running.has(g.id));
  const status = statusInfo(focused?.status);

  return (
    <div className="bigpicture fixed inset-0 z-[70] flex flex-col overflow-hidden text-on-scrim" role="application" aria-label="Mode Big Picture">
      {/* Illustration du jeu sélectionné, en fondu, sur le fond animé du thème. */}
      <div className="absolute inset-0 -z-10 bg-bg/40" />
      {focused && (
        <div
          key={focused.id}
          className="bp-hero absolute inset-0 -z-10 bg-cover bg-center"
          style={{ backgroundImage: hero ? `url("${hero}")` : placeholderGradient(hueOf(focused.title), 120) }}
        />
      )}
      <div className="absolute inset-0 -z-10 bg-gradient-to-r from-scrim/90 via-scrim/55 to-scrim/10" />
      <div className="absolute inset-0 -z-10 bg-gradient-to-t from-scrim via-scrim/40 to-transparent" />

      <header className="flex items-center gap-4 px-14 pt-8 text-lg">
        <Gamepad2 size={30} className="text-accent" />
        <span className="font-display text-2xl font-bold">{t("app.name")}</span>
        <span className="ml-auto flex items-center gap-6 text-on-scrim/80">
          {runningGame && (
            <span className="flex items-center gap-2 text-ok">
              <span className="h-2.5 w-2.5 animate-pulse rounded-full bg-ok" /> {runningGame.title}
            </span>
          )}
          <span className="text-sm">{pad ? "🎮 Manette connectée" : "Branchez une manette, ou utilisez les flèches"}</span>
          <Clock />
          <button onClick={onExit} className="rounded-full p-2 hover:bg-on-scrim/10" aria-label="Quitter le mode Big Picture">
            <X size={26} />
          </button>
        </span>
      </header>

      {focused && (
        <section className="px-14 pt-10" aria-live="polite">
          <div className="flex items-center gap-3 text-base text-on-scrim/80">
            {focused.sources.slice(0, 1).map((s) => (
              <SourceBadge key={s.id} source={s.source} />
            ))}
            {status && <span className="rounded-full bg-scrim/60 px-3 py-0.5 text-sm">{status.emoji} {status.label}</span>}
            {running.has(focused.id) && <span className="font-semibold uppercase text-ok">{t("game.running")}</span>}
          </div>
          <h1 className="font-display mt-3 max-w-5xl text-6xl font-bold leading-tight drop-shadow-2xl">{focused.title}</h1>
          <p className="mt-3 text-xl text-on-scrim/80">
            {[
              focused.playtimeMinutes > 0 ? `${formatMinutes(focused.playtimeMinutes)} de jeu` : "Jamais lancé",
              focused.lastPlayed ? `joué ${formatRelative(focused.lastPlayed).toLowerCase()}` : null,
              focused.achTotal > 0 ? `${focused.achUnlocked}/${focused.achTotal} succès` : null,
              focused.timeToBeatMain ? `histoire ≈ ${formatMinutes(focused.timeToBeatMain)}` : null,
            ]
              .filter(Boolean)
              .join("  ·  ")}
          </p>
        </section>
      )}

      <main className="mt-auto flex min-h-0 flex-col gap-7 overflow-y-hidden pb-6 pt-8">
        {rows.length === 0 && <p className="px-14 text-2xl text-on-scrim/80">Aucun jeu installé pour l'instant.</p>}
        {rows.map((r, ri) => (
          <div key={r.id} ref={(el) => void (shelves.current[ri] = el)} className={`transition-opacity duration-300 ${ri < pos.row || ri > pos.row + 1 ? "hidden" : ri === pos.row ? "opacity-100" : "opacity-55"}`}>
            <h2 className="px-14 pb-3 text-xl font-semibold">{r.title}</h2>
            <div className="flex gap-6 overflow-x-hidden px-14 py-5">
              {r.games.map((g, ci) => {
                const sel = ri === pos.row && ci === pos.col;
                return (
                  <button
                    key={g.id}
                    data-col={ci}
                    onMouseEnter={() => setPos({ row: ri, col: ci })}
                    onClick={() => (sel ? act("select") : setPos({ row: ri, col: ci }))}
                    className={`bp-card relative aspect-[2/3] shrink-0 overflow-hidden rounded-2xl transition-all duration-200 ${sel ? "bp-selected w-56" : "w-44"} ${!g.available && g.installed ? "grayscale" : ""}`}
                    aria-label={g.title}
                    aria-current={sel}
                  >
                    <Cover title={g.title} path={g.coverPath} />
                    {running.has(g.id) && <span className="absolute bottom-2 left-2 rounded bg-scrim/80 px-2 py-0.5 text-xs font-bold uppercase text-ok">{t("game.running")}</span>}
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </main>

      <footer className="flex items-center gap-8 bg-scrim/60 px-14 py-4 text-base">
        <Hint btn="A" label={focused && focused.installed && focused.available ? "Jouer" : "Ouvrir"} />
        <Hint btn="Y" label="Détails" />
        <Hint btn="B" label="Quitter" />
        <span className="ml-auto text-sm text-on-scrim/70">F11, ou Vue + Menu : entrer / sortir du mode Big Picture</span>
      </footer>

      {details && (
        <div className="absolute inset-0 z-10 flex items-center justify-center bg-scrim/75 backdrop-blur-sm" role="dialog" aria-label={details.title}>
          <div className="flex w-[min(64rem,90vw)] gap-10 rounded-3xl border border-on-scrim/15 bg-panel/95 p-10 text-fg shadow-2xl">
            <div className="w-64 shrink-0 overflow-hidden rounded-2xl">
              <Cover title={details.title} path={details.coverPath} />
            </div>
            <div className="flex min-w-0 flex-col gap-4">
              <h2 className="font-display text-4xl font-bold">{details.title}</h2>
              <p className="text-lg text-muted">
                {[details.developer, details.genres.slice(0, 3).join(", "), details.playtimeMinutes > 0 ? `${formatMinutes(details.playtimeMinutes)} de jeu` : null].filter(Boolean).join(" · ")}
              </p>
              {details.description && <p className="line-clamp-5 text-base leading-relaxed text-fg/85">{details.description}</p>}
              <div className="mt-auto flex gap-4">
                {[
                  { label: details.installed ? (details.available ? t("game.play") : t("game.unavailableLong")) : t("game.install"), icon: <Play size={22} className="fill-current" /> },
                  { label: details.favorite ? "Retirer des favoris" : "Ajouter aux favoris", icon: <Heart size={22} className={details.favorite ? "fill-current" : ""} /> },
                ].map((b, i) => (
                  <button
                    key={i}
                    onMouseEnter={() => setDetailBtn(i)}
                    onClick={() => {
                      setDetailBtn(i);
                      act("select");
                    }}
                    className={`flex h-16 items-center gap-3 rounded-2xl px-8 text-xl font-semibold transition ${detailBtn === i ? "scale-105 bg-accent text-accent-fg shadow-xl" : "bg-panel2 text-fg"}`}
                  >
                    {b.icon} {b.label}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
