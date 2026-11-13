import { useCallback, useEffect, useState } from "react";
import { api, on } from "./api";
import { Library } from "./components/Library";
import { Sidebar, type Page } from "./components/Sidebar";
import { ToastProvider, useToasts } from "./components/Toasts";
import { errorMessage, SOURCE_LABELS, t } from "./i18n";
import { DEFAULT_FILTERS, type Filters, type Section } from "./lib/filter";
import type { Game, ScanState } from "./types";

export function sectionTitle(section: Section): string {
  switch (section.kind) {
    case "all":
      return t("nav.all");
    case "installed":
      return t("nav.installed");
    case "favorites":
      return t("nav.favorites");
    case "recent":
      return t("nav.recent");
    case "hidden":
      return t("nav.hidden");
    case "source":
      return SOURCE_LABELS[section.source];
    case "collection":
      return t("nav.collections");
  }
}

function Shell() {
  const toasts = useToasts();
  const [games, setGames] = useState<Game[]>([]);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [page, setPage] = useState<Page>("library");
  const [section, setSection] = useState<Section>({ kind: "all" });
  const [filters, setFilters] = useState<Filters>(DEFAULT_FILTERS);
  const [view, setView] = useState<"grid" | "list">("grid");
  const [running] = useState<Set<string>>(new Set());

  const reload = useCallback(async () => {
    try {
      setGames(await api.getGames());
    } catch (e) {
      toasts.error(errorMessage(e));
    } finally {
      setLoading(false);
    }
  }, [toasts]);

  useEffect(() => {
    void reload();
    void api.isScanning().then(setScanning);
    const offLib = on<null>("library-updated", () => void reload());
    const offScan = on<ScanState>("scan-state", (s) => {
      setScanning(s.running);
      for (const r of s.reports ?? []) {
        if (r.error) toasts.error(`${SOURCE_LABELS[r.source]} : ${r.error}`);
      }
    });
    return () => {
      offLib();
      offScan();
    };
  }, [reload, toasts]);

  const play = useCallback(
    async (game: Game) => {
      try {
        await api.launchGame(game.id);
        toasts.info(`${game.title} : lancement…`);
      } catch (e) {
        toasts.error(`${game.title} : ${errorMessage(e)}`);
      }
    },
    [toasts],
  );

  return (
    <div className="flex h-full">
      <Sidebar games={games} page={page} section={section} onSection={setSection} onPage={setPage} />
      <main className="min-w-0 flex-1">
        <Library
          games={games}
          loading={loading}
          scanning={scanning}
          running={running}
          section={section}
          title={sectionTitle(section)}
          filters={filters}
          onFilters={setFilters}
          view={view}
          onView={setView}
          onRefresh={() => void api.refreshLibrary()}
          onOpen={() => {}}
          onPlay={play}
        />
      </main>
    </div>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <Shell />
    </ToastProvider>
  );
}
