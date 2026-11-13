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
      style={{ background: `linear-gradient(160deg, hsl(${hue} 45% 32%), hsl(${(hue + 50) % 360} 50% 16%))` }}
    >
      <span className="line-clamp-4 text-sm font-semibold text-white/90">{title}</span>
    </div>
  );
}

export function SourceBadge({ source, short = false }: { source: SourceId; short?: boolean }) {
  return (
    <span
      className="inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-white"
      style={{ background: SOURCE_COLORS[source] }}
      title={SOURCE_LABELS[source]}
    >
      {short ? SOURCE_LABELS[source].split(" ")[0] : SOURCE_LABELS[source]}
    </span>
  );
}
