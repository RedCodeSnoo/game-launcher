// Graphiques des statistiques (visx) : couleurs, polices et arrondis du thème actif (variables CSS).
import { AxisBottom, AxisLeft } from "@visx/axis";
import { Group } from "@visx/group";
import { ParentSize } from "@visx/responsive";
import { scaleBand, scaleLinear } from "@visx/scale";
import { Bar } from "@visx/shape";
import { formatMinutes, formatSeconds } from "../lib/format";
import type { Bucket } from "../types";

const axisLabel = { fill: "var(--muted)", fontSize: 10, fontFamily: "var(--t-font-body)" } as const;

function bucketLabel(b: Bucket, monthly: boolean): string {
  const d = new Date(`${b.start}T12:00:00`);
  return monthly ? d.toLocaleDateString("fr-FR", { month: "short" }) : d.toLocaleDateString("fr-FR", { day: "numeric", month: "numeric" });
}

/** Barres verticales : temps par semaine ou par mois. */
export function TimeBars({ buckets, monthly, height = 200 }: { buckets: Bucket[]; monthly: boolean; height?: number }) {
  const total = buckets.reduce((n, b) => n + b.seconds, 0);
  return (
    <div style={{ height }}>
    <ParentSize debounceTime={60}>
      {({ width }) => {
        if (width < 10) return null;
        const margin = { top: 8, right: 4, bottom: 22, left: 40 };
        const innerW = width - margin.left - margin.right;
        const innerH = height - margin.top - margin.bottom;
        const maxHours = Math.max(1, Math.ceil(Math.max(...buckets.map((b) => b.seconds)) / 3600));
        const x = scaleBand<string>({ domain: buckets.map((b) => b.start), range: [0, innerW], padding: 0.28 });
        const y = scaleLinear<number>({ domain: [0, maxHours], range: [innerH, 0], nice: true });
        return (
          <svg width={width} height={height} role="img" aria-label={`Temps de jeu ${monthly ? "par mois" : "par semaine"} : ${formatSeconds(total)}`}>
            <Group left={margin.left} top={margin.top}>
              {y.ticks(4).map((t) => (
                <line key={t} x1={0} x2={innerW} y1={y(t)} y2={y(t)} stroke="var(--line)" strokeDasharray="2 4" />
              ))}
              {buckets.map((b) => {
                const h = innerH - y(b.seconds / 3600);
                const bx = x(b.start) ?? 0;
                return (
                  <g key={b.start}>
                    <title>{`${monthly ? "" : "Semaine du "}${bucketLabel(b, monthly)} : ${formatSeconds(b.seconds)}`}</title>
                    <rect x={bx} y={0} width={x.bandwidth()} height={innerH} fill="transparent" />
                    <Bar x={bx} y={innerH - h} width={x.bandwidth()} height={Math.max(h, b.seconds > 0 ? 2 : 0)} rx={Math.min(4, x.bandwidth() / 3)} fill="var(--accent)" opacity={0.9} />
                  </g>
                );
              })}
              <AxisLeft scale={y} numTicks={4} hideAxisLine hideTicks tickFormat={(v) => `${v} h`} tickLabelProps={() => ({ ...axisLabel, textAnchor: "end", dx: -4, dy: 3 })} />
              <AxisBottom
                top={innerH}
                scale={x}
                hideAxisLine
                hideTicks
                tickFormat={(s) => bucketLabel(buckets.find((b) => b.start === s)!, monthly)}
                tickLabelProps={() => ({ ...axisLabel, textAnchor: "middle" })}
              />
            </Group>
          </svg>
        );
      }}
    </ParentSize>
    </div>
  );
}

export interface HBarItem {
  key: string;
  label: string;
  minutes: number;
  sub?: string;
  color?: string;
}

/** Barres horizontales triées (répartition par launcher, par genre). */
export function HBars({ items, empty = "Aucune donnée." }: { items: HBarItem[]; empty?: string }) {
  if (items.length === 0) return <p className="py-6 text-center text-sm text-muted">{empty}</p>;
  const max = Math.max(...items.map((i) => i.minutes), 1);
  const scale = scaleLinear<number>({ domain: [0, max], range: [0, 100] });
  return (
    <ul className="flex flex-col gap-3">
      {items.map((i) => (
        <li key={i.key}>
          <div className="mb-1 flex items-baseline justify-between gap-3 text-sm">
            <span className="flex min-w-0 items-center gap-2">
              {i.color && <span className="h-2.5 w-2.5 shrink-0 rounded-full" style={{ background: i.color }} />}
              <span className="truncate">{i.label}</span>
              {i.sub && <span className="shrink-0 text-xs text-muted">{i.sub}</span>}
            </span>
            <span className="shrink-0 tabular-nums font-medium">{formatMinutes(i.minutes)}</span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-panel2">
            <div className="h-full rounded-full" style={{ width: `${scale(i.minutes)}%`, background: i.color ?? "var(--accent)" }} />
          </div>
        </li>
      ))}
    </ul>
  );
}

const DAYS = ["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"];

/** Carte des habitudes : 7 jours × 24 heures. */
export function HabitsHeatmap({ habits }: { habits: number[][] }) {
  const max = Math.max(1, ...habits.flat());
  const busiest = habits.flatMap((row, d) => row.map((s, h) => ({ d, h, s }))).sort((a, b) => b.s - a.s)[0];
  return (
    <div className="flex flex-col gap-2">
      <div className="grid grid-cols-[2.5rem_repeat(24,minmax(0,1fr))] gap-[3px] text-[10px] text-muted">
        <span />
        {Array.from({ length: 24 }, (_, h) => (
          <span key={h} className="text-center">
            {h % 6 === 0 ? `${h}h` : ""}
          </span>
        ))}
        {habits.map((row, d) => (
          <Row key={d} label={DAYS[d]} row={row} max={max} />
        ))}
      </div>
      {busiest && busiest.s > 0 && (
        <p className="text-xs text-muted">
          Moment favori : le <span className="font-medium text-fg">{DAYS[busiest.d].toLowerCase()}</span> vers <span className="font-medium text-fg">{busiest.h} h</span>.
        </p>
      )}
    </div>
  );
}

function Row({ label, row, max }: { label: string; row: number[]; max: number }) {
  return (
    <>
      <span className="self-center">{label}</span>
      {row.map((s, h) => (
        <span
          key={h}
          className="aspect-square rounded-[3px]"
          title={`${label} ${h} h : ${formatSeconds(s)}`}
          style={{ background: s > 0 ? `color-mix(in srgb, var(--accent) ${Math.round(18 + (s / max) * 82)}%, var(--panel2))` : "var(--panel2)" }}
        />
      ))}
    </>
  );
}
