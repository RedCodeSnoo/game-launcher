// Filtres (source, installé, disque, genre) et tris de la bibliothèque (F-NAV-03, F-NAV-04).
import { useEffect, useRef, useState } from "react";
import { ArrowDownAZ, ArrowUpAZ, SlidersHorizontal } from "lucide-react";
import { SOURCE_LABELS } from "../i18n";
import { DEFAULT_FILTERS, listDrives, listGenres, listSources, type Filters, type SortKey } from "../lib/filter";
import type { Game } from "../types";

const SORTS: { id: SortKey; label: string }[] = [
  { id: "name", label: "Nom" },
  { id: "lastPlayed", label: "Dernier lancement" },
  { id: "playtime", label: "Temps de jeu" },
  { id: "added", label: "Date d'ajout" },
  { id: "achievements", label: "Progression des succès" },
];

export function activeFilterCount(f: Filters): number {
  return (f.installed !== "all" ? 1 : 0) + f.sources.length + (f.drive ? 1 : 0) + (f.genre ? 1 : 0);
}

function Chip({ active, onClick, children }: { active: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      onClick={onClick}
      aria-pressed={active}
      className={`rounded-full border px-2.5 py-1 text-xs transition ${active ? "border-accent bg-accent/20 text-fg" : "border-line text-muted hover:text-fg"}`}
    >
      {children}
    </button>
  );
}

export function FilterBar({ games, filters, onChange }: { games: Game[]; filters: Filters; onChange: (f: Filters) => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const count = activeFilterCount(filters);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const sources = listSources(games);
  const drives = listDrives(games);
  const genres = listGenres(games);
  const toggleSource = (s: (typeof sources)[number]) =>
    onChange({ ...filters, sources: filters.sources.includes(s) ? filters.sources.filter((x) => x !== s) : [...filters.sources, s] });

  return (
    <>
      <div ref={ref} className="relative">
        <button
          onClick={() => setOpen((o) => !o)}
          aria-expanded={open}
          className={`flex h-9 items-center gap-2 rounded-md border px-3 text-sm ${count > 0 ? "border-accent bg-accent/15" : "border-line bg-panel"} hover:border-accent`}
        >
          <SlidersHorizontal size={15} /> Filtres
          {count > 0 && <span className="rounded-full bg-accent px-1.5 text-xs font-semibold text-accent-fg">{count}</span>}
        </button>
        {open && (
          <div className="absolute right-0 top-11 z-30 flex w-96 flex-col gap-4 rounded-xl border border-line bg-panel p-4 shadow-2xl">
            <div>
              <p className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-muted">État</p>
              <div className="flex flex-wrap gap-1.5">
                <Chip active={filters.installed === "all"} onClick={() => onChange({ ...filters, installed: "all" })}>
                  Tous
                </Chip>
                <Chip active={filters.installed === "installed"} onClick={() => onChange({ ...filters, installed: "installed" })}>
                  Installés
                </Chip>
                <Chip active={filters.installed === "notInstalled"} onClick={() => onChange({ ...filters, installed: "notInstalled" })}>
                  Non installés
                </Chip>
              </div>
            </div>
            {sources.length > 1 && (
              <div>
                <p className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-muted">Source</p>
                <div className="flex flex-wrap gap-1.5">
                  {sources.map((s) => (
                    <Chip key={s} active={filters.sources.includes(s)} onClick={() => toggleSource(s)}>
                      {SOURCE_LABELS[s]}
                    </Chip>
                  ))}
                </div>
              </div>
            )}
            {drives.length > 0 && (
              <div>
                <p className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-muted">Disque</p>
                <div className="flex flex-wrap gap-1.5">
                  {drives.map((d) => (
                    <Chip key={d} active={filters.drive === d} onClick={() => onChange({ ...filters, drive: filters.drive === d ? null : d })}>
                      {d}
                    </Chip>
                  ))}
                </div>
              </div>
            )}
            {genres.length > 0 && (
              <label className="flex flex-col gap-1.5">
                <span className="text-xs font-semibold uppercase tracking-wide text-muted">Genre</span>
                <select
                  value={filters.genre ?? ""}
                  onChange={(e) => onChange({ ...filters, genre: e.target.value || null })}
                  className="h-9 rounded-md border border-line bg-bg px-2 text-sm"
                >
                  <option value="">Tous les genres</option>
                  {genres.map((g) => (
                    <option key={g} value={g}>
                      {g}
                    </option>
                  ))}
                </select>
              </label>
            )}
            {count > 0 && (
              <button
                className="self-start text-xs text-accent hover:underline"
                onClick={() => onChange({ ...filters, installed: DEFAULT_FILTERS.installed, sources: [], drive: null, genre: null })}
              >
                Réinitialiser les filtres
              </button>
            )}
          </div>
        )}
      </div>
      <div className="flex">
        <select
          value={filters.sort}
          onChange={(e) => {
            const sort = e.target.value as SortKey;
            // Par défaut : A→Z pour le nom, du plus grand au plus petit pour le reste.
            onChange({ ...filters, sort, sortDesc: sort !== "name" });
          }}
          aria-label="Trier par"
          className="h-9 rounded-l-md border border-r-0 border-line bg-panel px-2 text-sm"
        >
          {SORTS.map((s) => (
            <option key={s.id} value={s.id}>
              {s.label}
            </option>
          ))}
        </select>
        <button
          onClick={() => onChange({ ...filters, sortDesc: !filters.sortDesc })}
          title={filters.sortDesc ? "Ordre décroissant" : "Ordre croissant"}
          aria-label={filters.sortDesc ? "Ordre décroissant" : "Ordre croissant"}
          className="flex h-9 w-9 items-center justify-center rounded-r-md border border-line bg-panel text-muted hover:text-fg"
        >
          {filters.sortDesc ? <ArrowUpAZ size={16} /> : <ArrowDownAZ size={16} />}
        </button>
      </div>
    </>
  );
}
