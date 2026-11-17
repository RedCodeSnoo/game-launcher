// Grille de jaquettes et vue liste, toutes deux virtualisées (fluide au-delà de 1 000 jeux, §6.1).
import { memo, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Download, HardDrive, Heart, Play, Square } from "lucide-react";
import { Cover, SourceBadge } from "./Cover";
import { t } from "../i18n";
import { formatMinutes, formatRelative, percent } from "../lib/format";
import type { Game } from "../types";

interface Props {
  games: Game[];
  running: Set<string>;
  onOpen: (game: Game) => void;
  onPlay: (game: Game) => void;
  /** « Arrêter le suivi » d'un jeu en cours (E1-01). */
  onStop: (game: Game) => void;
}

function useWidth<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => setWidth(entry.contentRect.width));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return [ref, width] as const;
}

/** Libellé d'état d'un jeu pour les lecteurs d'écran et les infobulles. */
function stateLabel(game: Game, isRunning: boolean): string {
  if (isRunning) return t("game.running");
  if (!game.installed) return t("game.notInstalled");
  if (!game.available) return t("game.unavailableLong");
  return t("game.installed");
}

const Card = memo(function Card({ game, isRunning, onOpen, onPlay, onStop }: { game: Game; isRunning: boolean; onOpen: Props["onOpen"]; onPlay: Props["onPlay"]; onStop: Props["onStop"] }) {
  const unavailable = game.installed && !game.available;
  return (
    <div className="group relative flex min-w-0 flex-col">
      <button
        onClick={() => onOpen(game)}
        onDoubleClick={() => onPlay(game)}
        title={`${game.title} – ${stateLabel(game, isRunning)}`}
        className={`game-card relative aspect-[2/3] w-full overflow-hidden rounded-lg bg-panel2 shadow-md ring-1 ring-line transition group-hover:ring-accent ${
          unavailable ? "grayscale" : ""
        } ${!game.installed ? "opacity-55 group-hover:opacity-90" : ""}`}
      >
        <Cover title={game.title} path={game.coverPath} />
        <span className="absolute left-1.5 top-1.5 flex gap-1">
          {game.sources.slice(0, 2).map((s) => (
            <SourceBadge key={s.id} source={s.source} short />
          ))}
        </span>
        {game.favorite && <Heart size={14} className="absolute right-1.5 top-1.5 fill-danger text-danger drop-shadow" />}
        {unavailable && (
          <span className="absolute inset-0 flex items-center justify-center bg-scrim/45">
            <HardDrive size={30} className="text-on-scrim/85" />
          </span>
        )}
        {isRunning && <span className="absolute bottom-1.5 left-1.5 flex items-center gap-1 rounded bg-scrim/80 px-1.5 py-0.5 text-[10px] font-bold uppercase text-ok"><span className="h-1.5 w-1.5 animate-pulse rounded-full bg-ok" />{t("game.running")}</span>}
        {game.installed && game.available && !isRunning && <span className="absolute bottom-2 left-2 h-2.5 w-2.5 rounded-full bg-ok ring-2 ring-scrim/50" />}
      </button>
      {!unavailable && !isRunning && (
        <button
          onClick={() => onPlay(game)}
          tabIndex={-1}
          aria-label={game.installed ? t("game.play") : t("game.install")}
          className="absolute right-2 top-[calc(100%-5.25rem)] hidden h-10 w-10 items-center justify-center rounded-full bg-accent text-accent-fg shadow-lg hover:brightness-110 group-hover:flex"
        >
          {game.installed ? <Play size={18} className="ml-0.5 fill-current" /> : <Download size={18} />}
        </button>
      )}
      {isRunning && (
        <button
          onClick={() => onStop(game)}
          tabIndex={-1}
          aria-label={`Arrêter le suivi de ${game.title}`}
          title="Arrêter le suivi (le jeu n'est pas fermé)"
          className="absolute right-2 top-[calc(100%-5.25rem)] hidden h-10 w-10 items-center justify-center rounded-full bg-panel text-fg shadow-lg ring-1 ring-line hover:bg-danger hover:text-on-scrim group-hover:flex"
        >
          <Square size={15} className="fill-current" />
        </button>
      )}
      <div className="mt-1.5 h-9 px-0.5">
        <p className="truncate text-[13px] font-medium" title={game.title}>
          {game.title}
        </p>
        <p className="truncate text-[11px] text-muted">{game.playtimeMinutes > 0 ? formatMinutes(game.playtimeMinutes) : stateLabel(game, isRunning)}</p>
      </div>
    </div>
  );
});

const GAP = 16;
const MIN_CARD = 148;

