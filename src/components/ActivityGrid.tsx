// Grille d'activité façon GitHub (S2-02 à S2-05) : 53 semaines × 7 jours, 5 niveaux dans la couleur d'accent.
// Survol : date, temps et jeux du jour (avec jaquettes). Clic : sessions du jour.
import { createPortal } from "react-dom";
import { useEffect, useMemo, useRef, useState } from "react";
import { api, on } from "../api";
import { formatDateTime, formatSeconds } from "../lib/format";
import type { ActivityGrid as Grid, DayDetail } from "../types";
import { Cover } from "./Cover";
import { Modal } from "./ui";

const CELL = 12;
const GAP = 3;
const LEFT = 28;
const TOP = 16;
const MONTHS = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];

/** Niveau d'intensité (0 à 4) d'une journée. */
export function level(seconds: number): number {
  if (seconds <= 0) return 0;
  if (seconds < 1800) return 1;
  if (seconds < 5400) return 2;
  if (seconds < 10800) return 3;
  return 4;
}

export const LEVEL_FILL = [
  "var(--panel2)",
  "color-mix(in srgb, var(--accent) 30%, var(--panel2))",
  "color-mix(in srgb, var(--accent) 55%, var(--panel2))",
  "color-mix(in srgb, var(--accent) 78%, var(--panel2))",
  "var(--accent)",
];

function parseDay(s: string): Date {
  const [y, m, d] = s.split("-").map(Number);
  return new Date(y, m - 1, d, 12);
}

