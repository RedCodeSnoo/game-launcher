// Page Paramètres : comptes, sources, dossiers surveillés, apparence, démarrage, sauvegarde, journaux (§8.1).
import { useCallback, useEffect, useState, type ReactNode } from "react";
import { CheckCircle2, CircleSlash, Download, FolderOpen, RefreshCw, Upload } from "lucide-react";
import { api, pickFile, pickSavePath } from "../api";
import { errorMessage, SOURCE_LABELS } from "../i18n";
import type { SourceStatus } from "../types";
import { SgdbForm, SteamAccountForm } from "./AccountsSection";
import { WatchedFoldersPanel } from "./AddGameDialog";
import { Appearance } from "./Appearance";
import { IntegrationSettings } from "./Integrations";
import { UpdateSettings } from "./Updates";
import { useToasts } from "./Toasts";
import { Button } from "./ui";

export function SettingsCard({ title, description, children }: { title: string; description?: string; children: ReactNode }) {
  return (
    <section className="rounded-xl border border-line bg-panel p-5">
      <h2 className="text-base font-semibold">{title}</h2>
      {description && <p className="mt-0.5 text-sm text-muted">{description}</p>}
      <div className="mt-4">{children}</div>
    </section>
  );
}

export function Toggle({ checked, onChange, label, disabled }: { checked: boolean; onChange: (v: boolean) => void; label: string; disabled?: boolean }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`relative h-6 w-11 shrink-0 rounded-full transition disabled:opacity-40 ${checked ? "bg-accent" : "bg-line"}`}
    >
      <span className={`absolute top-0.5 h-5 w-5 rounded-full bg-on-scrim shadow transition-all ${checked ? "left-[1.375rem]" : "left-0.5"}`} />
    </button>
  );
}

function capabilityText(s: SourceStatus): string {
  const parts = ["jeux installés"];
  if (s.capabilities.owned) parts.push("non installés");
  parts.push(s.capabilities.playtime ? "heures de la source" : "heures suivies par l'application");
  parts.push(s.capabilities.achievements ? "succès" : "pas de succès");
  return parts.join(" · ");
}

export function SourcesList({ compact }: { compact?: boolean }) {
  const toasts = useToasts();
  const [sources, setSources] = useState<SourceStatus[] | null>(null);
  const load = useCallback(() => api.getSources().then(setSources).catch((e) => toasts.error(errorMessage(e))), [toasts]);
  useEffect(() => void load(), [load]);

  if (!sources) return <div className="skeleton h-32 rounded-lg" />;
  return (
    <ul className="flex flex-col divide-y divide-line">
      {sources.map((s) => (
        <li key={s.source} className="flex items-center gap-3 py-2.5">
          {s.available ? <CheckCircle2 size={18} className="shrink-0 text-ok" /> : <CircleSlash size={18} className="shrink-0 text-muted" />}
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium">
              {SOURCE_LABELS[s.source]}
              <span className="ml-2 text-xs font-normal text-muted">
                {s.source === "local" ? `${s.totalGames} jeu(x)` : s.available ? `détecté · ${s.installedGames} installé(s)${s.totalGames > s.installedGames ? ` · ${s.totalGames} au total` : ""}` : "non détecté sur ce PC"}
              </span>
            </p>
            {!compact && <p className="truncate text-xs text-muted">{capabilityText(s)}</p>}
          </div>
          <Toggle
            checked={s.enabled}
            label={`Activer ${SOURCE_LABELS[s.source]}`}
            onChange={async (enabled) => {
              try {
                await api.setSourceEnabled(s.source, enabled);
                await load();
              } catch (e) {
                toasts.error(errorMessage(e));
              }
            }}
          />
        </li>
      ))}
    </ul>
  );
}

function LogsViewer() {
  const [logs, setLogs] = useState<string | null>(null);
  const load = useCallback(() => api.getLogs().then(setLogs).catch(() => setLogs("")), []);
  return (
    <div className="flex flex-col gap-3">
      <div className="flex gap-2">
        <Button onClick={load}>
          <RefreshCw size={15} /> {logs === null ? "Afficher le journal" : "Recharger"}
        </Button>
        <Button onClick={() => void api.openDataFolder("logs")}>
          <FolderOpen size={15} /> Ouvrir le dossier
        </Button>
      </div>
      {logs !== null && (
        <pre className="selectable max-h-72 overflow-auto whitespace-pre-wrap rounded-md border border-line bg-bg p-3 font-mono text-xs leading-relaxed text-muted">{logs || "Journal vide."}</pre>
      )}
    </div>
  );
}

