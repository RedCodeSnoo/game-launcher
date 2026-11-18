// Intégrations (V1.5) : jeux gratuits Epic, actualités Steam, galerie de captures, durée de vie, réglages.
import { useCallback, useEffect, useState } from "react";
import { ChevronLeft, ChevronRight, ExternalLink, FolderOpen, Gift, Images, Newspaper, RefreshCw, X } from "lucide-react";
import { api, imageUrl, on, pickFolder } from "../api";
import { errorMessage } from "../i18n";
import { formatDate, formatDateTime, formatMinutes, formatRelative } from "../lib/format";
import type { FreeGame, Game, NewsItem, Screenshot, SyncInfo } from "../types";
import { useToasts } from "./Toasts";
import { Button, Field, TextInput } from "./ui";

const open = (url: string) => void api.openUrl(url).catch(() => {});

function daysLeft(iso: string): string {
  const ms = new Date(iso).getTime() - Date.now();
  const d = Math.ceil(ms / 864e5);
  return d <= 1 ? "dernier jour" : `encore ${d} jours`;
}

/** Encart de l'accueil : jeux offerts cette semaine sur Epic, et ceux de la semaine suivante. */
export function FreeGamesCard() {
  const [games, setGames] = useState<FreeGame[] | null>(null);
  useEffect(() => {
    const load = () => void api.getFreeGames().then((r) => setGames(r.games)).catch(() => setGames([]));
    load();
    return on<null>("free-games-updated", load);
  }, []);
  if (!games || games.length === 0) return null;
  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h2 className="flex items-center gap-2 text-base font-semibold">
          <Gift size={17} className="text-accent" /> Gratuit sur Epic Games
        </h2>
        <button onClick={() => open("https://store.epicgames.com/fr/free-games")} className="flex items-center gap-1 text-xs text-accent hover:underline">
          Tous les jeux gratuits <ExternalLink size={12} />
        </button>
      </div>
      <div className="grid grid-cols-[repeat(auto-fill,minmax(16rem,1fr))] gap-3">
        {games.slice(0, 4).map((g) => (
          <button key={g.id} onClick={() => open(g.url)} className="group overflow-hidden rounded-xl border border-line bg-panel text-left transition hover:border-accent">
            <div className="relative aspect-[16/9] overflow-hidden bg-panel2">
              {g.image ? (
                <img src={g.image} alt="" loading="lazy" className="h-full w-full object-cover transition duration-500 group-hover:scale-105" />
              ) : (
                <div className="flex h-full items-center justify-center text-accent">
                  <Gift size={36} />
                </div>
              )}
              <span className={`absolute left-2 top-2 rounded-md px-2 py-0.5 text-[11px] font-bold uppercase tracking-wide ${g.current ? "bg-accent text-accent-fg" : "bg-scrim/75 text-on-scrim"}`}>
                {g.current ? "Gratuit maintenant" : "Bientôt gratuit"}
              </span>
            </div>
            <div className="p-3">
              <p className="truncate text-sm font-semibold">{g.title}</p>
              <p className="text-xs text-muted">{g.current ? `Jusqu'au ${formatDate(g.end)} · ${daysLeft(g.end)}` : `À partir du ${formatDate(g.start)}`}</p>
            </div>
          </button>
        ))}
      </div>
    </section>
  );
}

function NewsList({ items, showGame }: { items: NewsItem[]; showGame: boolean }) {
  return (
    <ul className="flex flex-col divide-y divide-line">
      {items.map((n) => (
        <li key={`${n.appid}-${n.id}`}>
          <button onClick={() => n.url && open(n.url)} className="group flex w-full flex-col gap-1 py-3 text-left" disabled={!n.url}>
            <span className="flex flex-wrap items-center gap-x-2 text-xs text-muted">
              {showGame && n.gameTitle && <span className="font-medium text-accent">{n.gameTitle}</span>}
              <span>{formatRelative(n.date)}</span>
              {n.feed && <span>· {n.feed}</span>}
            </span>
            <span className="text-sm font-semibold group-hover:text-accent">{n.title}</span>
            {n.excerpt && <span className="line-clamp-2 text-xs leading-relaxed text-muted">{n.excerpt}</span>}
          </button>
        </li>
      ))}
    </ul>
  );
}

