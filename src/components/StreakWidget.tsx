// Widget de série (S3-01) : flamme, série en cours et mini-grille des 4 dernières semaines.
import { useEffect, useState } from "react";
import { Flame } from "lucide-react";
import { api, on } from "../api";
import { formatSeconds } from "../lib/format";
import type { ActivityGrid, Streaks } from "../types";
import { LEVEL_FILL, level } from "./ActivityGrid";

function lastDays(grid: ActivityGrid, n: number): { day: string; seconds: number }[] {
  const totals = new Map(grid.days.map((d) => [d.day, d.seconds]));
  const [y, m, d] = grid.today.split("-").map(Number);
  const out = [];
  for (let i = n - 1; i >= 0; i--) {
    const date = new Date(y, m - 1, d - i, 12);
    const key = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
    out.push({ day: key, seconds: totals.get(key) ?? 0 });
  }
  return out;
}

export function StreakWidget({ onOpen, large }: { onOpen?: () => void; large?: boolean }) {
  const [streaks, setStreaks] = useState<Streaks | null>(null);
  const [grid, setGrid] = useState<ActivityGrid | null>(null);
  useEffect(() => {
    const load = () => {
      void api.getStreaks().then(setStreaks).catch(() => {});
      void api.getActivity().then(setGrid).catch(() => {});
    };
    load();
    return on<null>("library-updated", load);
  }, []);
  if (!streaks || !grid) return null;
  const days = lastDays(grid, 28);
  const label = streaks.current > 0 ? `${streaks.current} jour${streaks.current > 1 ? "s" : ""}` : "Pas de série";
  return (
    <button
      onClick={onOpen}
      className={`flex w-full flex-col gap-2 rounded-lg border border-line bg-panel2/60 text-left transition hover:border-accent ${large ? "p-4" : "p-2.5"}`}
      title={streaks.inDanger ? "Jouez aujourd'hui pour garder votre série" : `Meilleure série : ${streaks.best} jours`}
      aria-label={`Série en cours : ${label}`}
    >
      <span className="flex items-center gap-2">
        <Flame size={large ? 22 : 16} className={streaks.current > 0 ? (streaks.inDanger ? "text-warn" : "text-accent") : "text-muted"} />
        <span className={`font-display font-semibold ${large ? "text-lg" : "text-sm"}`}>{label}</span>
        {streaks.inDanger && <span className="ml-auto text-[10px] font-semibold uppercase tracking-wide text-warn">à jouer</span>}
        {!streaks.inDanger && streaks.jokersAvailable > 0 && (
          <span className="ml-auto text-[11px] text-muted" title="Jokers de série disponibles">
            🃏 {streaks.jokersAvailable}
          </span>
        )}
      </span>
      <span className="grid grid-cols-7 gap-[3px]" aria-hidden="true">
        {days.map((d) => (
          <span
            key={d.day}
            className={`${large ? "h-4" : "h-2"} rounded-[2px]`}
            style={{ background: LEVEL_FILL[level(d.seconds)], outline: d.day === grid.today ? "1px solid var(--fg)" : undefined }}
            title={`${d.day} : ${d.seconds > 0 ? formatSeconds(d.seconds) : "pas de partie"}`}
          />
        ))}
      </span>
    </button>
  );
}
