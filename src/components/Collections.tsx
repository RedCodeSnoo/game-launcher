// Collections personnalisées (F-NAV-06) et proposition de fusion des doublons (§5.2).
import { useEffect, useRef, useState } from "react";
import { FolderHeart, Plus, Trash2 } from "lucide-react";
import { api } from "../api";
import { errorMessage } from "../i18n";
import type { Collection, Game, MergeSuggestion } from "../types";
import { SourceBadge } from "./Cover";
import { useToasts } from "./Toasts";
import { Button, Modal, TextInput } from "./ui";

/** Menu de la fiche jeu : cocher les collections du jeu, en créer une. */
export function CollectionsMenu({ game, collections }: { game: Game; collections: Collection[] }) {
  const toasts = useToasts();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => ref.current && !ref.current.contains(e.target as Node) && setOpen(false);
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
  }, [open]);

  const guard = async (action: () => Promise<unknown>) => {
    try {
      await action();
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  return (
    <div ref={ref} className="relative">
      <Button variant="ghost" onClick={() => setOpen((o) => !o)} aria-expanded={open}>
        <FolderHeart size={17} /> Collections{game.collections.length > 0 ? ` (${game.collections.length})` : ""}
      </Button>
      {open && (
        <div className="absolute left-0 top-11 z-30 w-72 rounded-xl border border-line bg-panel p-3 shadow-2xl">
          {collections.length === 0 && <p className="pb-2 text-sm text-muted">Aucune collection pour l'instant.</p>}
          <ul className="flex max-h-56 flex-col overflow-y-auto">
            {collections.map((c) => (
              <li key={c.id}>
                <label className="flex cursor-pointer items-center gap-2 rounded px-1.5 py-1.5 text-sm hover:bg-panel2">
                  <input
                    type="checkbox"
                    checked={game.collections.includes(c.id)}
                    onChange={(e) => guard(() => api.setGameCollection(game.id, c.id, e.target.checked))}
                    className="h-4 w-4 accent-[var(--accent)]"
                  />
                  <span className="min-w-0 flex-1 truncate">{c.name}</span>
                </label>
              </li>
            ))}
          </ul>
          <form
            className="mt-2 flex gap-2 border-t border-line pt-3"
            onSubmit={(e) => {
              e.preventDefault();
              void guard(async () => {
                const id = await api.createCollection(name);
                await api.setGameCollection(game.id, id, true);
                setName("");
              });
            }}
          >
            <TextInput value={name} onChange={(e) => setName(e.target.value)} placeholder="Nouvelle collection" aria-label="Nom de la nouvelle collection" />
            <Button type="submit" variant="primary" disabled={!name.trim()} aria-label="Créer la collection">
              <Plus size={15} />
            </Button>
          </form>
        </div>
      )}
    </div>
  );
}

/** Boutons d'une collection ouverte dans la bibliothèque : renommer, supprimer. */
export function CollectionActions({ collection, onDeleted }: { collection: Collection; onDeleted: () => void }) {
  const toasts = useToasts();
  return (
    <>
      <Button
        onClick={async () => {
          const name = window.prompt("Nouveau nom de la collection", collection.name);
          if (!name || name === collection.name) return;
          await api.renameCollection(collection.id, name).catch((e) => toasts.error(errorMessage(e)));
        }}
      >
        Renommer
      </Button>
      <Button
        variant="danger"
        aria-label="Supprimer la collection"
        title="Supprimer la collection (les jeux ne sont pas touchés)"
        onClick={async () => {
          if (!window.confirm(`Supprimer la collection « ${collection.name} » ? Les jeux restent dans la bibliothèque.`)) return;
          await api.deleteCollection(collection.id).catch((e) => toasts.error(errorMessage(e)));
          onDeleted();
        }}
      >
        <Trash2 size={15} />
      </Button>
    </>
  );
}

/** Bandeau « doublons possibles » : même titre sur plusieurs sources. */
export function MergeBanner({ games, suggestions, onChanged }: { games: Game[]; suggestions: MergeSuggestion[]; onChanged: () => void }) {
  const toasts = useToasts();
  const [open, setOpen] = useState(false);
  if (suggestions.length === 0) return null;

  const act = async (action: () => Promise<unknown>, ok?: string) => {
    try {
      await action();
      if (ok) toasts.success(ok);
      onChanged();
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  return (
    <>
      <div className="mx-5 mb-3 flex items-center gap-3 rounded-lg border border-line bg-panel px-4 py-2 text-sm">
        <span className="flex-1">
          {suggestions.length} jeu(x) semblent présents sur plusieurs sources. Les fusionner regroupe leurs heures sur une seule fiche.
        </span>
        <Button onClick={() => setOpen(true)}>Voir</Button>
      </div>
      {open && (
        <Modal title="Doublons possibles" onClose={() => setOpen(false)}>
          <ul className="flex flex-col gap-2">
            {suggestions.map((s) => {
              const members = s.gameIds.map((id) => games.find((g) => g.id === id)).filter((g): g is Game => Boolean(g));
              return (
                <li key={s.gameIds.join()} className="flex items-center gap-3 rounded-md border border-line bg-bg p-3">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium">{s.title}</p>
                    <p className="mt-1 flex flex-wrap gap-1">
                      {members.flatMap((g) => g.sources).map((src) => (
                        <SourceBadge key={src.id} source={src.source} />
                      ))}
                    </p>
                  </div>
                  <Button onClick={() => act(() => api.dismissMerge(s.title))}>Garder séparés</Button>
                  <Button variant="primary" onClick={() => act(() => api.mergeGames(s.gameIds[0], s.gameIds.slice(1)), `« ${s.title} » fusionné`)}>
                    Fusionner
                  </Button>
                </li>
              );
            })}
          </ul>
        </Modal>
      )}
    </>
  );
}