/** Fil d'actualités des jeux Steam de la bibliothèque (accueil). */
export function NewsFeedCard() {
  const [items, setItems] = useState<NewsItem[] | null>(null);
  const [enabled, setEnabled] = useState(true);
  useEffect(() => {
    void api.getSettings().then((s) => {
      const on = s.show_news !== "0";
      setEnabled(on);
      if (on) void api.getNewsFeed().then(setItems).catch(() => setItems([]));
    });
  }, []);
  if (!enabled || !items || items.length === 0) return null;
  return (
    <section className="flex flex-col gap-2">
      <h2 className="flex items-center gap-2 text-base font-semibold">
        <Newspaper size={17} className="text-accent" /> Actualités de vos jeux
      </h2>
      <div className="rounded-xl border border-line bg-panel px-4">
        <NewsList items={items.slice(0, 6)} showGame />
      </div>
    </section>
  );
}

/** Onglet « Actualités » de la fiche d'un jeu Steam. */
export function GameNewsTab({ game }: { game: Game }) {
  const [items, setItems] = useState<NewsItem[] | null>(null);
  useEffect(() => void api.getGameNews(game.id).then(setItems).catch(() => setItems([])), [game.id]);
  if (!items) return <div className="skeleton h-32 rounded-lg" />;
  if (items.length === 0) return <p className="py-8 text-center text-sm text-muted">Pas d'actualité récente (ou pas de connexion).</p>;
  return (
    <div className="max-w-3xl">
      <NewsList items={items} showGame={false} />
    </div>
  );
}

const SOURCE_NAMES: Record<Screenshot["source"], string> = { steam: "Steam", xbox: "Xbox Game Bar", nvidia: "NVIDIA", custom: "Dossier perso" };

