// Écran principal : barre du haut (recherche, vue, Actualiser) et grille ou liste des jeux.
import { useEffect, useMemo, useRef, type ReactNode } from "react";
import { LayoutGrid, List, RefreshCw, Search, X } from "lucide-react";
import { GameGrid, GameList } from "./GameGrid";
import { t } from "../i18n";
import { applyFilters, type Filters, type Section } from "../lib/filter";
import type { Game } from "../types";

interface Props {
  games: Game[];
  loading: boolean;
  scanning: boolean;
  running: Set<string>;
  section: Section;
  title: string;
  filters: Filters;
  onFilters: (f: Filters) => void;
  view: "grid" | "list";
  onView: (v: "grid" | "list") => void;
  onRefresh: () => void;
  onOpen: (g: Game) => void;
  onPlay: (g: Game) => void;
  onStop: (g: Game) => void;
  toolbarExtra?: ReactNode;
  banner?: ReactNode;
}

export function IconButton({ label, active, onClick, children, disabled }: { label: string; active?: boolean; onClick: () => void; children: ReactNode; disabled?: boolean }) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={label}
      aria-label={label}
      aria-pressed={active}
      className={`flex h-9 w-9 items-center justify-center rounded-md border border-line transition ${
        active ? "bg-accent text-accent-fg" : "bg-panel text-muted hover:text-fg"
      } disabled:opacity-50`}
    >
      {children}
    </button>
  );
}

export function Library(props: Props) {
  const { games, section, filters, onFilters, view } = props;
  const searchRef = useRef<HTMLInputElement>(null);
  const shown = useMemo(() => applyFilters(games, section, filters), [games, section, filters]);

  // Ctrl+F ou « / » : accès direct à la recherche au clavier.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement;
      if ((e.ctrlKey && e.key.toLowerCase() === "f") || (e.key === "/" && !typing)) {
        e.preventDefault();
        searchRef.current?.focus();
        searchRef.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="flex h-full min-w-0 flex-col">
      <header className="flex flex-wrap items-center gap-2 px-5 py-3">
        <h1 className="mr-2 text-lg font-semibold">{props.title}</h1>
        <span className="mr-auto text-xs text-muted">{t("lib.count", { n: shown.length })}</span>
        <label className="relative">
          <Search size={15} className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-muted" />
          <input
            ref={searchRef}
            value={filters.search}
            onChange={(e) => onFilters({ ...filters, search: e.target.value })}
            onKeyDown={(e) => e.key === "Escape" && onFilters({ ...filters, search: "" })}
            placeholder={t("lib.search")}
            aria-label={t("lib.search")}
            className="h-9 w-64 rounded-md border border-line bg-panel pl-8 pr-8 text-sm outline-none placeholder:text-muted focus:border-accent"
          />
          {filters.search && (
            <button onClick={() => onFilters({ ...filters, search: "" })} className="absolute right-2 top-1/2 -translate-y-1/2 text-muted hover:text-fg" aria-label="Effacer la recherche">
              <X size={14} />
            </button>
          )}
        </label>
        {props.toolbarExtra}
        <div className="flex gap-1">
          <IconButton label={t("lib.grid")} active={view === "grid"} onClick={() => props.onView("grid")}>
            <LayoutGrid size={16} />
          </IconButton>
          <IconButton label={t("lib.list")} active={view === "list"} onClick={() => props.onView("list")}>
            <List size={16} />
          </IconButton>
        </div>
        <button
          onClick={props.onRefresh}
          disabled={props.scanning}
          className="flex h-9 items-center gap-2 rounded-md border border-line bg-panel px-3 text-sm hover:border-accent disabled:opacity-60"
        >
          <RefreshCw size={15} className={props.scanning ? "animate-spin" : ""} />
          {props.scanning ? t("lib.refreshing") : t("lib.refresh")}
        </button>
      </header>
      {props.banner}
      <div className="min-h-0 flex-1">
        {props.loading ? (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(148px,1fr))] gap-4 px-5">
            {Array.from({ length: 18 }, (_, i) => (
              <div key={i} className="skeleton aspect-[2/3] rounded-lg" />
            ))}
          </div>
        ) : shown.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-1 pb-20 text-center">
            <p className="text-base font-medium">{t("lib.empty")}</p>
            <p className="text-sm text-muted">{t("lib.emptyHint")}</p>
          </div>
        ) : view === "grid" ? (
          <GameGrid games={shown} running={props.running} onOpen={props.onOpen} onPlay={props.onPlay} onStop={props.onStop} />
        ) : (
          <GameList games={shown} running={props.running} onOpen={props.onOpen} onPlay={props.onPlay} onStop={props.onStop} />
        )}
      </div>
    </div>
  );
}
