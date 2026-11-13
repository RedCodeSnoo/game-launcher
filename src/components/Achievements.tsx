// Onglet Succès d'une fiche jeu (F-SUC-01 à F-SUC-05).
import { useEffect, useState } from "react";
import { EyeOff, Lock, Trophy } from "lucide-react";
import { api } from "../api";
import { errorMessage, t } from "../i18n";
import { formatDate, percent } from "../lib/format";
import type { Achievement, Game } from "../types";
import { useToasts } from "./Toasts";
import { ProgressBar } from "./ui";

function Icon({ a }: { a: Achievement }) {
  const [failed, setFailed] = useState(false);
  const url = a.unlocked ? a.iconUnlocked : (a.iconLocked ?? a.iconUnlocked);
  if (url && !failed) {
    return <img src={url} alt="" loading="lazy" draggable={false} onError={() => setFailed(true)} className={`h-12 w-12 rounded ${a.unlocked ? "" : "opacity-60 grayscale"}`} />;
  }
  return (
    <div className={`flex h-12 w-12 items-center justify-center rounded ${a.unlocked ? "bg-accent/25 text-accent" : "bg-panel2 text-muted"}`}>
      {a.unlocked ? <Trophy size={22} /> : <Lock size={20} />}
    </div>
  );
}

export function AchievementsTab({ game }: { game: Game }) {
  const toasts = useToasts();
  const [items, setItems] = useState<Achievement[] | null>(null);
  const [reveal, setReveal] = useState(false);

  // Rechargé quand la synchronisation met à jour les compteurs du jeu.
  useEffect(() => {
    let cancelled = false;
    api
      .getAchievements(game.id)
      .then((list) => !cancelled && setItems(list))
      .catch((e) => toasts.error(errorMessage(e)));
    return () => {
      cancelled = true;
    };
  }, [game.id, game.achTotal, game.achUnlocked, toasts]);

  if (!items) return <div className="skeleton h-24 rounded-lg" />;
  if (items.length === 0) {
    return (
      <div className="flex flex-col items-center gap-2 py-10 text-center">
        <Trophy size={28} className="text-muted" />
        <p className="text-base font-medium">{t("game.noAchievements")}</p>
        <p className="max-w-md text-sm text-muted">
          {game.sources.some((s) => s.source === "steam")
            ? "Ce jeu n'a pas de succès, ou ils n'ont pas encore été synchronisés (compte Steam à configurer dans les paramètres)."
            : "Cette source ne fournit pas de succès."}
        </p>
      </div>
    );
  }

  const unlocked = items.filter((a) => a.unlocked).length;
  const hiddenLocked = items.filter((a) => a.hidden && !a.unlocked).length;
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-4">
        <div className="min-w-0 flex-1">
          <p className="text-sm">
            <span className="font-semibold">
              {unlocked} / {items.length}
            </span>{" "}
            succès débloqués · {percent(unlocked, items.length)} %
          </p>
          <ProgressBar value={percent(unlocked, items.length)} className="mt-1.5" />
        </div>
        {hiddenLocked > 0 && (
          <label className="flex items-center gap-2 text-sm text-muted">
            <input type="checkbox" checked={reveal} onChange={(e) => setReveal(e.target.checked)} className="h-4 w-4 accent-[var(--accent)]" />
            Révéler les {hiddenLocked} succès cachés
          </label>
        )}
      </div>
      <ul className="grid grid-cols-1 gap-2 xl:grid-cols-2">
        {items.map((a) => {
          const masked = a.hidden && !a.unlocked && !reveal;
          return (
            <li key={a.apiName} className={`flex items-center gap-3 rounded-lg border border-line bg-panel p-3 ${a.unlocked ? "" : "opacity-75"}`}>
              {masked ? (
                <div className="flex h-12 w-12 items-center justify-center rounded bg-panel2 text-muted">
                  <EyeOff size={20} />
                </div>
              ) : (
                <Icon a={a} />
              )}
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium">{masked ? "Succès caché" : a.name}</p>
                <p className="line-clamp-2 text-xs text-muted">{masked ? "Les détails seront révélés une fois le succès débloqué." : a.description || " "}</p>
              </div>
              <div className="shrink-0 text-right text-xs text-muted">
                {a.unlocked && <p className="font-medium text-ok">{a.unlockedAt ? formatDate(a.unlockedAt) : "Débloqué"}</p>}
                {a.globalPercent !== null && <p title="Part des joueurs qui ont ce succès">{a.globalPercent.toFixed(1).replace(".", ",")} % des joueurs</p>}
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