/** Onglet « Captures » : galerie, plein écran (flèches, Échap), ouverture du dossier. */
export function ScreenshotsTab({ game }: { game: Game }) {
  const [shots, setShots] = useState<Screenshot[] | null>(null);
  const [index, setIndex] = useState<number | null>(null);
  useEffect(() => void api.getScreenshots(game.id).then(setShots).catch(() => setShots([])), [game.id]);
  const move = useCallback((d: number) => setIndex((i) => (i === null || !shots ? i : (i + d + shots.length) % shots.length)), [shots]);
  useEffect(() => {
    if (index === null) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight") move(1);
      if (e.key === "ArrowLeft") move(-1);
      if (e.key === "Escape") {
        e.stopPropagation();
        setIndex(null);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [index, move]);

  if (!shots) return <div className="skeleton h-32 rounded-lg" />;
  if (shots.length === 0)
    return (
      <p className="py-8 text-center text-sm text-muted">
        Aucune capture trouvée. GameLauncher lit les captures Steam, celles de la Xbox Game Bar (Vidéos\Captures), de NVIDIA (Vidéos\nom du jeu) et les dossiers ajoutés dans Paramètres → Intégrations.
      </p>
    );
  const current = index !== null ? shots[index] : null;
  const folderOf = (p: string) => p.slice(0, Math.max(p.lastIndexOf("\\"), p.lastIndexOf("/")));
  return (
    <>
      <p className="mb-3 flex items-center gap-2 text-sm text-muted">
        <Images size={15} /> {shots.length} capture{shots.length > 1 ? "s" : ""}
      </p>
      <div className="grid grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-3">
        {shots.map((s, i) => (
          <button key={s.path} onClick={() => setIndex(i)} className="group relative aspect-video overflow-hidden rounded-lg bg-panel2 ring-1 ring-line hover:ring-accent">
            <img src={imageUrl(s.path)} alt="" loading="lazy" className="h-full w-full object-cover transition duration-300 group-hover:scale-105" />
            <span className="absolute bottom-1.5 left-1.5 rounded bg-scrim/70 px-1.5 py-0.5 text-[10px] text-on-scrim">{SOURCE_NAMES[s.source]}</span>
          </button>
        ))}
      </div>
      {current && (
        <div className="fixed inset-0 z-50 flex flex-col bg-scrim/95" role="dialog" aria-label="Capture en plein écran">
          <div className="flex items-center gap-3 p-3 text-sm text-on-scrim">
            <span className="flex-1 truncate">
              {index! + 1} / {shots.length} · {SOURCE_NAMES[current.source]} · {formatDateTime(current.takenAt)}
            </span>
            <button onClick={() => void api.openPath(folderOf(current.path))} className="flex items-center gap-1.5 rounded-md bg-on-scrim/10 px-3 py-1.5 hover:bg-on-scrim/20">
              <FolderOpen size={15} /> Ouvrir le dossier
            </button>
            <button onClick={() => setIndex(null)} className="rounded-md p-1.5 hover:bg-on-scrim/10" aria-label="Fermer">
              <X size={20} />
            </button>
          </div>
          <div className="relative flex min-h-0 flex-1 items-center justify-center px-16 pb-6">
            <img src={imageUrl(current.path)} alt="" className="max-h-full max-w-full rounded-md object-contain shadow-2xl" />
            <button onClick={() => move(-1)} className="absolute left-3 rounded-full bg-on-scrim/10 p-2 text-on-scrim hover:bg-on-scrim/20" aria-label="Précédente">
              <ChevronLeft size={26} />
            </button>
            <button onClick={() => move(1)} className="absolute right-3 rounded-full bg-on-scrim/10 p-2 text-on-scrim hover:bg-on-scrim/20" aria-label="Suivante">
              <ChevronRight size={26} />
            </button>
          </div>
        </div>
      )}
    </>
  );
}

/** Durée de vie (IGDB) en une ligne, ou rien. */
export function timeToBeatText(g: Game): string | null {
  const parts = [g.timeToBeatMain ? `histoire ≈ ${formatMinutes(g.timeToBeatMain)}` : null, g.timeToBeatFull ? `100 % ≈ ${formatMinutes(g.timeToBeatFull)}` : null].filter(Boolean);
  return parts.length ? parts.join(" · ") : null;
}

// ---------------------------------------------------------------------------
// Paramètres → Intégrations
// ---------------------------------------------------------------------------

function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button role="switch" aria-checked={checked} aria-label={label} onClick={() => onChange(!checked)} className={`relative h-6 w-11 shrink-0 rounded-full transition ${checked ? "bg-accent" : "bg-line"}`}>
      <span className={`absolute top-0.5 h-5 w-5 rounded-full bg-on-scrim shadow transition-all ${checked ? "left-[1.375rem]" : "left-0.5"}`} />
    </button>
  );
}

function Row({ title, hint, children }: { title: string; hint: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="min-w-0">
        <p className="text-sm font-medium">{title}</p>
        <div className="text-xs text-muted">{hint}</div>
      </div>
      {children}
    </div>
  );
}

