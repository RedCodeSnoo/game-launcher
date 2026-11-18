// Profil de joueur (B-05) : photo ajustable (ou emoji), pseudo, top 10 des jeux préférés réordonnable,
// badges, statistiques clés et export en image à montrer à n'importe qui.
import { useEffect, useMemo, useRef, useState } from "react";
import { Camera, Download, GripVertical, ImagePlus, Plus, Search, Smile, Sparkles, Trash2, X } from "lucide-react";
import { api, imageUrl, pickSavePath } from "../api";
import { errorMessage } from "../i18n";
import { startDrag, useDragging, useDrop } from "../lib/drag";
import { formatMinutes } from "../lib/format";
import { searchGames } from "../lib/organize";
import { moveInTop, parseTop, placeInTop, removeFromTop, TOP_SIZE, type Top } from "../lib/profile";
import { renderProfileCard } from "../lib/sharecard";
import type { Dashboard, Game, Recap } from "../types";
import { Cover } from "./Cover";
import { PhotoEditor } from "./PhotoEditor";
import { useToasts } from "./Toasts";
import { Button } from "./ui";

const AVATARS = ["🎮", "🦊", "🐉", "🦉", "🐺", "🐱", "🤖", "👾", "🧙", "🥷", "🦄", "🐼", "🌸", "🏮", "🍂", "❄️", "🌊", "⚡"];

function Avatar({ photo, emoji, size }: { photo: string | null; emoji: string; size: number }) {
  return (
    <span className="flex shrink-0 items-center justify-center overflow-hidden rounded-full bg-panel2 ring-4 ring-accent ring-offset-4 ring-offset-panel" style={{ width: size, height: size, fontSize: size * 0.5 }}>
      {photo ? <img src={photo} alt="Photo de profil" className="h-full w-full object-cover" draggable={false} /> : emoji}
    </span>
  );
}

/** Choix d'un jeu de la bibliothèque pour un emplacement du top. */
function GamePicker({ games, exclude, slot, onPick, onClose }: { games: Game[]; exclude: Set<string>; slot: number; onPick: (id: string) => void; onClose: () => void }) {
  const [q, setQ] = useState("");
  const pool = useMemo(() => games.filter((g) => !g.hidden && !exclude.has(g.id)), [games, exclude]);
  const results = useMemo(() => (q.trim() ? searchGames(pool, q, 24) : [...pool].sort((a, b) => b.playtimeMinutes - a.playtimeMinutes || a.sortTitle.localeCompare(b.sortTitle)).slice(0, 24)), [pool, q]);
  return (
    <div className="fixed inset-0 z-[60] flex items-start justify-center bg-scrim/55 px-4 pt-[10vh] backdrop-blur-sm" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div role="dialog" aria-modal="true" aria-label={`Jeu n° ${slot + 1}`} className="w-full max-w-2xl overflow-hidden rounded-2xl border border-line bg-panel shadow-2xl" onKeyDown={(e) => e.key === "Escape" && onClose()}>
        <div className="flex items-center gap-3 border-b border-line px-4">
          <span className="font-display flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-accent text-sm font-bold text-accent-fg">{slot + 1}</span>
          <Search size={17} className="text-muted" />
          <input autoFocus value={q} onChange={(e) => setQ(e.target.value)} placeholder="Chercher un jeu de votre bibliothèque…" className="h-14 flex-1 bg-transparent text-base outline-none placeholder:text-muted" aria-label="Chercher un jeu" />
          <button onClick={onClose} className="text-muted hover:text-fg" aria-label="Fermer">
            <X size={18} />
          </button>
        </div>
        <div className="grid max-h-[60vh] grid-cols-6 gap-3 overflow-y-auto p-4">
          {results.map((g) => (
            <button key={g.id} onClick={() => onPick(g.id)} className="group text-left" title={g.title}>
              <span className="block aspect-[2/3] overflow-hidden rounded-lg ring-1 ring-line transition group-hover:ring-2 group-hover:ring-accent">
                <Cover title={g.title} path={g.coverPath} />
              </span>
              <span className="mt-1 block truncate text-xs font-medium">{g.title}</span>
            </button>
          ))}
          {results.length === 0 && <p className="col-span-6 py-8 text-center text-sm text-muted">Aucun jeu trouvé.</p>}
        </div>
      </div>
    </div>
  );
}

