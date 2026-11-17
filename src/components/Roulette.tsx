// « Je sais pas à quoi jouer » (O-06) : roulette animée parmi les jeux installés, avec filtres.
import { useMemo, useRef, useState } from "react";
import { Dices, Play, RotateCcw } from "lucide-react";
import { formatMinutes } from "../lib/format";
import { DEFAULT_ROULETTE, pick, rouletteCandidates, type RouletteOptions } from "../lib/organize";
import { useTheme } from "../themes/ThemeContext";
import type { Game } from "../types";
import { Cover } from "./Cover";
import { Button, Modal } from "./ui";

const CARD = 132;
const GAP = 12;
const REEL = 36;

export function Roulette({ games, onClose, onPlay, onOpen }: { games: Game[]; onClose: () => void; onPlay: (g: Game) => void; onOpen: (id: string) => void }) {
  const { reduced } = useTheme();
  const [opts, setOpts] = useState<RouletteOptions>(DEFAULT_ROULETTE);
  const [reel, setReel] = useState<Game[]>([]);
  const [offset, setOffset] = useState(0);
  const [spinning, setSpinning] = useState(false);
  const [result, setResult] = useState<Game | null>(null);
  const viewport = useRef<HTMLDivElement>(null);
  const candidates = useMemo(() => rouletteCandidates(games, opts), [games, opts]);
  const genres = useMemo(() => [...new Set(games.flatMap((g) => g.genres))].sort(), [games]);
  const hasDurations = games.some((g) => g.timeToBeatMain);

  const spin = () => {
    if (candidates.length === 0) return;
    const winner = pick(candidates, Math.random())!;
    // Bande de jaquettes : du hasard, puis le gagnant à la position REEL - 4.
    const strip = Array.from({ length: REEL }, (_, i) => (i === REEL - 4 ? winner : pick(candidates, Math.random())!));
    setResult(null);
    setReel(strip);
    setOffset(0);
    const width = viewport.current?.clientWidth ?? 600;
    const target = (REEL - 4) * (CARD + GAP) - (width - CARD) / 2;
    if (reduced) {
      setOffset(target);
      setResult(winner);
      return;
    }
    setSpinning(true);
    requestAnimationFrame(() => requestAnimationFrame(() => setOffset(target)));
    window.setTimeout(() => {
      setSpinning(false);
      setResult(winner);
    }, 3300);
  };

  const check = (key: keyof RouletteOptions, label: string, disabled = false, hint?: string) => (
    <label className={`flex items-center gap-2 text-sm ${disabled ? "opacity-50" : ""}`} title={hint}>
      <input type="checkbox" className="h-4 w-4 accent-[var(--accent)]" disabled={disabled} checked={Boolean(opts[key])} onChange={(e) => setOpts((o) => ({ ...o, [key]: e.target.checked }))} />
      {label}
    </label>
  );

  return (
    <Modal title="Je sais pas à quoi jouer 🎲" onClose={onClose} wide>
      <div className="flex flex-col gap-5">
        <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
          {check("backlogOnly", "Backlog seulement")}
          {check("shortOnly", "Durée courte (15 h max)", !hasDurations, hasDurations ? undefined : "Disponible quand la durée de vie des jeux est connue.")}
          {check("notPlayed30", "Pas joué depuis 30 jours")}
          <select className="h-8 rounded-md border border-line bg-bg px-2 text-sm" value={opts.genre ?? ""} onChange={(e) => setOpts((o) => ({ ...o, genre: e.target.value || null }))} aria-label="Genre">
            <option value="">Tous les genres</option>
            {genres.map((g) => (
              <option key={g} value={g}>
                {g}
              </option>
            ))}
          </select>
          <span className="ml-auto text-xs text-muted">{candidates.length} jeu(x) possible(s)</span>
        </div>

        <div ref={viewport} className="relative h-[13.5rem] overflow-hidden rounded-xl border border-line bg-panel2/60">
          <div className="pointer-events-none absolute inset-y-0 left-1/2 z-10 w-[3px] -translate-x-1/2 bg-accent shadow-[0_0_12px_var(--accent)]" />
          <div className="pointer-events-none absolute inset-y-0 left-0 z-10 w-24 bg-gradient-to-r from-panel to-transparent" />
          <div className="pointer-events-none absolute inset-y-0 right-0 z-10 w-24 bg-gradient-to-l from-panel to-transparent" />
          {reel.length === 0 ? (
            <div className="flex h-full flex-col items-center justify-center gap-2 text-sm text-muted">
              <Dices size={34} className="text-accent" />
              Lancez la roulette pour tirer un jeu au hasard.
            </div>
          ) : (
            <div
              className="flex h-full items-center"
              style={{
                gap: GAP,
                paddingLeft: GAP,
                transform: `translateX(${-offset}px)`,
                transition: spinning ? "transform 3.2s cubic-bezier(0.12, 0.8, 0.2, 1)" : "none",
              }}
            >
              {reel.map((g, i) => (
                <div
                  key={i}
                  className={`shrink-0 overflow-hidden rounded-lg ring-1 transition ${result && i === REEL - 4 ? "scale-105 ring-2 ring-accent" : "ring-line"}`}
                  style={{ width: CARD, height: CARD * 1.5 * 0.92 }}
                >
                  <Cover title={g.title} path={g.coverPath} />
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="flex min-h-14 flex-wrap items-center gap-3">
          {result ? (
            <>
              <div className="min-w-0 flex-1">
                <p className="text-xs uppercase tracking-wide text-muted">Ce soir, ce sera…</p>
                <p className="font-display truncate text-xl font-semibold">{result.title}</p>
                <p className="text-xs text-muted">
                  {[result.playtimeMinutes > 0 ? `${formatMinutes(result.playtimeMinutes)} de jeu` : "Jamais lancé", result.timeToBeatMain ? `histoire ≈ ${formatMinutes(result.timeToBeatMain)}` : null].filter(Boolean).join(" · ")}
                </p>
              </div>
              <Button onClick={() => onOpen(result.id)}>Voir la fiche</Button>
              <Button onClick={spin}>
                <RotateCcw size={15} /> Relancer
              </Button>
              <Button
                variant="primary"
                onClick={() => {
                  onPlay(result);
                  onClose();
                }}
              >
                <Play size={15} className="fill-current" /> Jouer
              </Button>
            </>
          ) : (
            <Button variant="primary" className="ml-auto !h-11 !px-6" onClick={spin} disabled={spinning || candidates.length === 0}>
              <Dices size={18} /> {spinning ? "Ça tourne…" : "Lancer la roulette"}
            </Button>
          )}
        </div>
      </div>
    </Modal>
  );
}