export function IntegrationSettings() {
  const toasts = useToasts();
  const [prefs, setPrefs] = useState<Record<string, string>>({});
  const [sync, setSync] = useState<SyncInfo | null>(null);
  const [igdbSet, setIgdbSet] = useState(false);
  const [igdb, setIgdb] = useState({ id: "", secret: "" });
  const [busy, setBusy] = useState(false);
  const load = useCallback(() => {
    void api.getSettings().then(setPrefs).catch(() => {});
    void api.getSyncInfo().then(setSync).catch(() => {});
    void api.getAccounts().then((a) => setIgdbSet(a.igdbSet)).catch(() => {});
  }, []);
  useEffect(load, [load]);
  const set = async (key: string, value: string) => {
    try {
      await api.setSetting(key, value);
      setPrefs((p) => ({ ...p, [key]: value }));
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };
  const folders = (prefs.screenshot_folders ?? "").split("\n").filter((f) => f.trim());

  return (
    <div className="flex flex-col gap-6">
      <Row title="Discord Rich Presence" hint={<>Affiche « Joue à &lt;jeu&gt; » dans votre profil Discord, avec la jaquette (jeux Steam) et le temps écoulé. Discord doit être ouvert sur ce PC.</>}>
        <Toggle checked={prefs.discord_rpc === "1"} onChange={(v) => void set("discord_rpc", v ? "1" : "0")} label="Discord Rich Presence" />
      </Row>
      {prefs.discord_rpc === "1" && (
        <Field label="Identifiant d'application Discord" hint="Créez une application gratuite sur discord.com/developers/applications (son nom s'affiche dans Discord, par exemple « GameLauncher »), puis collez ici son « Application ID ».">
          <div className="flex gap-2">
            <TextInput value={prefs.discord_client_id ?? ""} inputMode="numeric" placeholder="123456789012345678" onChange={(e) => setPrefs((p) => ({ ...p, discord_client_id: e.target.value.replace(/\D/g, "") }))} />
            <Button onClick={() => void set("discord_client_id", prefs.discord_client_id ?? "").then(() => toasts.success("Identifiant enregistré"))}>Enregistrer</Button>
            <Button variant="ghost" onClick={() => open("https://discord.com/developers/applications")}>
              <ExternalLink size={14} />
            </Button>
          </div>
        </Field>
      )}

      <Row title="Jeux gratuits Epic : notification" hint="Une notification Windows quand de nouveaux jeux deviennent gratuits sur Epic Games Store (l'encart de l'accueil reste toujours affiché).">
        <Toggle checked={prefs.epic_free_notify === "1"} onChange={(v) => void set("epic_free_notify", v ? "1" : "0")} label="Notification des jeux gratuits Epic" />
      </Row>
      <Row title="Actualités des jeux Steam" hint="Fil d'actualités sur l'accueil et onglet Actualités dans la fiche des jeux Steam (API officielle de Steam, sans compte).">
        <Toggle checked={prefs.show_news !== "0"} onChange={(v) => void set("show_news", v ? "1" : "0")} label="Actualités Steam" />
      </Row>

      <div className="flex flex-col gap-2 border-t border-line pt-5">
        <p className="text-sm font-medium">Durée de vie des jeux (IGDB)</p>
        <p className="text-xs text-muted">
          Histoire principale et 100 %, affichées dans la fiche et utilisées par la roulette. Source officielle IGDB : il faut un identifiant et un secret d'application Twitch (gratuits, dev.twitch.tv/console). Ils sont rangés dans le coffre Windows.
          {igdbSet && <span className="font-medium text-ok"> Identifiants enregistrés.</span>}
        </p>
        <div className="grid gap-2 sm:grid-cols-[1fr_1fr_auto_auto]">
          <TextInput placeholder="Client ID" value={igdb.id} onChange={(e) => setIgdb((x) => ({ ...x, id: e.target.value }))} aria-label="Client ID Twitch" />
          <TextInput placeholder="Client secret" type="password" value={igdb.secret} onChange={(e) => setIgdb((x) => ({ ...x, secret: e.target.value }))} aria-label="Client secret Twitch" />
          <Button
            disabled={busy || !igdb.id.trim() || !igdb.secret.trim()}
            onClick={async () => {
              setBusy(true);
              try {
                await api.setIgdbCredentials(igdb.id, igdb.secret);
                setIgdb({ id: "", secret: "" });
                setIgdbSet(true);
                toasts.success("Identifiants IGDB vérifiés et enregistrés");
              } catch (e) {
                toasts.error(errorMessage(e));
              } finally {
                setBusy(false);
              }
            }}
          >
            Vérifier et enregistrer
          </Button>
          {igdbSet && (
            <Button variant="ghost" onClick={() => void api.setIgdbCredentials().then(() => setIgdbSet(false))}>
              Retirer
            </Button>
          )}
        </div>
      </div>

      <div className="flex flex-col gap-2 border-t border-line pt-5">
        <p className="text-sm font-medium">Dossiers de captures d'écran</p>
        <p className="text-xs text-muted">Steam, la Xbox Game Bar (Vidéos\Captures) et NVIDIA (Vidéos\nom du jeu) sont lus automatiquement. Ajoutez d'autres dossiers : un sous-dossier au nom du jeu, ou des fichiers dont le nom commence par le titre.</p>
        <ul className="flex flex-col gap-1">
          {folders.map((f) => (
            <li key={f} className="flex items-center gap-2 rounded-md border border-line bg-panel2/50 px-2.5 py-1.5 text-sm">
              <FolderOpen size={14} className="text-muted" />
              <span className="selectable min-w-0 flex-1 truncate">{f}</span>
              <button onClick={() => void set("screenshot_folders", folders.filter((x) => x !== f).join("\n"))} className="text-muted hover:text-danger" aria-label={`Retirer ${f}`}>
                <X size={14} />
              </button>
            </li>
          ))}
        </ul>
        <div>
          <Button
            onClick={async () => {
              const f = await pickFolder("Dossier de captures d'écran");
              if (f && !folders.includes(f)) void set("screenshot_folders", [...folders, f].join("\n"));
            }}
          >
            <FolderOpen size={15} /> Ajouter un dossier…
          </Button>
        </div>
      </div>

      <div className="flex flex-col gap-3 border-t border-line pt-5">
        <p className="text-sm font-medium">Sauvegarde et synchronisation</p>
        <p className="text-xs text-muted">
          Copie de vos heures, sessions, notes, statuts, thèmes perso et réglages dans un dossier de votre choix (OneDrive, Google Drive, clé USB…). Sur un autre PC, choisissez le même dossier puis « Restaurer ». Les clés API ne sont jamais copiées.
        </p>
        {sync?.folder && (
          <p className="selectable text-xs">
            Dossier : <span className="font-medium">{sync.folder}</span>
            {sync.last && <> · dernière synchronisation {formatRelative(sync.last).toLowerCase()} ({formatDateTime(sync.last)})</>}
            {sync.remote && (
              <>
                <br />
                Copie présente : {sync.remote.games} jeux, {sync.remote.sessions} sessions, depuis {sync.remote.machine} le {formatDateTime(sync.remote.exportedAt)}
              </>
            )}
          </p>
        )}
        <div className="flex flex-wrap gap-2">
          <Button
            variant="primary"
            disabled={busy}
            onClick={async () => {
              const folder = sync?.folder ?? (await pickFolder("Dossier de synchronisation"));
              if (!folder) return;
              setBusy(true);
              try {
                const m = await api.syncNow(folder);
                toasts.success(`Synchronisé : ${m.games} jeux, ${m.sessions} sessions`);
                load();
              } catch (e) {
                toasts.error(errorMessage(e));
              } finally {
                setBusy(false);
              }
            }}
          >
            <RefreshCw size={15} className={busy ? "animate-spin" : ""} /> Synchroniser maintenant
          </Button>
          <Button
            onClick={async () => {
              const folder = await pickFolder("Dossier de synchronisation");
              if (!folder) return;
              setBusy(true);
              try {
                await api.syncNow(folder);
                load();
              } catch (e) {
                toasts.error(errorMessage(e));
              } finally {
                setBusy(false);
              }
            }}
          >
            <FolderOpen size={15} /> {sync?.folder ? "Changer de dossier…" : "Choisir le dossier…"}
          </Button>
          <Button
            disabled={busy}
            onClick={async () => {
              const folder = sync?.folder ?? (await pickFolder("Dossier contenant une sauvegarde GameLauncher"));
              if (!folder) return;
              if (!window.confirm("Restaurer depuis ce dossier ? La bibliothèque actuelle (heures, sessions, notes, réglages) sera remplacée par la copie du dossier.")) return;
              try {
                const m = await api.restoreFromFolder(folder);
                toasts.success(`Restauré : ${m.games} jeux, ${m.sessions} sessions`);
                load();
              } catch (e) {
                toasts.error(errorMessage(e));
              }
            }}
          >
            Restaurer depuis le dossier…
          </Button>
        </div>
        <Row title="Synchroniser automatiquement" hint="Au plus toutes les 10 minutes, quand vos données ont changé (fin de session, note, statut…).">
          <Toggle checked={sync?.auto ?? false} onChange={(v) => void set("sync_auto", v ? "1" : "0").then(load)} label="Synchroniser automatiquement" />
        </Row>
      </div>
    </div>
  );
}
