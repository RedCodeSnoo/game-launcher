// Jaquette d'un jeu, avec une vignette de remplacement tant que l'image n'est pas en cache.
import { useEffect, useState } from "react";
import { imageUrl } from "../api";
import { hueOf } from "../lib/format";
import { SOURCE_COLORS, SOURCE_LABELS } from "../i18n";
import type { SourceId } from "../types";

export function Cover({ title, path, className = "" }: { title: string; path: string | null; className?: string }) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [path]);
  const url = imageUrl(path);
  if (url && !failed) {
    return <img src={url} alt="" loading="lazy" draggable={false} onError={() => setFailed(true)} className={`h-full w-full object-cover ${className}`} />;
  }
  const hue = hueOf(title);
  return (
    <div
      className={`flex h-full w-full items-center justify-center p-3 text-center ${className}`}
      style={{ background: placeholderGradient(hue) }}
    >
      <span className="font-display line-clamp-4 text-sm font-semibold text-on-scrim/90">{title}</span>
    </div>
  );
}

/** Vignette de remplacement : teinte propre au jeu, mêlée aux couleurs du thème. */
export function placeholderGradient(hue: number, angle = 160): string {
  return `linear-gradient(${angle}deg, color-mix(in oklch, hsl(${hue} 45% 34%) 72%, var(--accent)), color-mix(in oklch, hsl(${(hue + 50) % 360} 50% 16%) 70%, var(--scrim)))`;
}

export function SourceBadge({ source, short = false }: { source: SourceId; short?: boolean }) {
  return (
    <span
      className="inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-on-scrim"
      style={{ background: SOURCE_COLORS[source] }}
      title={SOURCE_LABELS[source]}
    >
      {short ? SOURCE_LABELS[source].split(" ")[0] : SOURCE_LABELS[source]}
    </span>
  );
}
