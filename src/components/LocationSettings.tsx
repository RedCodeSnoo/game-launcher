// Emplacement et lancement personnalisés d'une source (E2) : dossier, exécutable principal, méthode de lancement.
import { useState } from "react";
import { RotateCcw } from "lucide-react";
import { api, pickFile, pickFolder } from "../api";
import { errorMessage, SOURCE_LABELS, t } from "../i18n";
import type { GameSourceView } from "../types";
import { useToasts } from "./Toasts";
import { Button, Field, TextInput } from "./ui";

/** Champ « valeur détectée / valeur personnalisée » avec sélecteur et retour à la valeur détectée (E2-02). */
function OverrideField(props: {
  label: string;
  detected: string | null;
  value: string;
  onChange: (v: string) => void;
  onBrowse: () => Promise<string | null>;
  emptyHint: string;
}) {
  const custom = props.value.trim() !== "";
  return (
    <Field label={props.label}>
      <div className="flex gap-2">
        <TextInput value={props.value} onChange={(e) => props.onChange(e.target.value)} placeholder={props.detected ?? props.emptyHint} aria-label={`${props.label} (valeur personnalisée)`} />
        <Button onClick={async () => props.onChange((await props.onBrowse()) ?? props.value)}>Parcourir…</Button>
      </div>
      <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted">
        <span className="selectable min-w-0 break-all">
          Détecté : {props.detected ?? "aucune valeur"}
          {custom ? " · valeur personnalisée utilisée" : props.detected ? " · utilisé" : ""}
        </span>
        {custom && (
          <button onClick={() => props.onChange("")} className="flex items-center gap-1 text-accent hover:underline">
            <RotateCcw size={12} /> Rétablir la valeur détectée
          </button>
        )}
      </div>
    </Field>
  );
}

export function LocationSettings({ source }: { source: GameSourceView }) {
  const toasts = useToasts();
  const isLocal = source.source === "local";
  const [dir, setDir] = useState(source.installPathOverride ?? "");
  const [exe, setExe] = useState(source.executablePathOverride ?? "");
  const [mode, setMode] = useState<"launcher" | "exe">(isLocal || source.launchMode === "exe" ? "exe" : "launcher");
  const [args, setArgs] = useState((isLocal ? source.launchArgs : source.directArgs) ?? "");
  const [workDir, setWorkDir] = useState((isLocal ? source.workingDir : source.directWorkingDir) ?? "");
  const [saving, setSaving] = useState(false);

  const effectiveDir = dir.trim() || source.installPath || undefined;
  const hasExe = exe.trim() !== "" || source.executablePath !== null;

  const save = async () => {
    setSaving(true);
    try {
      const result = await api.updateSource(source.id, {
        installPathOverride: dir.trim() || null,
        executablePathOverride: exe.trim() || null,
        launchMode: isLocal ? undefined : mode,
        launchArgs: args,
        workingDir: workDir,
      });
      toasts.success("Emplacement et lancement enregistrés");
      if (result?.warning === "EXE_OUTSIDE_DIR") toasts.info(t("warn.EXE_OUTSIDE_DIR"));
    } catch (e) {
      toasts.error(errorMessage(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-sm font-semibold">Emplacement et lancement – {SOURCE_LABELS[source.source]}</h3>
      <p className="text-xs text-muted">
        À corriger si la détection s'est trompée. Ces valeurs servent au lancement direct et au suivi du temps de jeu ; elles restent internes à GameLauncher, les fichiers du launcher ne sont jamais modifiés.
      </p>
      <OverrideField label="Dossier d'installation" detected={source.installPath} value={dir} onChange={setDir} onBrowse={() => pickFolder("Dossier d'installation du jeu", effectiveDir)} emptyHint="Aucun dossier détecté" />
      <OverrideField
        label="Exécutable principal"
        detected={source.executablePath}
        value={exe}
        onChange={setExe}
        onBrowse={() => pickFile("Exécutable du jeu", ["exe"], effectiveDir)}
        emptyHint="Aucun exécutable détecté : tout programme du dossier compte comme le jeu"
      />

      {!isLocal && (
        <Field label="Méthode de lancement">
          <div className="flex flex-col gap-2 text-sm">
            <label className="flex items-center gap-2">
              <input type="radio" name={`mode-${source.id}`} checked={mode === "launcher"} onChange={() => setMode("launcher")} />
              Via le launcher d'origine ({SOURCE_LABELS[source.source]})
            </label>
            <label className={`flex items-center gap-2 ${hasExe ? "" : "opacity-50"}`}>
              <input type="radio" name={`mode-${source.id}`} checked={mode === "exe"} disabled={!hasExe} onChange={() => setMode("exe")} />
              Exécutable direct
            </label>
            <p className="text-xs text-muted">
              {hasExe
                ? "Certains jeux exigent leur launcher (DRM, anti-triche, Riot, Battle.net) : lancés directement, ils peuvent refuser de démarrer ou rouvrir le launcher eux-mêmes."
                : "Choisissez d'abord l'exécutable principal pour activer cette option."}
            </p>
          </div>
        </Field>
      )}

      {mode === "exe" && (
        <>
          <Field label="Arguments de lancement">
            <TextInput value={args} onChange={(e) => setArgs(e.target.value)} placeholder="-windowed" />
          </Field>
          <Field label="Dossier de travail" hint="Par défaut : le dossier de l'exécutable.">
            <div className="flex gap-2">
              <TextInput value={workDir} onChange={(e) => setWorkDir(e.target.value)} />
              <Button onClick={async () => setWorkDir((await pickFolder("Dossier de travail", effectiveDir)) ?? workDir)}>Parcourir…</Button>
            </div>
          </Field>
        </>
      )}
      <div>
        <Button variant="primary" disabled={saving} onClick={() => void save()}>
          {t("common.save")}
        </Button>
      </div>
    </section>
  );
}
