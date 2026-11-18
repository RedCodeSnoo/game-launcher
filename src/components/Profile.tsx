// Profil de joueur (B-05) : avatar, pseudo, jeu favori, badges, statistiques clés, export en image.
import { useEffect, useMemo, useState } from "react";
import { Download, Sparkles } from "lucide-react";
import { api, pickSavePath } from "../api";
import { errorMessage } from "../i18n";
import { formatMinutes } from "../lib/format";
import { renderCard } from "../lib/sharecard";
import type { Dashboard, Game, Recap } from "../types";
import { Cover } from "./Cover";
import { useToasts } from "./Toasts";
import { Button } from "./ui";

const AVATARS = ["🎮", "🦊", "🐉", "🦉", "🐺", "🐱", "🤖", "👾", "🧙", "🥷", "🦄", "🐼", "🌸", "🏮", "🍂", "❄️", "🌊", "⚡"];

export function ProfilePage({ games, onOpenGame, onRecap }: { games: Game[]; onOpenGame: (id: string) => void; onRecap: (year: number) => void }) {
  const toasts = useToasts();
  const [prefs, setPrefs] = useState<Record<string, string>>({});
  const [stats, setStats] = useState<Dashboard | null>(null);
  const [recap, setRecap] = useState<Recap | null>(null);
  const year = new Date().getFullYear();
  useEffect(() => {
    void api.getSettings().then(setPrefs).catch(() => {});
    void api.getDashboard("all").then(setStats).catch(() => {});
    void api.getRecap(year).then(setRecap).catch(() => {});
  }, [year]);

  const save = (key: string, value: string) => {
    setPrefs((p) => ({ ...p, [key]: value }));
    void api.setSetting(key, value).catch((e) => toasts.error(errorMessage(e)));
  };
  const name = prefs.profile_name ?? "";
  const avatar = prefs.profile_avatar || "🎮";
  const played = useMemo(() => games.filter((g) => !g.hidden && g.playtimeMinutes > 0).sort((a, b) => b.playtimeMinutes - a.playtimeMinutes), [games]);
  const favorite = games.find((g) => g.id === prefs.profile_favorite) ?? played[0] ?? null;
  const earned = stats?.badges.filter((b) => b.earned) ?? [];
  const profile = recap?.profile;

  const exportImage = async () => {
    if (!stats) return;
    try {
      const png = await renderCard({
        kind: "profile",
        heading: name.trim() || "Joueur",
        subheading: `${avatar}  Profil de joueur`,
        bigValue: `${Math.round(stats.totalMinutes / 60).toLocaleString("fr-FR")} h`,
        bigLabel: `de jeu · ${played.length} jeux joués`,
        stats: [
          { label: "meilleure série", value: `${stats.streaks.best} j 🔥` },
          { label: "succès", value: `${stats.achievements.unlocked}` },
          { label: "complétion", value: `${Math.round(stats.achievements.completion)} %` },
        ],
        games: played.slice(0, 5).map((g) => ({ title: g.title, coverPath: g.coverPath, value: formatMinutes(g.playtimeMinutes) })),
        profile: profile ?? { emoji: avatar, title: "Joueur", description: "" },
        badges: earned.map((b) => b.name),
        footer: "GameLauncher",
      });
      const path = await pickSavePath("Image du profil", `profil-${(name.trim() || "joueur").toLowerCase().replace(/\W+/g, "-")}.png`, ["png"]);
      if (!path) return;
      await api.savePng(path, png);
      toasts.success("Image du profil enregistrée");
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex max-w-5xl flex-col gap-6 px-6 py-6">
        <section className="relative overflow-hidden rounded-2xl border border-line bg-panel p-7">
          <div className="pointer-events-none absolute -right-20 -top-24 h-72 w-72 rounded-full bg-accent/20 blur-3xl" />
          <div className="relative flex flex-wrap items-center gap-6">
            <div className="flex h-28 w-28 items-center justify-center rounded-full bg-panel2 text-6xl ring-4 ring-accent" aria-label="Avatar">
              {avatar}
            </div>
            <div className="min-w-0 flex-1">
              <input
                value={name}
                maxLength={32}
                onChange={(e) => setPrefs((p) => ({ ...p, profile_name: e.target.value }))}
                onBlur={(e) => save("profile_name", e.target.value.trim())}
                placeholder="Votre pseudo"
                className="font-display w-full bg-transparent text-4xl font-bold outline-none placeholder:text-muted/60"
                aria-label="Pseudo"
              />
              {profile && (
                <p className="mt-1 flex flex-wrap items-center gap-2 text-base text-muted">
                  <span className="text-xl">{profile.emoji}</span> <span className="font-semibold text-fg">{profile.title}</span> · {profile.description}
                </p>
              )}
            </div>
            <div className="flex flex-col gap-2">
              <Button variant="primary" onClick={() => void exportImage()} disabled={!stats}>
                <Download size={15} /> Exporter en image
              </Button>
              <Button onClick={() => onRecap(recap?.available ? year : year - 1)}>
                <Sparkles size={15} /> Récap {recap?.available ? year : year - 1}
              </Button>
            </div>
          </div>
          <div className="relative mt-5 flex flex-wrap gap-1.5">
            {AVATARS.map((a) => (
              <button key={a} onClick={() => save("profile_avatar", a)} className={`h-10 w-10 rounded-lg text-xl transition ${a === avatar ? "bg-accent/25 ring-2 ring-accent" : "hover:bg-panel2"}`} aria-label={`Avatar ${a}`}>
                {a}
              </button>
            ))}
          </div>
        </section>

        <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
          {[
            ["Temps de jeu", stats ? formatMinutes(stats.totalMinutes) : "…"],
            ["Jeux joués", String(played.length)],
            ["Meilleure série", stats ? `${stats.streaks.best} jour${stats.streaks.best > 1 ? "s" : ""}` : "…"],
            ["Succès", stats ? `${stats.achievements.unlocked} · ${Math.round(stats.achievements.completion)} %` : "…"],
          ].map(([label, value]) => (
            <div key={label} className="card rounded-xl border border-line bg-panel px-4 py-3.5">
              <p className="text-xs uppercase tracking-wide text-muted">{label}</p>
              <p className="font-display mt-1 text-2xl font-semibold tabular-nums">{value}</p>
            </div>
          ))}
        </div>

        <div className="grid grid-cols-1 gap-5 lg:grid-cols-[18rem_minmax(0,1fr)]">
          <section className="rounded-xl border border-line bg-panel p-5">
            <h2 className="mb-3 text-sm font-semibold">Jeu favori</h2>
            {favorite ? (
              <button onClick={() => onOpenGame(favorite.id)} className="block w-full overflow-hidden rounded-xl ring-1 ring-line hover:ring-accent">
                <div className="aspect-[2/3]">
                  <Cover title={favorite.title} path={favorite.coverPath} />
                </div>
              </button>
            ) : (
              <p className="text-sm text-muted">Jouez un peu pour qu'il apparaisse.</p>
            )}
            <select className="mt-3 h-9 w-full rounded-md border border-line bg-bg px-2 text-sm" value={prefs.profile_favorite ?? ""} onChange={(e) => save("profile_favorite", e.target.value)} aria-label="Jeu favori">
              <option value="">Le plus joué{played[0] ? ` (${played[0].title})` : ""}</option>
              {played.slice(0, 60).map((g) => (
                <option key={g.id} value={g.id}>
                  {g.title}
                </option>
              ))}
            </select>
          </section>
          <section className="rounded-xl border border-line bg-panel p-5">
            <h2 className="mb-3 text-sm font-semibold">Badges</h2>
            <ul className="grid grid-cols-2 gap-2.5 sm:grid-cols-3">
              {(stats?.badges ?? []).map((b) => (
                <li key={b.id} className={`flex flex-col items-center gap-1 rounded-lg border p-3 text-center ${b.earned ? "border-accent/60 bg-accent/10" : "border-line bg-panel2/40 opacity-55 grayscale"}`} title={b.description}>
                  <span className="text-3xl">{b.icon}</span>
                  <span className="text-sm font-semibold">{b.name}</span>
                  <span className="text-[11px] leading-tight text-muted">{b.description}</span>
                </li>
              ))}
            </ul>
            <h2 className="mb-3 mt-6 text-sm font-semibold">Jeux les plus joués</h2>
            <ol className="grid grid-cols-5 gap-3">
              {played.slice(0, 5).map((g, i) => (
                <li key={g.id}>
                  <button onClick={() => onOpenGame(g.id)} className="block w-full text-left">
                    <div className="aspect-[2/3] overflow-hidden rounded-lg ring-1 ring-line hover:ring-accent">
                      <Cover title={g.title} path={g.coverPath} />
                    </div>
                    <p className="mt-1 truncate text-xs font-medium">
                      {i + 1}. {g.title}
                    </p>
                    <p className="text-[11px] text-muted">{formatMinutes(g.playtimeMinutes)}</p>
                  </button>
                </li>
              ))}
            </ol>
          </section>
        </div>
      </div>
    </div>
  );
}
