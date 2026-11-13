import { useCallback, useEffect, useMemo, useState } from "react";
import { Plus } from "lucide-react";
import { api, on, onFileDrop } from "./api";
import { AchievementsTab } from "./components/Achievements";
import { AddGameDialog } from "./components/AddGameDialog";
import { GameDetail } from "./components/GameDetail";
import { Library } from "./components/Library";
import { MetadataSettings } from "./components/MetadataSettings";
import { Settings } from "./components/Settings";
import { Sidebar, type Page } from "./components/Sidebar";
import { ToastProvider, useToasts } from "./components/Toasts";
import { Button } from "./components/ui";
import { errorMessage, SOURCE_LABELS, t } from "./i18n";
import { DEFAULT_FILTERS, type Filters, type Section } from "./lib/filter";
import type { Game, ScanState, SyncState } from "./types";

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
  const [running, setRunning] = useState<Set<string>>(new Set());
  const [openGameId, setOpenGameId] = useState<string | null>(null);
  const [addDialog, setAddDialog] = useState<{ exe?: string } | null>(null);
  const [syncing, setSyncing] = useState(false);

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
    void api.getRunning().then((ids) => setRunning(new Set(ids)));
    const offLib = on<null>("library-updated", () => void reload());
    const offRun = on<string[]>("running-changed", (ids) => setRunning(new Set(ids)));
    const offScan = on<ScanState>("scan-state", (s) => {
      setScanning(s.running);
      for (const r of s.reports ?? []) {
        if (r.error) toasts.error(`${SOURCE_LABELS[r.source]} : ${r.error}`);
      }
    });
    const offSync = on<SyncState>("sync-state", (s) => {
      setSyncing(s.running);
      for (const err of s.errors ?? []) {
        // Hors ligne : rien d'anormal, la bibliothèque locale reste utilisable.
        if (err.code !== "OFFLINE") toasts.error(`${SOURCE_LABELS[err.source]} : ${errorMessage(err)}`);
      }
    });
    // Glisser-déposer d'un .exe : ouvre l'ajout manuel pré-rempli.
    const offDrop = onFileDrop((paths) => {
      const exe = paths.find((p) => p.toLowerCase().endsWith(".exe"));
      if (exe) setAddDialog({ exe });
      else if (paths.length > 0) toasts.error(t("err.NOT_AN_EXE"));
    });
    return () => {
      offLib();
      offRun();
      offScan();
      offSync();
      offDrop();
    };
  }, [reload, toasts]);

  const play = useCallback(
    async (game: Game) => {
      try {
        if (!game.installed) {
          await api.installGame(game.id);
          toasts.info(`${game.title} : ouverture du launcher pour l'installation…`);
        } else {
          await api.launchGame(game.id);
          toasts.info(`${game.title} : lancement…`);
        }
      } catch (e) {
        toasts.error(`${game.title} : ${errorMessage(e)}`);
      }
    },
    [toasts],
  );

  const openGame = useMemo(() => games.find((g) => g.id === openGameId) ?? null, [games, openGameId]);
  const closeGame = useCallback(() => setOpenGameId(null), []);

  // À l'ouverture d'une fiche : mise à jour discrète des succès et des visuels du jeu.
  useEffect(() => {
    if (openGameId) void api.refreshGame(openGameId).catch(() => {});
  }, [openGameId]);

  return (
    <div className="flex h-full">
      <Sidebar
        games={games}
        page={page}
        section={section}
        onSection={(s) => {
          setSection(s);
          setOpenGameId(null);
        }}
        onPage={(p) => {
          setPage(p);
          setOpenGameId(null);
        }}
      />
      <main className="min-w-0 flex-1">
        {openGame ? (
          <GameDetail
            key={openGame.id}
            game={openGame}
            isRunning={running.has(openGame.id)}
            onBack={closeGame}
            onPlay={play}
            achievements={<AchievementsTab game={openGame} />}
            extraSettings={<MetadataSettings game={openGame} />}
          />
        ) : page === "settings" ? (
          <Settings />
        ) : (
          <Library
            games={games}
            loading={loading}
            scanning={scanning || syncing}
            running={running}
            section={section}
            title={sectionTitle(section)}
            filters={filters}
            onFilters={setFilters}
            view={view}
            onView={setView}
            onRefresh={() => void api.refreshLibrary()}
            onOpen={(g) => setOpenGameId(g.id)}
            onPlay={play}
            toolbarExtra={
              <Button onClick={() => setAddDialog({})}>
                <Plus size={15} /> {t("add.button")}
              </Button>
            }
          />
        )}
      </main>
      {addDialog && <AddGameDialog initialExe={addDialog.exe} onClose={() => setAddDialog(null)} />}
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
