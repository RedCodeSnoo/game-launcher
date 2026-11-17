// Journal de session (O-04) : à la fin d'une partie, « Une note sur cette session ? » (option).
import { useEffect, useState } from "react";
import { NotebookPen, X } from "lucide-react";
import { api, on } from "../api";
import { formatSeconds } from "../lib/format";
import { Button } from "./ui";

interface Ended {
  sessionId: string;
  gameId: string;
  title: string;
  seconds: number;
}

export function SessionJournal() {
  const [enabled, setEnabled] = useState(false);
  const [ended, setEnded] = useState<Ended | null>(null);
  const [note, setNote] = useState("");
  useEffect(() => {
    const load = () => void api.getSettings().then((s) => setEnabled(s.session_journal === "1")).catch(() => {});
    load();
    const offSettings = on<null>("library-updated", load);
    const offEnded = on<Ended>("session-ended", (e) => {
      setEnded(e);
      setNote("");
    });
    return () => {
      offSettings();
      offEnded();
    };
  }, []);
  if (!enabled || !ended) return null;
  const save = async () => {
    if (note.trim()) await api.setSessionNote(ended.sessionId, note).catch(() => {});
    setEnded(null);
  };
  return (
    <div role="dialog" aria-label="Note de session" className="fixed bottom-5 right-5 z-40 w-96 rounded-xl border border-line bg-panel p-4 shadow-2xl">
      <div className="mb-2 flex items-start gap-2">
        <NotebookPen size={18} className="mt-0.5 shrink-0 text-accent" />
        <div className="min-w-0 flex-1">
          <p className="text-sm font-semibold">Une note sur cette session ?</p>
          <p className="truncate text-xs text-muted">
            {ended.title} · {formatSeconds(ended.seconds)}
          </p>
        </div>
        <button onClick={() => setEnded(null)} className="text-muted hover:text-fg" aria-label="Plus tard">
          <X size={16} />
        </button>
      </div>
      <input
        autoFocus
        value={note}
        maxLength={500}
        onChange={(e) => setNote(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void save();
          if (e.key === "Escape") setEnded(null);
        }}
        placeholder="Boss du chapitre 3 battu…"
        className="h-9 w-full rounded-md border border-line bg-bg px-2.5 text-sm outline-none focus:border-accent"
      />
      <div className="mt-3 flex justify-end gap-2">
        <Button variant="ghost" onClick={() => setEnded(null)}>
          Passer
        </Button>
        <Button variant="primary" onClick={() => void save()} disabled={!note.trim()}>
          Enregistrer
        </Button>
      </div>
    </div>
  );
}