export function Settings() {
  const toasts = useToasts();
  const [autostart, setAutostart] = useState(false);
  const [prefs, setPrefs] = useState<Record<string, string>>({});
  useEffect(() => void api.getAutostart().then(setAutostart).catch(() => {}), []);
  useEffect(() => void api.getSettings().then(setPrefs).catch(() => {}), []);
  const setPref = async (key: string, value: string) => {
    try {
      await api.setSetting(key, value);
      setPrefs((p) => ({ ...p, [key]: value }));
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };
  /** Ligne d'option avec interrupteur, enregistrée dans les réglages (« 1 » / « 0 »). */
  const option = (key: string, label: string, hint: string, fallback = false, disabled = false) => (
    <div className="flex items-center justify-between gap-4">
      <div>
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-muted">{hint}</p>
      </div>
      <Toggle checked={(prefs[key] ?? (fallback ? "1" : "0")) === "1"} label={label} disabled={disabled} onChange={(v) => void setPref(key, v ? "1" : "0")} />
    </div>
  );

  const run = async (action: () => Promise<unknown>, ok: string) => {
    try {
      await action();
      toasts.success(ok);
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex max-w-5xl flex-col gap-5 px-6 py-5">
        <h1 className="text-lg font-semibold">Paramètres</h1>

        <SettingsCard title="Apparence" description="Survolez un thème pour l'essayer, cliquez pour l'adopter.">
          <Appearance />
        </SettingsCard>

        <SettingsCard title="Compte Steam" description="Affiche les jeux possédés non installés, les heures de jeu et les succès.">
          <SteamAccountForm />
        </SettingsCard>

        <SettingsCard title="Jaquettes – SteamGridDB" description="Les jeux Steam et Epic ont leurs visuels sans clé. SteamGridDB complète les autres sources.">
          <SgdbForm />
        </SettingsCard>

        <SettingsCard title="Sources" description="Une source désactivée n'est plus détectée et ses jeux sont masqués ; ses données sont conservées.">
          <SourcesList />
        </SettingsCard>

        <SettingsCard title="Dossiers surveillés" description="Pour les jeux sans launcher.">
          <WatchedFoldersPanel />
        </SettingsCard>

        <SettingsCard title="Pendant le jeu">
          <div className="flex flex-col gap-4">
          {option("minimize_on_launch", "Fermer la fenêtre au lancement d'un jeu", "GameLauncher reste dans la zone de notification ; le suivi du temps de jeu continue.")}
          {option("session_journal", "Journal de session", "À la fin d'une partie, GameLauncher propose d'ajouter une note (« boss du chapitre 3 battu »), visible dans l'historique.")}
          </div>
        </SettingsCard>

        <SettingsCard title="Suivi du temps de jeu" description="GameLauncher observe la liste des processus toutes les 3 secondes, sans droits administrateur.">
          <div className="flex flex-col gap-4">
            {option("track_external", "Suivre les jeux lancés en dehors de l'application", "Steam, raccourci du Bureau, launcher Riot… Désactivé, seuls les jeux lancés depuis GameLauncher sont suivis.", true)}
            {option("notify_detected", "Notification « Session détectée »", "Affiche une notification Windows quand un jeu lancé en dehors de l'application est reconnu.")}
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-sm font-medium">Durée minimale d'une session (secondes)</p>
                <p className="text-xs text-muted">Les sessions plus courtes (lancement raté, fermeture immédiate) ne sont pas enregistrées.</p>
              </div>
              <input
                type="number"
                min={0}
                max={3600}
                aria-label="Durée minimale d'une session (secondes)"
                className="h-9 w-24 rounded-md border border-line bg-bg px-2 text-sm"
                value={prefs.min_session_secs ?? "60"}
                onChange={(e) => setPrefs((p) => ({ ...p, min_session_secs: e.target.value }))}
                onBlur={(e) => {
                  const n = Math.min(3600, Math.max(0, Math.round(Number(e.target.value) || 0)));
                  void setPref("min_session_secs", String(n));
                }}
              />
            </div>
            <div>
              <p className="text-sm font-medium">Processus à ignorer</p>
              <p className="mb-2 text-xs text-muted">
                Les gestionnaires de plantage, anti-triche, launchers et vues web sont déjà ignorés. Ajoutez ici d'autres noms d'exécutables qui ne doivent pas maintenir une session ouverte (un par ligne, <code>*</code> accepté).
              </p>
              <textarea
                aria-label="Processus à ignorer"
                rows={3}
                placeholder={"MonOutil.exe\n*Helper*.exe"}
                className="w-full rounded-md border border-line bg-bg px-2 py-1.5 font-mono text-xs"
                value={prefs.tracking_exclusions ?? ""}
                onChange={(e) => setPrefs((p) => ({ ...p, tracking_exclusions: e.target.value }))}
                onBlur={(e) => void setPref("tracking_exclusions", e.target.value)}
              />
            </div>
          </div>
        </SettingsCard>

        <SettingsCard title="Arrière-plan et démarrage" description="GameLauncher reste dans la zone de notification (près de l'horloge) pour suivre le temps de jeu.">
          <div className="flex flex-col gap-4">
          {option("close_quits", "La croix ferme complètement l'application", "Par défaut, fermer la fenêtre laisse GameLauncher actif dans la zone de notification. Pour quitter : clic droit sur l'icône, puis Quitter.")}
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="text-sm font-medium">Lancer GameLauncher au démarrage de Windows</p>
              <p className="text-xs text-muted">Ajoute une entrée de démarrage pour votre session uniquement, sans droits administrateur.</p>
            </div>
            <Toggle
              checked={autostart}
              label="Lancer au démarrage de Windows"
              onChange={async (enabled) => {
                try {
                  await api.setAutostart(enabled);
                  setAutostart(enabled);
                } catch (e) {
                  toasts.error(errorMessage(e));
                }
              }}
            />
          </div>
          {option("start_minimized", "Démarrer réduit dans la zone de notification", "S'applique au démarrage avec Windows : la fenêtre ne s'ouvre pas, seule l'icône apparaît.", true, !autostart)}
          </div>
        </SettingsCard>

        <SettingsCard title="Intégrations" description="Discord, jeux gratuits Epic, actualités Steam, durée de vie des jeux, captures d'écran, sauvegarde dans un dossier.">
          <IntegrationSettings />
        </SettingsCard>

        <SettingsCard title="Mises à jour" description="Nouvelles versions publiées sur GitHub, vérifiées et signées avant installation.">
          <UpdateSettings toggle={(checked, onChange) => <Toggle checked={checked} label="Vérifier automatiquement les mises à jour" onChange={onChange} />} />
        </SettingsCard>

        <SettingsCard title="Sauvegarde" description="Heures, sessions, favoris, collections et réglages. Les clés API n'y figurent pas. Une copie automatique est faite chaque jour.">
          <div className="flex flex-wrap gap-2">
            <Button
              onClick={async () => {
                const path = await pickSavePath("Sauvegarde GameLauncher", `gamelauncher-${new Date().toISOString().slice(0, 10)}.db`, ["db"]);
                if (path) await run(() => api.exportBackup(path), "Sauvegarde exportée");
              }}
            >
              <Download size={15} /> Exporter…
            </Button>
            <Button
              onClick={async () => {
                const path = await pickFile("Sauvegarde GameLauncher", ["db"]);
                if (!path) return;
                if (!window.confirm("Restaurer cette sauvegarde ? La bibliothèque actuelle (heures, sessions, collections) sera remplacée.")) return;
                await run(() => api.importBackup(path), "Sauvegarde restaurée");
              }}
            >
              <Upload size={15} /> Restaurer…
            </Button>
            <Button onClick={() => void api.openDataFolder("backups")}>
              <FolderOpen size={15} /> Copies automatiques
            </Button>
          </div>
        </SettingsCard>

        <SettingsCard title="Journaux" description="Pour diagnostiquer une source qui ne détecte plus ses jeux.">
          <LogsViewer />
        </SettingsCard>
      </div>
    </div>
  );
}
