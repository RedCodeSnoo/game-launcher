// Recherche rapide Ctrl+K (O-05) : trouver et lancer un jeu, ouvrir une page, changer de thème, au clavier.
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { BarChart3, Dices, Home, Kanban, Library, Palette, Play, Search, Settings, SquareArrowOutUpRight, Tv, UserRound } from "lucide-react";
import { fuzzyScore, searchGames } from "../lib/organize";
import { useTheme } from "../themes/ThemeContext";
import type { Game } from "../types";
import { Cover } from "./Cover";

export type PaletteTarget = "home" | "library" | "backlog" | "stats" | "profile" | "settings";

interface Item {
  id: string;
  group: "Jeux" | "Pages" | "Thèmes" | "Actions";
  label: string;
  hint?: string;
  icon: ReactNode;
  run: () => void;
  /** Action secondaire (Maj + Entrée). */
  alt?: () => void;
}

interface Props {
  games: Game[];
  onClose: () => void;
  onPlay: (g: Game) => void;
  onOpenGame: (id: string) => void;
  onPage: (p: PaletteTarget) => void;
  onRoulette: () => void;
  onBigPicture?: () => void;
}

export function CommandPalette({ games, onClose, onPlay, onOpenGame, onPage, onRoulette, onBigPicture }: Props) {
  const [query, setQuery] = useState("");
  const [index, setIndex] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const { themes, setTheme } = useTheme();

  const items = useMemo<Item[]>(() => {
    const close = (f: () => void) => () => {
      f();
      onClose();
    };
    const gameItems: Item[] = searchGames(games, query, query ? 8 : 5).map((g) => ({
      id: `game:${g.id}`,
      group: "Jeux",
      label: g.title,
      hint: g.installed && g.available ? "Entrée : jouer · Maj+Entrée : fiche" : "Entrée : ouvrir la fiche",
      icon: (
        <span className="block h-9 w-6 overflow-hidden rounded-sm">
          <Cover title={g.title} path={g.coverPath} mini />
        </span>
      ),
      run: close(() => (g.installed && g.available ? onPlay(g) : onOpenGame(g.id))),
      alt: close(() => onOpenGame(g.id)),
    }));
    const pages: Item[] = (
      [
        ["home", "Accueil", <Home size={16} />],
        ["library", "Bibliothèque", <Library size={16} />],
        ["backlog", "Backlog", <Kanban size={16} />],
        ["stats", "Statistiques", <BarChart3 size={16} />],
        ["profile", "Profil", <UserRound size={16} />],
        ["settings", "Paramètres", <Settings size={16} />],
      ] as [PaletteTarget, string, ReactNode][]
    ).map(([id, label, icon]) => ({ id: `page:${id}`, group: "Pages", label, icon, run: close(() => onPage(id)) }));
    const actions: Item[] = [{ id: "action:roulette", group: "Actions", label: "Je sais pas à quoi jouer", hint: "Roulette", icon: <Dices size={16} />, run: close(onRoulette) }];
    if (onBigPicture) actions.push({ id: "action:bigpicture", group: "Actions", label: "Mode Big Picture", hint: "F11", icon: <Tv size={16} />, run: close(onBigPicture) });
    const themeItems: Item[] = themes.map((t) => ({ id: `theme:${t.id}`, group: "Thèmes", label: `Thème : ${t.name}`, hint: t.emoji, icon: <Palette size={16} />, run: close(() => setTheme(t.id)) }));
    const others = [...pages, ...actions, ...themeItems]
      .map((it) => ({ it, s: query ? fuzzyScore(query, it.label) : 1 }))
      .filter((x) => x.s > 0)
      .sort((a, b) => b.s - a.s)
      .map((x) => x.it);
    return [...gameItems, ...(query ? others.slice(0, 8) : others.filter((i) => i.group !== "Thèmes"))];
  }, [games, query, themes, onClose, onPlay, onOpenGame, onPage, onRoulette, setTheme]);

  useEffect(() => setIndex(0), [query]);
  useEffect(() => {
    input.current?.focus();
  }, []);
  useEffect(() => {
    list.current?.querySelector(`[data-index="${index}"]`)?.scrollIntoView({ block: "nearest" });
  }, [index]);

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setIndex((i) => Math.min(items.length - 1, i + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setIndex((i) => Math.max(0, i - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const it = items[index];
      if (it) (e.shiftKey && it.alt ? it.alt : it.run)();
    } else if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  };

  let lastGroup = "";
  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center bg-scrim/55 px-4 pt-[12vh] backdrop-blur-[2px]" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div role="dialog" aria-modal="true" aria-label="Recherche rapide" className="w-full max-w-xl overflow-hidden rounded-2xl border border-line bg-panel shadow-2xl" onKeyDown={onKey}>
        <div className="flex items-center gap-3 border-b border-line px-4">
          <Search size={18} className="text-muted" />
          <input
            ref={input}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Rechercher un jeu, une page, un thème…"
            className="h-14 flex-1 bg-transparent text-base outline-none placeholder:text-muted"
            aria-label="Rechercher"
            aria-activedescendant={items[index]?.id}
          />
          <kbd className="rounded border border-line px-1.5 py-0.5 text-[10px] text-muted">Échap</kbd>
        </div>
        <ul ref={list} role="listbox" className="max-h-[55vh] overflow-y-auto p-2">
          {items.length === 0 && <li className="px-3 py-8 text-center text-sm text-muted">Aucun résultat.</li>}
          {items.map((it, i) => {
            const header = it.group !== lastGroup ? it.group : null;
            lastGroup = it.group;
            return (
              <li key={it.id}>
                {header && <p className="px-3 pb-1 pt-2 text-[11px] font-semibold uppercase tracking-wider text-muted">{header}</p>}
                <button
                  id={it.id}
                  data-index={i}
                  role="option"
                  aria-selected={i === index}
                  onMouseMove={() => setIndex(i)}
                  onClick={(e) => (e.shiftKey && it.alt ? it.alt : it.run)()}
                  className={`flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm ${i === index ? "bg-accent/15 text-fg" : "text-fg/90"}`}
                >
                  <span className="flex w-6 shrink-0 justify-center text-muted">{it.icon}</span>
                  <span className="min-w-0 flex-1 truncate">{it.label}</span>
                  {i === index && it.hint && <span className="shrink-0 text-xs text-muted">{it.hint}</span>}
                  {i === index && it.group === "Jeux" && <Play size={14} className="shrink-0 text-accent" />}
                  {i === index && it.group === "Pages" && <SquareArrowOutUpRight size={14} className="shrink-0 text-accent" />}
                </button>
              </li>
            );
          })}
        </ul>
        <div className="flex gap-4 border-t border-line px-4 py-2 text-[11px] text-muted">
          <span>↑↓ naviguer</span>
          <span>Entrée valider</span>
          <span>Maj+Entrée ouvrir la fiche</span>
        </div>
      </div>
    </div>
  );
}
