// Jaquette personnalisée et association à une fiche (F-MET-03, F-MET-04), dans les paramètres d'un jeu.
import { useState } from "react";
import { ImagePlus, RotateCcw, Search } from "lucide-react";
import { api, pickFile } from "../api";
import { errorMessage } from "../i18n";
import type { Game, StoreMatch } from "../types";
import { useToasts } from "./Toasts";
import { Button, Modal, TextInput } from "./ui";

function MatchDialog({ game, onClose }: { game: Game; onClose: () => void }) {
  const toasts = useToasts();
  const [term, setTerm] = useState(game.title);
  const [results, setResults] = useState<StoreMatch[] | null>(null);
  const [busy, setBusy] = useState(false);

  const search = async () => {
    setBusy(true);
    try {
      setResults(await api.searchMetadata(term));
    } catch (e) {
      toasts.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const choose = async (m: StoreMatch) => {
    setBusy(true);
    try {
      await api.applyMetadata(game.id, m.appid);
      toasts.success(`Fiche « ${m.name} » associée`);
      onClose();
    } catch (e) {
      toasts.error(errorMessage(e));
      setBusy(false);
    }
  };

  return (
    <Modal title="Associer une fiche" onClose={onClose}>
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void search();
        }}
      >
        <TextInput value={term} onChange={(e) => setTerm(e.target.value)} autoFocus aria-label="Nom du jeu à rechercher" />
        <Button variant="primary" type="submit" disabled={busy || !term.trim()}>
          <Search size={15} /> Rechercher
        </Button>
      </form>
      <p className="mt-2 text-xs text-muted">Choisissez la fiche qui correspond au jeu : la jaquette, la bannière et la description seront récupérées.</p>
      <ul className="mt-3 flex flex-col gap-1">
        {results?.length === 0 && <li className="py-4 text-center text-sm text-muted">Aucun résultat.</li>}
        {results?.map((m) => (
          <li key={m.appid}>
            <button disabled={busy} onClick={() => choose(m)} className="flex w-full items-center gap-3 rounded-md border border-line bg-bg p-2 text-left hover:border-accent disabled:opacity-50">
              {m.image ? <img src={m.image} alt="" className="h-9 w-24 rounded object-cover" /> : <span className="h-9 w-24 rounded bg-panel2" />}
              <span className="min-w-0 flex-1 truncate text-sm font-medium">{m.name}</span>
            </button>
          </li>
        ))}
      </ul>
    </Modal>
  );
}

export function MetadataSettings({ game }: { game: Game }) {
  const toasts = useToasts();
  const [matching, setMatching] = useState(false);

  const run = async (action: () => Promise<unknown>, ok: string) => {
    try {
      await action();
      toasts.success(ok);
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-sm font-semibold">Jaquette et fiche</h3>
      <div className="flex flex-wrap gap-2">
        <Button
          onClick={async () => {
            const path = await pickFile("Image de la jaquette", ["png", "jpg", "jpeg", "webp"]);
            if (path) await run(() => api.setCover(game.id, path), "Jaquette remplacée");
          }}
        >
          <ImagePlus size={15} /> Remplacer la jaquette…
        </Button>
        <Button onClick={() => run(() => api.setCover(game.id), "Jaquette automatique rétablie")}>
          <RotateCcw size={15} /> Jaquette automatique
        </Button>
        <Button onClick={() => setMatching(true)}>
          <Search size={15} /> Associer une fiche…
        </Button>
        <Button onClick={() => run(() => api.refreshGame(game.id, true), "Jeu mis à jour")}>
          <RotateCcw size={15} /> Actualiser ce jeu
        </Button>
      </div>
      {matching && <MatchDialog game={game} onClose={() => setMatching(false)} />}
    </section>
  );
}