function TopSlot({ index, game, onAdd, onRemove, onOpen, onDrop }: { index: number; game: Game | null; onAdd: () => void; onRemove: () => void; onOpen: () => void; onDrop: (from: string | undefined, gameId: string) => void }) {
  const ref = useDrop<HTMLDivElement>((p) => onDrop(p.from, p.gameId));
  const dragging = useDragging();
  const isSource = dragging?.from === `top:${index}`;
  const podium = index < 3;
  return (
    <div ref={ref} data-drop="" className={`top-slot group relative rounded-xl transition ${isSource ? "opacity-40" : ""}`}>
      {game ? (
        <button
          onPointerDown={(e) => startDrag(e, { gameId: game.id, title: game.title, cover: imageUrl(game.coverPath), from: `top:${index}` })}
          onClick={onOpen}
          className="relative block aspect-[2/3] w-full cursor-grab overflow-hidden rounded-xl shadow-lg ring-1 ring-line transition hover:-translate-y-1 hover:ring-2 hover:ring-accent active:cursor-grabbing"
          title={`${game.title} – glisser pour changer l'ordre`}
        >
          <Cover title={game.title} path={game.coverPath} />
          <span className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-scrim/90 via-scrim/50 to-transparent px-2.5 pb-2 pt-8 text-left">
            <span className="block truncate text-[13px] font-semibold text-on-scrim">{game.title}</span>
            <span className="block text-[11px] text-on-scrim/75">{game.playtimeMinutes > 0 ? formatMinutes(game.playtimeMinutes) : "Jamais lancé"}</span>
          </span>
          <GripVertical size={16} className="absolute right-1.5 top-2 text-on-scrim/0 drop-shadow transition group-hover:text-on-scrim/90" />
        </button>
      ) : (
        <button onClick={onAdd} className="flex aspect-[2/3] w-full flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed border-line bg-panel2/30 text-muted transition hover:border-accent hover:bg-accent/10 hover:text-accent" aria-label={`Ajouter le jeu n° ${index + 1}`}>
          <span className="flex h-11 w-11 items-center justify-center rounded-full bg-panel2 transition group-hover:bg-accent group-hover:text-accent-fg">
            <Plus size={22} />
          </span>
          <span className="text-xs font-medium">Ajouter</span>
        </button>
      )}
      <span className={`font-display pointer-events-none absolute -left-2 -top-2 flex h-9 w-9 items-center justify-center rounded-full text-sm font-extrabold shadow-lg ${podium ? "bg-accent text-accent-fg" : "border border-line bg-panel text-fg"}`}>{index + 1}</span>
      {game && (
        <button onClick={onRemove} className="absolute -right-2 -top-2 hidden h-7 w-7 items-center justify-center rounded-full border border-line bg-panel text-muted shadow hover:text-danger group-hover:flex" aria-label={`Retirer ${game.title} du top`}>
          <X size={14} />
        </button>
      )}
    </div>
  );
}

