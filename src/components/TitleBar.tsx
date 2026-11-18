// Barre de titre de la fenêtre, aux couleurs du thème (la fenêtre n'a plus le cadre Windows par défaut).
// Glisser pour déplacer, double-clic pour agrandir ; l'ancrage Windows (Aero Snap) reste disponible.
import { useEffect, useState } from "react";
import { Copy, Minus, Square, X } from "lucide-react";
import { windowControls } from "../api";

export function TitleBar({ subtitle }: { subtitle?: string }) {
  const [maximized, setMaximized] = useState(false);
  useEffect(() => windowControls.onMaximizedChange(setMaximized), []);
  return (
    <header data-tauri-drag-region className="titlebar relative z-40 flex h-8 shrink-0 select-none items-center bg-panel/70">
      {/* Zone de déplacement (le nom de l'application est déjà en haut de la barre latérale). */}
      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center pl-3.5 text-xs text-muted">
        {subtitle && (
          <span data-tauri-drag-region className="truncate">
            {subtitle}
          </span>
        )}
      </div>
      <div className="flex h-full">
        <button onClick={() => void windowControls.minimize()} className="titlebar-btn" aria-label="Réduire" title="Réduire">
          <Minus size={15} />
        </button>
        <button onClick={() => void windowControls.toggleMaximize()} className="titlebar-btn" aria-label={maximized ? "Restaurer" : "Agrandir"} title={maximized ? "Restaurer" : "Agrandir"}>
          {maximized ? <Copy size={13} className="-scale-x-100" /> : <Square size={12} />}
        </button>
        <button onClick={() => void windowControls.close()} className="titlebar-btn titlebar-close" aria-label="Fermer" title="Fermer">
          <X size={16} />
        </button>
      </div>
    </header>
  );
}