export function GameGrid({ games, running, onOpen, onPlay, onStop }: Props) {
  const [ref, width] = useWidth<HTMLDivElement>();
  const scrollRef = useRef<HTMLDivElement>(null);
  const cols = Math.max(1, Math.floor((width + GAP) / (MIN_CARD + GAP)));
  const cardWidth = width > 0 ? (width - GAP * (cols - 1)) / cols : MIN_CARD;
  const rowHeight = Math.round(cardWidth * 1.5 + 42 + GAP);
  const rows = Math.ceil(games.length / cols);
  const virtualizer = useVirtualizer({ count: rows, getScrollElement: () => scrollRef.current, estimateSize: () => rowHeight, overscan: 2 });
  useEffect(() => virtualizer.measure(), [rowHeight, virtualizer]);

  return (
    <div ref={scrollRef} className="h-full overflow-y-auto px-5 pb-5">
      <div ref={ref} className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
        {width > 0 &&
          virtualizer.getVirtualItems().map((row) => (
            <div
              key={row.key}
              className="absolute left-0 grid w-full"
              style={{ top: row.start, height: rowHeight, gap: GAP, gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}
            >
              {games.slice(row.index * cols, row.index * cols + cols).map((game) => (
                <Card key={game.id} game={game} isRunning={running.has(game.id)} onOpen={onOpen} onPlay={onPlay} onStop={onStop} />
              ))}
            </div>
          ))}
      </div>
    </div>
  );
}

const ROW_HEIGHT = 52;

export function GameList({ games, running, onOpen, onPlay, onStop }: Props) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({ count: games.length, getScrollElement: () => scrollRef.current, estimateSize: () => ROW_HEIGHT, overscan: 8 });
  return (
    <div ref={scrollRef} className="h-full overflow-y-auto px-5 pb-5">
      <div className="sticky top-0 z-10 grid grid-cols-[2.25rem_minmax(0,1fr)_8rem_7rem_8rem_6rem_3rem] items-center gap-3 border-b border-line bg-bg px-2 py-2 text-xs uppercase tracking-wide text-muted">
        <span />
        <span>Nom</span>
        <span>Source</span>
        <span>Temps de jeu</span>
        <span>Dernier lancement</span>
        <span>Succès</span>
        <span />
      </div>
      <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((row) => {
          const game = games[row.index];
          const isRunning = running.has(game.id);
          const unavailable = game.installed && !game.available;
          return (
            <div
              key={game.id}
              role="button"
              tabIndex={0}
              onClick={() => onOpen(game)}
              onKeyDown={(e) => e.key === "Enter" && onOpen(game)}
              onDoubleClick={() => onPlay(game)}
              className={`absolute left-0 grid w-full cursor-pointer grid-cols-[2.25rem_minmax(0,1fr)_8rem_7rem_8rem_6rem_3rem] items-center gap-3 rounded-md px-2 hover:bg-panel2 ${
                !game.installed ? "opacity-60" : ""
              } ${unavailable ? "grayscale" : ""}`}
              style={{ top: row.start, height: ROW_HEIGHT }}
            >
              <div className="h-10 w-7 overflow-hidden rounded bg-panel2 text-[0px]">
                <Cover title={game.title} path={game.coverPath} />
              </div>
              <div className="min-w-0">
                <p className="truncate font-medium">{game.title}</p>
                <p className="flex items-center gap-1 truncate text-xs text-muted">
                  {unavailable && <HardDrive size={12} />}
                  {stateLabel(game, isRunning)}
                </p>
              </div>
              <span className="flex gap-1 overflow-hidden">
                {game.sources.slice(0, 2).map((s) => (
                  <SourceBadge key={s.id} source={s.source} short />
                ))}
              </span>
              <span className="text-sm tabular-nums">{game.playtimeMinutes > 0 ? formatMinutes(game.playtimeMinutes) : "–"}</span>
              <span className="text-sm text-muted">{formatRelative(game.lastPlayed)}</span>
              <span className="text-sm tabular-nums text-muted">{game.achTotal > 0 ? `${game.achUnlocked}/${game.achTotal} · ${percent(game.achUnlocked, game.achTotal)} %` : "–"}</span>
              {isRunning ? (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onStop(game);
                  }}
                  aria-label={`Arrêter le suivi de ${game.title}`}
                  title="Arrêter le suivi (le jeu n'est pas fermé)"
                  className="flex h-8 w-8 items-center justify-center rounded-full text-ok hover:bg-danger hover:text-on-scrim"
                >
                  <Square size={13} className="fill-current" />
                </button>
              ) : (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onPlay(game);
                  }}
                  disabled={unavailable}
                  aria-label={game.installed ? t("game.play") : t("game.install")}
                  className="flex h-8 w-8 items-center justify-center rounded-full text-muted hover:bg-accent hover:text-accent-fg disabled:opacity-30"
                >
                  {game.installed ? <Play size={15} className="ml-0.5 fill-current" /> : <Download size={15} />}
                </button>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
