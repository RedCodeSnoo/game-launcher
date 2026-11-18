// Backlog (O-02) : un tableau par statut ; glisser-déposer une carte change son statut.
import { useMemo, useState } from "react";
import { startDrag, useDragging, useDrop } from "../lib/drag";
import { Plus, Search } from "lucide-react";
import { api, imageUrl } from "../api";
import { errorMessage } from "../i18n";
import { formatMinutes, formatRelative } from "../lib/format";
import { searchGames, STATUSES } from "../lib/organize";
import type { Game, GameStatus } from "../types";
import { Cover } from "./Cover";
import { useToasts } from "./Toasts";

/** Colonne du backlog : zone de dépôt, mise en évidence pendant un glissement. */
function Column({ label, onDrop, children }: { label: string; onDrop: (gameId: string) => void; children: React.ReactNode }) {
  const ref = useDrop<HTMLElement>((p) => onDrop(p.gameId));
  const dragging = useDragging();
  return (
    <section ref={ref} data-drop="" className={`backlog-col flex min-h-0 flex-col rounded-xl border bg-panel transition ${dragging ? "border-dashed border-accent/50" : "border-line"}`} aria-label={label}>
      {children}
    </section>
  );
}

export function Backlog({ games, onOpen }: { games: Game[]; onOpen: (id: string) => void }) {
  const toasts = useToasts();
  const [query, setQuery] = useState("");
  const visible = useMemo(() => games.filter((g) => !g.hidden), [games]);
  const byStatus = useMemo(() => {
    const map = new Map<GameStatus, Game[]>();
    for (const s of STATUSES) map.set(s.id, []);
    for (const g of visible) if (g.status) map.get(g.status)?.push(g);
    for (const list of map.values()) list.sort((a, b) => (b.statusChangedAt ?? "").localeCompare(a.statusChangedAt ?? "") || a.sortTitle.localeCompare(b.sortTitle));
    return map;
  }, [visible]);
  const suggestions = useMemo(() => (query.trim() ? searchGames(visible.filter((g) => !g.status), query, 6) : []), [visible, query]);

  const move = async (gameId: string, status: GameStatus | null) => {
    try {
      await api.updateGame(gameId, { status });
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  return (
    <div className="flex h-full flex-col">
      <header className="flex flex-wrap items-center gap-3 px-6 py-4">
        <div className="min-w-0">
          <h1 className="text-lg font-semibold">Backlog</h1>
          <p className="max-w-2xl text-sm text-muted">
            Votre liste de jeux à faire : rangez chaque jeu selon où vous en êtes (à jouer, en cours, terminé…). Ajoutez un jeu avec la recherche ci-contre, depuis le menu « Statut » de sa fiche, ou en glissant sa jaquette sur « Backlog » dans la barre latérale ; glissez une carte d'une colonne à l'autre pour changer son statut.
          </p>
        </div>
        <div className="relative ml-auto w-72">
          <Search size={15} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Ajouter un jeu au backlog…"
            className="h-9 w-full rounded-md border border-line bg-panel pl-8 pr-2 text-sm outline-none focus:border-accent"
            aria-label="Ajouter un jeu au backlog"
          />
          {suggestions.length > 0 && (
            <ul className="absolute left-0 right-0 top-10 z-30 overflow-hidden rounded-lg border border-line bg-panel shadow-xl">
              {suggestions.map((g) => (
                <li key={g.id}>
                  <button
                    onClick={() => {
                      void move(g.id, "backlog");
                      setQuery("");
                    }}
                    className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm hover:bg-panel2"
                  >
                    <Plus size={14} className="text-accent" /> {g.title}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </header>
      <div className="grid min-h-0 flex-1 auto-cols-[minmax(11rem,1fr)] grid-flow-col gap-3 overflow-x-auto px-6 pb-6">
        {STATUSES.map((s) => {
          const list = byStatus.get(s.id) ?? [];
          return (
            <Column key={s.id} label={`${s.label} : ${list.length} jeu(x)`} onDrop={(id) => list.some((g) => g.id === id) || void move(id, s.id)}>
              <h2 className="flex items-center gap-2 border-b border-line px-3 py-2.5 text-sm font-semibold">
                <span className="h-2.5 w-2.5 rounded-full" style={{ background: s.color }} />
                {s.label}
                <span className="ml-auto text-xs font-normal tabular-nums text-muted">{list.length}</span>
              </h2>
              <ul className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2">
                {list.map((g) => (
                  <li key={g.id}>
                    <button
                      onPointerDown={(e) => startDrag(e, { gameId: g.id, title: g.title, cover: imageUrl(g.coverPath) })}
                      onClick={() => onOpen(g.id)}
                      className="flex w-full cursor-grab items-center gap-2.5 rounded-lg border border-line bg-panel2/70 p-2 text-left transition hover:border-accent active:cursor-grabbing"
                    >
                      <span className="h-14 w-10 shrink-0 overflow-hidden rounded">
                        <Cover title={g.title} path={g.coverPath} mini />
                      </span>
                      <span className="min-w-0">
                        <span className="line-clamp-2 block text-sm font-medium leading-tight">{g.title}</span>
                        <span className="block truncate text-xs text-muted">
                          {[g.playtimeMinutes > 0 ? formatMinutes(g.playtimeMinutes) : null, g.lastPlayed ? formatRelative(g.lastPlayed).toLowerCase() : null, g.rating ? `★ ${g.rating}/10` : null].filter(Boolean).join(" · ") || "Jamais lancé"}
                        </span>
                      </span>
                    </button>
                  </li>
                ))}
                {list.length === 0 && <li className="px-2 py-6 text-center text-xs text-muted">Déposez un jeu ici</li>}
              </ul>
            </Column>
          );
        })}
      </div>
    </div>
  );
}
