import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Plus, WifiOff } from "lucide-react";
import { api, imageUrl, on, onFileDrop } from "./api";
import { AchievementsTab } from "./components/Achievements";
import { AddGameDialog } from "./components/AddGameDialog";
import { CollectionActions, CollectionsMenu, MergeBanner } from "./components/Collections";
import { FilterBar } from "./components/FilterBar";
import { GameDetail } from "./components/GameDetail";
import { Library } from "./components/Library";
import { MetadataSettings } from "./components/MetadataSettings";
import { Settings } from "./components/Settings";
import { Sidebar, type Page } from "./components/Sidebar";
import { Stats } from "./components/Stats";
import { ToastProvider, useToasts } from "./components/Toasts";
import { UpdateBanner } from "./components/Updates";
import { Wizard } from "./components/Wizard";
import { Backdrop } from "./themes/Backdrop";
import { ThemeProvider, useTheme } from "./themes/ThemeContext";
import { Button } from "./components/ui";
import { errorMessage, SOURCE_LABELS, t } from "./i18n";
import { DEFAULT_FILTERS, type Filters, type Section, type SortKey } from "./lib/filter";
import type { Collection, Game, MergeSuggestion, ScanState, SyncState } from "./types";

function sectionTitle(section: Section, collections: Collection[]): string {
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
      return collections.find((c) => c.id === section.id)?.name ?? t("nav.collections");
  }
}

