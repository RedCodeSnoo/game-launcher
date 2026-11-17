// Barre latérale : sections de la bibliothèque, sources, collections, pages.
import type { ReactNode } from "react";
import { BarChart3, Clock, EyeOff, FolderHeart, Gamepad2, HardDriveDownload, Heart, Library, Settings as SettingsIcon } from "lucide-react";
import { SOURCE_COLORS, SOURCE_LABELS, t } from "../i18n";
import { listSources, type Section } from "../lib/filter";
import type { Collection, Game, SourceId } from "../types";
import { StreakWidget } from "./StreakWidget";

export type Page = "library" | "stats" | "settings";

interface Props {
  games: Game[];
  collections: Collection[];
  page: Page;
  section: Section;
  onSection: (s: Section) => void;
  onPage: (p: Page) => void;
  status?: ReactNode;
}

const SOURCE_ORDER: SourceId[] = ["steam", "epic", "gog", "battlenet", "riot", "ubisoft", "ea", "xbox", "local"];

export function NavItem({ active, onClick, icon, label, count }: { active: boolean; onClick: () => void; icon: ReactNode; label: string; count?: number }) {
  return (
    <button
      onClick={onClick}
      aria-current={active ? "page" : undefined}
      aria-label={count !== undefined ? `${label} (${count})` : label}
      className={`flex w-full items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm transition ${
        active ? "bg-accent/20 font-medium text-fg" : "text-muted hover:bg-panel2 hover:text-fg"
      }`}
    >
      <span className="flex w-4 shrink-0 justify-center">{icon}</span>
      <span className="min-w-0 flex-1 truncate">{label}</span>
      {count !== undefined && <span className="text-xs tabular-nums text-muted">{count}</span>}
    </button>
  );
}

function Heading({ children }: { children: ReactNode }) {
  return <p className="mt-4 px-2.5 pb-1 text-[11px] font-semibold uppercase tracking-wider text-muted">{children}</p>;
}

export function Sidebar({ games, collections, page, section, onSection, onPage, status }: Props) {
  const visible = games.filter((g) => !g.hidden);
  const hiddenCount = games.length - visible.length;
  const sources = listSources(games);
  const isLib = page === "library";
  const go = (s: Section) => {
    onSection(s);
    onPage("library");
  };
  return (
    <aside className="flex w-56 shrink-0 flex-col border-r border-line bg-panel">
      <div className="flex items-center gap-2 px-4 py-4">
        <Gamepad2 size={22} className="text-accent" />
        <span className="text-base font-semibold">{t("app.name")}</span>
      </div>
      <nav className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto px-2 pb-2" aria-label="Bibliothèque">
        <NavItem active={isLib && section.kind === "all"} onClick={() => go({ kind: "all" })} icon={<Library size={16} />} label={t("nav.all")} count={visible.length} />
        <NavItem
          active={isLib && section.kind === "installed"}
          onClick={() => go({ kind: "installed" })}
          icon={<HardDriveDownload size={16} />}
          label={t("nav.installed")}
          count={visible.filter((g) => g.installed).length}
        />
        <NavItem
          active={isLib && section.kind === "favorites"}
          onClick={() => go({ kind: "favorites" })}
          icon={<Heart size={16} />}
          label={t("nav.favorites")}
          count={visible.filter((g) => g.favorite).length}
        />
        <NavItem active={isLib && section.kind === "recent"} onClick={() => go({ kind: "recent" })} icon={<Clock size={16} />} label={t("nav.recent")} />
        {hiddenCount > 0 && (
          <NavItem active={isLib && section.kind === "hidden"} onClick={() => go({ kind: "hidden" })} icon={<EyeOff size={16} />} label={t("nav.hidden")} count={hiddenCount} />
        )}

        {sources.length > 0 && <Heading>{t("nav.sources")}</Heading>}
        {SOURCE_ORDER.filter((s) => sources.includes(s)).map((s) => (
          <NavItem
            key={s}
            active={isLib && section.kind === "source" && section.source === s}
            onClick={() => go({ kind: "source", source: s })}
            icon={<span className="h-2.5 w-2.5 rounded-full" style={{ background: SOURCE_COLORS[s] }} />}
            label={SOURCE_LABELS[s]}
            count={visible.filter((g) => g.sources.some((x) => x.source === s)).length}
          />
        ))}

        {collections.length > 0 && <Heading>{t("nav.collections")}</Heading>}
        {collections.map((c) => (
          <NavItem
            key={c.id}
            active={isLib && section.kind === "collection" && section.id === c.id}
            onClick={() => go({ kind: "collection", id: c.id })}
            icon={<FolderHeart size={16} />}
            label={c.name}
            count={c.count}
          />
        ))}
      </nav>
      <div className="flex flex-col gap-0.5 border-t border-line p-2">
        <div className="mb-1.5">
          <StreakWidget onOpen={() => onPage("stats")} />
        </div>
        {status}
        <NavItem active={page === "stats"} onClick={() => onPage("stats")} icon={<BarChart3 size={16} />} label={t("nav.stats")} />
        <NavItem active={page === "settings"} onClick={() => onPage("settings")} icon={<SettingsIcon size={16} />} label={t("nav.settings")} />
      </div>
    </aside>
  );
}
