// Mises à jour automatiques : bannière « nouvelle version » et carte des Paramètres.
import { useCallback, useEffect, useState } from "react";
import { Download, RefreshCw, X } from "lucide-react";
import { api, on } from "../api";
import { errorMessage } from "../i18n";
import { formatDateTime } from "../lib/format";
import type { UpdateAvailable, UpdateInfo } from "../types";
import { useToasts } from "./Toasts";
import { Button } from "./ui";

/** État des mises à jour, tenu à jour par les événements du backend. */
export function useUpdateInfo() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const reload = useCallback(() => api.getUpdateInfo().then(setInfo).catch(() => {}), []);
  useEffect(() => {
    void reload();
    const offAvailable = on<UpdateAvailable>("update-available", () => void reload());
    const offState = on<string>("update-state", () => void reload());
    return () => {
      offAvailable();
      offState();
    };
  }, [reload]);
  return { info, setInfo, reload };
}

function stateText(state: string): string | null {
  switch (state) {
    case "waiting-game":
      return "Installation prévue à la fin de la partie en cours.";
    case "downloading":
      return "Téléchargement de la mise à jour…";
    case "installing":
      return "Installation : GameLauncher va redémarrer.";
    case "error":
      return "La mise à jour n'a pas pu être installée. Réessayez plus tard.";
    default:
      return null;
  }
}

export function UpdateBanner() {
  const toasts = useToasts();
  const { info } = useUpdateInfo();
  const [dismissed, setDismissed] = useState<string | null>(null);
  const available = info?.available;
  if (!info || !available || dismissed === available.version) return null;
  const busy = info.state === "waiting-game" || info.state === "downloading" || info.state === "installing";
  const status = stateText(info.state);
  return (
    <div role="region" aria-label="Mise à jour disponible" className="border-b border-accent/40 bg-accent/10 px-5 py-3">
      <div className="flex flex-wrap items-start gap-3">
        <Download size={18} className="mt-0.5 shrink-0 text-accent" />
        <div className="min-w-0 flex-1">
          <p className="text-sm font-semibold">
            GameLauncher {available.version} est disponible <span className="font-normal text-muted">(version actuelle : {info.currentVersion})</span>
          </p>
          {available.notes && <p className="selectable mt-1 max-h-24 overflow-y-auto whitespace-pre-line text-xs text-muted">{available.notes}</p>}
          {status && <p className="mt-1 text-xs font-medium">{status}</p>}
        </div>
        <div className="flex shrink-0 gap-2">
          <Button
            variant="primary"
            disabled={busy}
            onClick={async () => {
              try {
                await api.installUpdate();
              } catch (e) {
                toasts.error(errorMessage(e));
              }
            }}
          >
            Installer et redémarrer
          </Button>
          {!busy && (
            <Button variant="ghost" onClick={() => setDismissed(available.version)} aria-label="Plus tard">
              <X size={15} /> Plus tard
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}

export function UpdateSettings({ toggle }: { toggle: (checked: boolean, onChange: (v: boolean) => void) => React.ReactNode }) {
  const { info, setInfo } = useUpdateInfo();
  const [checking, setChecking] = useState(false);
  const [checked, setChecked] = useState(false);
  if (!info) return <div className="skeleton h-20 rounded-lg" />;
  const result = !checked
    ? null
    : info.lastCheckFailed
      ? "Vérification impossible pour le moment (pas de connexion ?)."
      : info.available
        ? `La version ${info.available.version} est disponible : voir la bannière en haut de la fenêtre.`
        : "GameLauncher est à jour.";
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-sm font-medium">Version actuelle : {info.currentVersion}</p>
          <p className="text-xs text-muted">{info.lastCheck ? `Dernière vérification : ${formatDateTime(info.lastCheck)}` : "Pas encore vérifié depuis le démarrage."}</p>
          {result && <p className="mt-1 text-xs">{result}</p>}
        </div>
        <Button
          disabled={checking}
          onClick={async () => {
            setChecking(true);
            try {
              setInfo(await api.checkForUpdates());
              setChecked(true);
            } finally {
              setChecking(false);
            }
          }}
        >
          <RefreshCw size={15} className={checking ? "animate-spin" : ""} /> Rechercher des mises à jour
        </Button>
      </div>
      <div className="flex items-center justify-between gap-4">
        <div>
          <p className="text-sm font-medium">Vérifier automatiquement</p>
          <p className="text-xs text-muted">Au démarrage puis toutes les 6 heures, même fenêtre fermée. L'installation n'a jamais lieu pendant une partie.</p>
        </div>
        {toggle(info.autoCheck, async (v) => {
          await api.setSetting("auto_update_check", v ? "1" : "0");
          setInfo({ ...info, autoCheck: v });
        })}
      </div>
    </div>
  );
}