function dayKey(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function longDate(day: string): string {
  return parseDay(day).toLocaleDateString("fr-FR", { weekday: "long", day: "numeric", month: "long", year: "numeric" });
}

interface Cell {
  day: string;
  seconds: number;
  col: number;
  row: number;
  future: boolean;
}

/** Cases de la grille, colonnes alignées sur les semaines (lundi en haut). */
export function buildCells(grid: Grid): { cells: Cell[]; columns: number; months: { col: number; label: string }[] } {
  const totals = new Map(grid.days.map((d) => [d.day, d.seconds]));
  const start = parseDay(grid.start);
  const end = parseDay(grid.end);
  const first = new Date(start);
  first.setDate(first.getDate() - ((first.getDay() + 6) % 7));
  const cells: Cell[] = [];
  const months: { col: number; label: string }[] = [];
  let col = 0;
  for (const d = new Date(first); d <= end; d.setDate(d.getDate() + 1)) {
    const row = (d.getDay() + 6) % 7;
    if (row === 0 && cells.length > 0) col++;
    if (d < start) continue;
    const key = dayKey(d);
    if (d.getDate() === 1 || cells.length === 0) {
      if (!months.length || months[months.length - 1].col < col - 2) months.push({ col, label: MONTHS[d.getMonth()] });
    }
    cells.push({ day: key, seconds: totals.get(key) ?? 0, col, row, future: key > grid.today });
  }
  return { cells, columns: col + 1, months };
}

function DaySessions({ detail, onClose }: { detail: DayDetail; onClose: () => void }) {
  return (
    <Modal title={longDate(detail.day)} onClose={onClose}>
      <p className="mb-3 text-sm text-muted">{detail.seconds > 0 ? `${formatSeconds(detail.seconds)} de jeu` : "Aucune partie ce jour-là."}</p>
      {detail.games.length > 0 && (
        <div className="mb-4 flex flex-wrap gap-3">
          {detail.games.map((g) => (
            <div key={g.gameId} className="flex items-center gap-2 rounded-lg border border-line bg-panel2 p-1.5 pr-3">
              <div className="h-12 w-8 shrink-0 overflow-hidden rounded">
                <Cover title={g.title} path={g.coverPath} mini />
              </div>
              <div className="min-w-0">
                <p className="max-w-44 truncate text-sm font-medium">{g.title}</p>
                <p className="text-xs text-muted">{formatSeconds(g.seconds)}</p>
              </div>
            </div>
          ))}
        </div>
      )}
      {detail.sessions.length > 0 && (
        <table className="w-full text-sm">
          <thead className="text-left text-xs uppercase tracking-wide text-muted">
            <tr>
              <th className="py-1.5 font-medium">Jeu</th>
              <th className="py-1.5 font-medium">Début</th>
              <th className="py-1.5 font-medium">Durée</th>
              <th className="py-1.5 font-medium">Origine</th>
            </tr>
          </thead>
          <tbody>
            {detail.sessions.map((s) => (
              <tr key={s.id} className="border-t border-line">
                <td className="max-w-48 truncate py-1.5">{s.gameTitle}</td>
                <td className="py-1.5 text-muted">{formatDateTime(s.startedAt)}</td>
                <td className="py-1.5 tabular-nums">{formatSeconds(s.durationSeconds)}</td>
                <td className="py-1.5 text-muted">{s.origin === "detected" ? "Détectée" : "Lancée depuis l'app"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </Modal>
  );
}

export function ActivityGrid({ gameId, showYearPicker = true }: { gameId?: string; showYearPicker?: boolean }) {
  const [year, setYear] = useState<number | undefined>(undefined);
  const [grid, setGrid] = useState<Grid | null>(null);
  const [hover, setHover] = useState<{ cell: Cell; x: number; y: number; bottom: number } | null>(null);
  // L'infobulle est placée par rapport à la fenêtre : on la retire dès que la page défile.
  useEffect(() => {
    if (!hover) return;
    const clear = () => setHover(null);
    window.addEventListener("scroll", clear, true);
    return () => window.removeEventListener("scroll", clear, true);
  }, [hover]);
  const [detail, setDetail] = useState<DayDetail | null>(null);
  const [open, setOpen] = useState<DayDetail | null>(null);
  const cache = useRef(new Map<string, DayDetail>());
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const load = () => {
      cache.current.clear();
      void api.getActivity(year, gameId).then(setGrid).catch(() => {});
    };
    load();
    return on<null>("library-updated", load);
  }, [year, gameId]);

  // Détail du jour survolé, chargé après un court délai.
  useEffect(() => {
    if (!hover || hover.cell.seconds === 0) return setDetail(null);
    const key = hover.cell.day;
    const known = cache.current.get(key);
    if (known) return setDetail(known);
    const t = setTimeout(() => {
      void api.getActivityDay(key, gameId).then((d) => {
        cache.current.set(key, d);
        setDetail(d);
      });
    }, 120);
    return () => clearTimeout(t);
  }, [hover, gameId]);

  const layout = useMemo(() => (grid ? buildCells(grid) : null), [grid]);
  if (!grid || !layout) return <div className="skeleton h-36 rounded-lg" />;
  const width = LEFT + layout.columns * (CELL + GAP);
  const height = TOP + 7 * (CELL + GAP);
  const total = grid.days.reduce((n, d) => n + d.seconds, 0);
  const active = grid.days.filter((d) => d.seconds >= grid.minMinutes * 60).length;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-sm text-muted">
          <span className="font-semibold text-fg">{formatSeconds(total)}</span> sur <span className="font-semibold text-fg">{active}</span> jour{active > 1 ? "s" : ""} joué{active > 1 ? "s" : ""}
          {year ? ` en ${year}` : " ces 12 derniers mois"}
        </p>
        {showYearPicker && (
          <select
            className="h-8 rounded-md border border-line bg-bg px-2 text-sm"
            value={year ?? ""}
            onChange={(e) => setYear(e.target.value ? Number(e.target.value) : undefined)}
            aria-label="Période de la grille"
          >
            <option value="">12 derniers mois</option>
            {grid.years.map((y) => (
              <option key={y} value={y}>
                {y}
              </option>
            ))}
          </select>
        )}
      </div>
      <div ref={box} className="relative overflow-x-auto" onMouseLeave={() => setHover(null)}>
        <svg viewBox={`0 0 ${width} ${height}`} className="w-full min-w-[640px]" role="img" aria-label={`Grille d'activité : ${formatSeconds(total)}`}>
          {layout.months.map((m) => (
            <text key={`${m.col}-${m.label}`} x={LEFT + m.col * (CELL + GAP)} y={10} className="fill-muted" fontSize="9">
              {m.label}
            </text>
          ))}
          {[
            [0, "lun."],
            [2, "mer."],
            [4, "ven."],
          ].map(([row, label]) => (
            <text key={label} x={0} y={TOP + (row as number) * (CELL + GAP) + 9.5} className="fill-muted" fontSize="9">
              {label}
            </text>
          ))}
          {layout.cells.map((c) => (
            <rect
              key={c.day}
              x={LEFT + c.col * (CELL + GAP)}
              y={TOP + c.row * (CELL + GAP)}
              width={CELL}
              height={CELL}
              rx={Math.min(3, CELL / 4)}
              fill={c.future ? "transparent" : LEVEL_FILL[level(c.seconds)]}
              stroke={c.day === grid.today ? "var(--fg)" : c.future ? "var(--line)" : "none"}
              strokeWidth={c.day === grid.today ? 1.2 : 0.6}
              className={c.future ? "" : "cursor-pointer"}
              onMouseEnter={(e) => {
                const r = box.current?.getBoundingClientRect();
                const t = (e.target as SVGRectElement).getBoundingClientRect();
                if (r) setHover({ cell: c, x: t.left + t.width / 2, y: t.top, bottom: t.bottom });
              }}
              onClick={() => {
                if (c.future) return;
                void api.getActivityDay(c.day, gameId).then(setOpen);
              }}
            />
          ))}
        </svg>
        {/* Infobulle hors de la zone qui défile (sinon coupée en haut), retournée sous la case faute de place au-dessus. */}
        {hover && !hover.cell.future && createPortal(
          <div
            className={`pointer-events-none fixed z-[70] w-64 -translate-x-1/2 rounded-lg border border-line bg-panel p-3 text-xs shadow-xl ${hover.y < 200 ? "" : "-translate-y-full"}`}
            style={{ left: Math.min(Math.max(hover.x, 140), window.innerWidth - 140), top: hover.y < 200 ? hover.bottom + 6 : hover.y - 6 }}
          >
            <p className="font-semibold capitalize">{longDate(hover.cell.day)}</p>
            <p className="text-muted">{hover.cell.seconds > 0 ? formatSeconds(hover.cell.seconds) : "Pas de partie"}</p>
            {detail && detail.day === hover.cell.day && detail.games.length > 0 && (
              <ul className="mt-2 flex flex-col gap-1.5">
                {detail.games.slice(0, 4).map((g) => (
                  <li key={g.gameId} className="flex items-center gap-2">
                    <span className="h-7 w-5 shrink-0 overflow-hidden rounded-sm">
                      <Cover title={g.title} path={g.coverPath} mini />
                    </span>
                    <span className="min-w-0 flex-1 truncate">{g.title}</span>
                    <span className="tabular-nums text-muted">{formatSeconds(g.seconds)}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>,
          document.body,
        )}
      </div>
      <div className="flex items-center justify-end gap-1.5 text-[11px] text-muted">
        Moins
        {LEVEL_FILL.map((f, i) => (
          <span key={i} className="h-3 w-3 rounded-[3px]" style={{ background: f }} />
        ))}
        Plus
      </div>
      {open && <DaySessions detail={open} onClose={() => setOpen(null)} />}
    </div>
  );
}