export function ProfilePage({ games, onOpenGame, onRecap }: { games: Game[]; onOpenGame: (id: string) => void; onRecap: (year: number) => void }) {
  const toasts = useToasts();
  const [prefs, setPrefs] = useState<Record<string, string> | null>(null);
  const [stats, setStats] = useState<Dashboard | null>(null);
  const [recap, setRecap] = useState<Recap | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const [avatarMenu, setAvatarMenu] = useState(false);
  const [picker, setPicker] = useState<number | null>(null);
  const [exporting, setExporting] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);
  const year = new Date().getFullYear();
  useEffect(() => {
    void api.getSettings().then(setPrefs).catch(() => setPrefs({}));
    void api.getDashboard("all").then(setStats).catch(() => {});
    void api.getRecap(year).then(setRecap).catch(() => {});
  }, [year]);

  const byId = useMemo(() => new Map(games.map((g) => [g.id, g])), [games]);
  const top = useMemo(() => parseTop(prefs?.profile_top10, new Set(byId.keys())), [prefs?.profile_top10, byId]);
  const save = (key: string, value: string) => {
    setPrefs((p) => ({ ...(p ?? {}), [key]: value }));
    void api.setSetting(key, value).catch((e) => toasts.error(errorMessage(e)));
  };
  const saveTop = (t: Top) => save("profile_top10", JSON.stringify(t));

  const name = prefs?.profile_name ?? "";
  const emoji = prefs?.profile_avatar || "🎮";
  const photo = prefs?.profile_photo || null;
  const played = games.filter((g) => !g.hidden && g.playtimeMinutes > 0).length;
  const earned = stats?.badges.filter((b) => b.earned) ?? [];
  const profile = recap?.profile;
  const filled = top.filter(Boolean).length;
  const statTiles: [string, string][] = [
    ["Temps de jeu", stats ? `${Math.round(stats.totalMinutes / 60).toLocaleString("fr-FR")} h` : "…"],
    ["Jeux joués", String(played)],
    ["Meilleure série", stats ? `${stats.streaks.best} j` : "…"],
    ["Succès", stats ? `${stats.achievements.unlocked}` : "…"],
  ];

  const onFile = (f: File | undefined) => {
    if (!f) return;
    if (!f.type.startsWith("image/")) return toasts.error("Choisissez une image (PNG, JPEG, WebP).");
    const reader = new FileReader();
    reader.onload = () => setEditing(String(reader.result));
    reader.readAsDataURL(f);
  };

  const exportImage = async () => {
    setExporting(true);
    try {
      const png = await renderProfileCard({
        name: name.trim() || "Joueur",
        photo,
        emoji,
        title: profile?.title ?? "Joueur",
        description: profile?.description ?? "",
        stats: statTiles.map(([label, value]) => ({ label, value })),
        top: top.map((id) => (id ? byId.get(id) : undefined)).filter((g): g is Game => !!g).map((g) => ({ title: g.title, coverPath: g.coverPath })),
        badges: earned.map((b) => ({ icon: b.icon, name: b.name })),
      });
      const path = await pickSavePath("Image du profil", `profil-${(name.trim() || "joueur").toLowerCase().replace(/\W+/g, "-")}.png`, ["png"]);
      if (path) {
        await api.savePng(path, png);
        toasts.success("Image du profil enregistrée");
      }
    } catch (e) {
      toasts.error(errorMessage(e));
    } finally {
      setExporting(false);
    }
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex max-w-5xl flex-col gap-6 px-6 py-6">
        {/* En-tête : photo, pseudo, profil de joueur. */}
        <section className="relative overflow-hidden rounded-2xl border border-line bg-panel">
          <div className="pointer-events-none absolute inset-x-0 top-0 h-28 bg-gradient-to-r from-accent/45 via-accent/20 to-accent/5" />
          <div className="relative flex flex-wrap items-end gap-6 px-7 pb-6 pt-12">
            <div className="relative">
              <button onClick={() => setAvatarMenu((v) => !v)} className="group relative block rounded-full" aria-label="Changer la photo de profil">
                <Avatar photo={photo} emoji={emoji} size={124} />
                <span className="absolute inset-0 flex items-center justify-center rounded-full bg-scrim/55 text-on-scrim opacity-0 transition group-hover:opacity-100">
                  <Camera size={26} />
                </span>
              </button>
              {avatarMenu && (
                <div className="absolute left-0 top-[8.5rem] z-30 w-72 rounded-xl border border-line bg-panel p-3 shadow-2xl">
                  <button onClick={() => fileInput.current?.click()} className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm hover:bg-panel2">
                    <ImagePlus size={16} className="text-accent" /> Choisir une photo…
                  </button>
                  {photo && (
                    <>
                      <button onClick={() => setEditing(photo)} className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm hover:bg-panel2">
                        <Camera size={16} className="text-accent" /> Réajuster la photo
                      </button>
                      <button onClick={() => (save("profile_photo", ""), setAvatarMenu(false))} className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-danger hover:bg-panel2">
                        <Trash2 size={16} /> Retirer la photo
                      </button>
                    </>
                  )}
                  <p className="mb-1.5 mt-2 flex items-center gap-1.5 px-1 text-xs text-muted">
                    <Smile size={13} /> {photo ? "Ou un emoji (sans photo)" : "Ou un emoji"}
                  </p>
                  <div className="grid grid-cols-6 gap-1">
                    {AVATARS.map((a) => (
                      <button key={a} onClick={() => (save("profile_avatar", a), save("profile_photo", ""), setAvatarMenu(false))} className={`h-9 rounded-md text-lg ${a === emoji && !photo ? "bg-accent/25 ring-1 ring-accent" : "hover:bg-panel2"}`} aria-label={`Avatar ${a}`}>
                        {a}
                      </button>
                    ))}
                  </div>
                </div>
              )}
              <input ref={fileInput} type="file" accept="image/png,image/jpeg,image/webp" className="hidden" onChange={(e) => (onFile(e.target.files?.[0]), (e.target.value = ""), setAvatarMenu(false))} />
            </div>
            <div className="min-w-0 flex-1 pb-1">
              <input
                value={name}
                maxLength={32}
                onChange={(e) => setPrefs((p) => ({ ...(p ?? {}), profile_name: e.target.value }))}
                onBlur={(e) => save("profile_name", e.target.value.trim())}
                placeholder="Votre pseudo"
                className="font-display w-full bg-transparent text-4xl font-bold outline-none placeholder:text-muted/60"
                aria-label="Pseudo"
              />
              {profile && (
                <p className="mt-1.5 flex flex-wrap items-center gap-2 text-sm text-muted">
                  <span className="rounded-full bg-accent/15 px-2.5 py-0.5 font-semibold text-accent">
                    {profile.emoji} {profile.title}
                  </span>
                  {profile.description}
                </p>
              )}
            </div>
            <div className="flex flex-col gap-2 pb-1">
              <Button variant="primary" onClick={() => void exportImage()} disabled={!stats || exporting}>
                <Download size={15} /> {exporting ? "Création…" : "Exporter en image"}
              </Button>
              <Button onClick={() => onRecap(recap?.available ? year : year - 1)}>
                <Sparkles size={15} /> Récap {recap?.available ? year : year - 1}
              </Button>
            </div>
          </div>
          <div className="relative grid grid-cols-2 border-t border-line lg:grid-cols-4">
            {statTiles.map(([label, value], i) => (
              <div key={label} className={`px-6 py-4 ${i > 0 ? "lg:border-l lg:border-line" : ""}`}>
                <p className="text-[11px] font-semibold uppercase tracking-wider text-muted">{label}</p>
                <p className="font-display mt-0.5 text-2xl font-bold tabular-nums">{value}</p>
              </div>
            ))}
          </div>
        </section>

        {/* Top 10 des jeux préférés. */}
        <section className="rounded-2xl border border-line bg-panel p-6">
          <div className="mb-5 flex flex-wrap items-baseline gap-3">
            <h2 className="font-display text-xl font-bold">Mes 10 jeux préférés</h2>
            <p className="text-sm text-muted">{filled === 0 ? "Cliquez sur + pour ajouter vos jeux, puis glissez-les pour les classer." : `${filled}/${TOP_SIZE} · glissez une jaquette pour changer l'ordre`}</p>
          </div>
          <div className="grid grid-cols-5 gap-x-5 gap-y-6 pl-2 pt-2">
            {top.map((id, i) => (
              <TopSlot
                key={i}
                index={i}
                game={id ? (byId.get(id) ?? null) : null}
                onAdd={() => setPicker(i)}
                onRemove={() => saveTop(removeFromTop(top, i))}
                onOpen={() => id && onOpenGame(id)}
                onDrop={(from, gameId) => {
                  const f = from?.startsWith("top:") ? Number(from.slice(4)) : -1;
                  saveTop(f >= 0 ? moveInTop(top, f, i) : placeInTop(top, gameId, i));
                }}
              />
            ))}
          </div>
        </section>

        {/* Badges. */}
        <section className="rounded-2xl border border-line bg-panel p-6">
          <h2 className="font-display mb-4 text-xl font-bold">
            Badges <span className="text-sm font-normal text-muted">{earned.length}/{stats?.badges.length ?? 0}</span>
          </h2>
          <ul className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            {(stats?.badges ?? []).map((b) => (
              <li key={b.id} className={`flex items-center gap-3 rounded-xl border p-3.5 ${b.earned ? "border-accent/50 bg-accent/10" : "border-line bg-panel2/30 opacity-50 grayscale"}`} title={b.description}>
                <span className="text-3xl">{b.icon}</span>
                <span className="min-w-0">
                  <span className="block text-sm font-semibold">{b.name}</span>
                  <span className="block text-xs leading-snug text-muted">{b.description}</span>
                </span>
              </li>
            ))}
          </ul>
        </section>
      </div>

      {editing && (
        <PhotoEditor
          src={editing}
          onCancel={() => setEditing(null)}
          onSave={(url) => {
            save("profile_photo", url);
            setEditing(null);
          }}
        />
      )}
      {picker !== null && (
        <GamePicker
          games={games}
          exclude={new Set(top.filter((v): v is string => !!v))}
          slot={picker}
          onClose={() => setPicker(null)}
          onPick={(id) => {
            saveTop(placeInTop(top, id, picker));
            setPicker(null);
          }}
        />
      )}
    </div>
  );
}
