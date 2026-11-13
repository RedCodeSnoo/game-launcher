// Ajout de jeux locaux : exécutable choisi à la main, ou scan des dossiers surveillés
// dont chaque candidat doit être validé (F-LOC-01 à F-LOC-04).
import { useCallback, useEffect, useState } from "react";
import { FolderPlus, FolderSearch, Trash2, TriangleAlert } from "lucide-react";
import { api, pickFile, pickFolder } from "../api";
import { errorMessage } from "../i18n";
import type { Candidate, WatchedFolder } from "../types";
import { useToasts } from "./Toasts";
import { Button, Field, Modal, Tabs, TextInput } from "./ui";

type Tab = "exe" | "folders";

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function ExeForm({ initialExe, onDone }: { initialExe?: string; onDone: () => void }) {
  const toasts = useToasts();
  const [exe, setExe] = useState(initialExe ?? "");
  const [title, setTitle] = useState("");
  const [args, setArgs] = useState("");
  const [workDir, setWorkDir] = useState("");
  const [busy, setBusy] = useState(false);

  // Nom proposé : celui de l'exécutable, tant que l'utilisateur n'a rien saisi.
  const suggested = fileName(exe).replace(/\.exe$/i, "");

  const submit = async () => {
    setBusy(true);
    try {
      await api.addLocalGame(exe.trim(), title.trim() || undefined, args.trim() || undefined, workDir.trim() || undefined);
      toasts.success(`${title.trim() || suggested} ajouté à la bibliothèque`);
      onDone();
    } catch (e) {
      toasts.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
    >
      <Field label="Exécutable du jeu (.exe)" hint="Vous pouvez aussi glisser-déposer un .exe sur la fenêtre.">
        <div className="flex gap-2">
          <TextInput value={exe} onChange={(e) => setExe(e.target.value)} placeholder="D:\Jeux\MonJeu\MonJeu.exe" autoFocus />
          <Button type="button" onClick={async () => setExe((await pickFile("Exécutable du jeu", ["exe"])) ?? exe)}>
            Parcourir…
          </Button>
        </div>
      </Field>
      <Field label="Nom affiché">
        <TextInput value={title} onChange={(e) => setTitle(e.target.value)} placeholder={suggested || "Nom du jeu"} />
      </Field>
      <div className="grid grid-cols-2 gap-4">
        <Field label="Arguments de lancement (facultatif)">
          <TextInput value={args} onChange={(e) => setArgs(e.target.value)} placeholder="-windowed" />
        </Field>
        <Field label="Dossier de travail (facultatif)" hint="Par défaut : le dossier de l'exécutable.">
          <TextInput value={workDir} onChange={(e) => setWorkDir(e.target.value)} />
        </Field>
      </div>
      <div className="flex justify-end">
        <Button variant="primary" type="submit" disabled={busy || !exe.trim()}>
          Ajouter le jeu
        </Button>
      </div>
    </form>
  );
}

interface Row extends Candidate {
  checked: boolean;
  chosenExe: string;
  chosenName: string;
}

export function WatchedFoldersPanel({ onAdded }: { onAdded?: () => void }) {
  const toasts = useToasts();
  const [folders, setFolders] = useState<WatchedFolder[]>([]);
  const [rows, setRows] = useState<Row[] | null>(null);
  const [scanning, setScanning] = useState(false);

  const reload = useCallback(() => api.getWatchedFolders().then(setFolders).catch((e) => toasts.error(errorMessage(e))), [toasts]);
  useEffect(() => void reload(), [reload]);

  const addFolder = async () => {
    const path = await pickFolder("Dossier à surveiller");
    if (!path) return;
    try {
      await api.addWatchedFolder(path);
      await reload();
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  const scan = async () => {
    setScanning(true);
    try {
      const found = await api.scanWatchedFolders();
      setRows(found.map((c) => ({ ...c, checked: false, chosenExe: c.exe, chosenName: c.name })));
    } catch (e) {
      toasts.error(errorMessage(e));
    } finally {
      setScanning(false);
    }
  };

  const update = (index: number, patch: Partial<Row>) => setRows((list) => list && list.map((r, i) => (i === index ? { ...r, ...patch } : r)));
  const selected = rows?.filter((r) => r.checked) ?? [];

  const addSelected = async () => {
    try {
      const count = await api.addCandidates(selected.map((r) => ({ name: r.chosenName.trim() || r.name, folder: r.folder, exe: r.chosenExe })));
      toasts.success(`${count} jeu(x) ajouté(s)`);
      setRows((list) => list && list.filter((r) => !r.checked));
      onAdded?.();
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  const ignoreRest = async () => {
    const rest = rows?.filter((r) => !r.checked) ?? [];
    try {
      await api.ignoreCandidates(rest.map((r) => r.exe));
      setRows((list) => list && list.filter((r) => r.checked));
      toasts.info(`${rest.length} candidat(s) ne seront plus proposés`);
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <div>
        <p className="mb-2 text-sm text-muted">Chaque sous-dossier d'un dossier surveillé est examiné à la recherche d'un jeu. Les installeurs et les outils sont écartés.</p>
        <ul className="flex flex-col gap-1">
          {folders.length === 0 && <li className="rounded-md border border-dashed border-line px-3 py-2 text-sm text-muted">Aucun dossier surveillé.</li>}
          {folders.map((f) => (
            <li key={f.id} className="flex items-center gap-2 rounded-md border border-line bg-bg px-3 py-1.5">
              <span className="selectable min-w-0 flex-1 truncate text-sm">{f.path}</span>
              {!f.exists && (
                <span className="flex items-center gap-1 text-xs text-warn">
                  <TriangleAlert size={13} /> introuvable (disque débranché ?)
                </span>
              )}
              <button
                onClick={async () => {
                  await api.removeWatchedFolder(f.id);
                  await reload();
                }}
                className="text-muted hover:text-danger"
                aria-label={`Ne plus surveiller ${f.path}`}
              >
                <Trash2 size={15} />
              </button>
            </li>
          ))}
        </ul>
        <div className="mt-2 flex gap-2">
          <Button onClick={addFolder}>
            <FolderPlus size={15} /> Ajouter un dossier
          </Button>
          <Button variant="primary" onClick={scan} disabled={scanning || folders.length === 0}>
            <FolderSearch size={15} /> {scanning ? "Scan en cours…" : "Scanner"}
          </Button>
        </div>
      </div>

      {rows && (
        <div>
          <div className="mb-2 flex items-center gap-2">
            <h3 className="text-sm font-semibold">Résultats du scan</h3>
            <span className="text-xs text-muted">{rows.length} candidat(s)</span>
            {rows.length > 0 && (
              <button className="ml-auto text-xs text-accent hover:underline" onClick={() => setRows(rows.map((r) => ({ ...r, checked: selected.length !== rows.length })))}>
                {selected.length === rows.length ? "Tout décocher" : "Tout cocher"}
              </button>
            )}
          </div>
          {rows.length === 0 ? (
            <p className="rounded-md border border-dashed border-line px-3 py-4 text-center text-sm text-muted">Aucun nouveau jeu trouvé.</p>
          ) : (
            <ul className="flex max-h-72 flex-col gap-1 overflow-y-auto pr-1">
              {rows.map((r, i) => (
                <li key={r.exe} className={`flex items-center gap-3 rounded-md border px-3 py-2 ${r.checked ? "border-accent bg-accent/10" : "border-line bg-bg"}`}>
                  <input type="checkbox" checked={r.checked} onChange={(e) => update(i, { checked: e.target.checked })} aria-label={`Ajouter ${r.name}`} className="h-4 w-4 accent-[var(--accent)]" />
                  <div className="min-w-0 flex-1">
                    <input
                      value={r.chosenName}
                      onChange={(e) => update(i, { chosenName: e.target.value })}
                      aria-label="Nom du jeu"
                      className="w-full bg-transparent text-sm font-medium outline-none focus:underline"
                    />
                    {r.alternatives.length === 0 ? (
                      <p className="truncate text-xs text-muted" title={r.exe}>
                        {r.exe}
                      </p>
                    ) : (
                      <select
                        value={r.chosenExe}
                        onChange={(e) => update(i, { chosenExe: e.target.value })}
                        aria-label="Exécutable à lancer"
                        className="mt-0.5 w-full rounded border border-line bg-panel px-1 py-0.5 text-xs text-muted"
                      >
                        {[r.exe, ...r.alternatives].map((exe) => (
                          <option key={exe} value={exe}>
                            {r.folder ? exe.slice(r.folder.length + 1) : exe}
                          </option>
                        ))}
                      </select>
                    )}
                  </div>
                  <span className="shrink-0 text-xs tabular-nums text-muted">{(r.size / 1_048_576).toFixed(1)} Mo</span>
                </li>
              ))}
            </ul>
          )}
          {rows.length > 0 && (
            <div className="mt-3 flex justify-end gap-2">
              <Button onClick={ignoreRest} disabled={selected.length === rows.length} title="Les candidats non cochés ne seront plus proposés aux prochains scans">
                Ignorer les non cochés
              </Button>
              <Button variant="primary" onClick={addSelected} disabled={selected.length === 0}>
                Ajouter la sélection ({selected.length})
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function AddGameDialog({ initialExe, onClose }: { initialExe?: string; onClose: () => void }) {
  const [tab, setTab] = useState<Tab>("exe");
  return (
    <Modal title="Ajouter un jeu" onClose={onClose} wide>
      <Tabs
        tabs={[
          { id: "exe", label: "Exécutable" },
          { id: "folders", label: "Dossiers surveillés" },
        ]}
        value={tab}
        onChange={setTab}
      />
      <div className="pt-4">{tab === "exe" ? <ExeForm initialExe={initialExe} onDone={onClose} /> : <WatchedFoldersPanel />}</div>
    </Modal>
  );
}