function Shell() {
  const toasts = useToasts();
  const [games, setGames] = useState<Game[]>([]);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [suggestions, setSuggestions] = useState<MergeSuggestion[]>([]);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [offline, setOffline] = useState(false);
  const [page, setPage] = useState<Page>("library");
  const [section, setSection] = useState<Section>({ kind: "all" });
  const [filters, setFilters] = useState<Filters>(DEFAULT_FILTERS);
  const [view, setView] = useState<"grid" | "list">("grid");
  const theme = useTheme();
  const [running, setRunning] = useState<Set<string>>(new Set());
  const [openGameId, setOpenGameId] = useState<string | null>(null);
  const [addDialog, setAddDialog] = useState<{ exe?: string } | null>(null);
  const [wizard, setWizard] = useState(false);
  const [version, setVersion] = useState(0);
  const prefsLoaded = useRef(false);

  const reload = useCallback(async () => {
    try {
      const [list, colls, merges] = await Promise.all([api.getGames(), api.getCollections(), api.getMergeSuggestions()]);
      setGames(list);
      setCollections(colls);
      setSuggestions(merges);
      setVersion((v) => v + 1);
    } catch (e) {
      toasts.error(errorMessage(e));
    } finally {
      setLoading(false);
    }
  }, [toasts]);

  // Préférences d'affichage et premier lancement.
  useEffect(() => {
    api
      .getSettings()
      .then((s) => {
        if (s.view === "list" || s.view === "grid") setView(s.view);
        if (s.sort) setFilters((f) => ({ ...f, sort: s.sort as SortKey, sortDesc: s.sort_desc === "1" }));
        if (s.wizard_done !== "1") setWizard(true);
      })
      .catch(() => {})
      .finally(() => {
        prefsLoaded.current = true;
      });
  }, []);

  // Aucun effet visuel pendant une partie (T1-06).
  const { setGameRunning, setDynamicImage } = theme;
  useEffect(() => setGameRunning(running.size > 0), [running, setGameRunning]);
  useEffect(() => {
    if (prefsLoaded.current) void api.setSetting("view", view).catch(() => {});
  }, [view]);
  useEffect(() => {
    if (!prefsLoaded.current) return;
    void api.setSetting("sort", filters.sort).catch(() => {});
    void api.setSetting("sort_desc", filters.sortDesc ? "1" : "0").catch(() => {});
  }, [filters.sort, filters.sortDesc]);

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
      if (s.running) return;
      const errors = s.errors ?? [];
      // Hors ligne : rien d'anormal, la bibliothèque locale reste utilisable.
      setOffline(errors.some((e) => e.code === "OFFLINE"));
      for (const err of errors) {
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

  // E1-01 : clôt la session sans toucher au jeu.
  const stopTracking = useCallback(
    async (game: Game) => {
      try {
        await api.stopTracking(game.id);
        toasts.info(`${game.title} : suivi arrêté, session enregistrée.`);
      } catch (e) {
        toasts.error(`${game.title} : ${errorMessage(e)}`);
      }
    },
    [toasts],
  );

  // E1-02/03 : ferme les processus du jeu après confirmation ; en cas d'échec, propose d'arrêter seulement le suivi.
  const forceClose = useCallback(
    async (game: Game) => {
      if (!window.confirm(`Forcer la fermeture de « ${game.title} » ?\n\nLes processus du jeu seront terminés immédiatement. Toute progression non sauvegardée sera perdue.`)) return;
      try {
        const killed = await api.forceCloseGame(game.id);
        toasts.success(killed > 0 ? `${game.title} : jeu fermé (${killed} processus), session enregistrée.` : `${game.title} : aucun processus à fermer, session enregistrée.`);
      } catch (e) {
        const detail = typeof e === "object" && e !== null && "detail" in e ? String((e as { detail: unknown }).detail) : "";
        const message = `${errorMessage(e)}${detail ? `\nProcessus restants : ${detail}` : ""}`;
        toasts.error(`${game.title} : ${message}`);
        if (window.confirm(`${message}\n\nArrêter quand même le suivi du temps de jeu ? (le jeu n'est pas fermé)`)) await stopTracking(game);
      }
    },
    [toasts, stopTracking],
  );

  const openGame = useMemo(() => games.find((g) => g.id === openGameId) ?? null, [games, openGameId]);
  // Thème dynamique : couleur et fond tirés de la jaquette du jeu affiché.
  const dynamicSource = openGame ? (imageUrl(openGame.coverPath) ?? null) : null;
  useEffect(() => setDynamicImage(dynamicSource), [dynamicSource, setDynamicImage]);
  const closeGame = useCallback(() => setOpenGameId(null), []);

  // À l'ouverture d'une fiche : mise à jour discrète des succès et des visuels du jeu.
  useEffect(() => {
    if (openGameId) void api.refreshGame(openGameId).catch(() => {});
  }, [openGameId]);

  const openCollection = section.kind === "collection" ? collections.find((c) => c.id === section.id) : undefined;

  return (
    <div className="flex h-full">
      <Sidebar
        games={games}
        collections={collections}
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
        status={
          offline ? (
            <p className="flex items-center gap-2 px-2.5 py-1 text-xs text-muted" title="La bibliothèque, le lancement et le suivi des heures fonctionnent sans Internet.">
              <WifiOff size={13} /> Hors ligne
            </p>
          ) : undefined
        }
      />
      <main className="flex min-w-0 flex-1 flex-col">
        <UpdateBanner />
        <div className="min-h-0 flex-1">
        {openGame ? (
          <GameDetail
            key={openGame.id}
            game={openGame}
            isRunning={running.has(openGame.id)}
            onBack={closeGame}
            onPlay={play}
            onStop={stopTracking}
            onForceClose={forceClose}
            achievements={<AchievementsTab game={openGame} />}
            extraSettings={<MetadataSettings game={openGame} />}
            headerExtra={<CollectionsMenu game={openGame} collections={collections} />}
          />
        ) : page === "settings" ? (
          <Settings />
        ) : page === "stats" ? (
          <Stats onOpenGame={setOpenGameId} refreshKey={version} />
        ) : (
          <Library
            games={games}
            loading={loading}
            scanning={scanning || syncing}
            running={running}
            section={section}
            title={sectionTitle(section, collections)}
            filters={filters}
            onFilters={setFilters}
            view={view}
            onView={setView}
            onRefresh={() => void api.refreshLibrary()}
            onOpen={(g) => setOpenGameId(g.id)}
            onPlay={play}
            onStop={stopTracking}
            toolbarExtra={
              <>
                {openCollection && <CollectionActions collection={openCollection} onDeleted={() => setSection({ kind: "all" })} />}
                <FilterBar games={games} filters={filters} onChange={setFilters} />
                <Button onClick={() => setAddDialog({})}>
                  <Plus size={15} /> {t("add.button")}
                </Button>
              </>
            }
            banner={section.kind === "all" ? <MergeBanner games={games} suggestions={suggestions} onChanged={reload} /> : undefined}
          />
        )}
        </div>
      </main>
      {addDialog && <AddGameDialog initialExe={addDialog.exe} onClose={() => setAddDialog(null)} />}
      {wizard && <Wizard onDone={() => setWizard(false)} />}
    </div>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <ThemeProvider>
        <Backdrop />
        <Shell />
      </ThemeProvider>
    </ToastProvider>
  );
}
