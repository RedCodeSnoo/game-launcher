// Notifications éphémères (retour visuel immédiat, §8.2).
import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { AlertTriangle, CheckCircle2, Info, X } from "lucide-react";

type Kind = "info" | "success" | "error";
interface Toast {
  id: number;
  kind: Kind;
  message: string;
}

interface ToastApi {
  info: (message: string) => void;
  success: (message: string) => void;
  error: (message: string) => void;
}

const ToastContext = createContext<ToastApi>({ info: () => {}, success: () => {}, error: () => {} });

export function useToasts(): ToastApi {
  return useContext(ToastContext);
}

let nextId = 1;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const dismiss = useCallback((id: number) => setToasts((list) => list.filter((t) => t.id !== id)), []);
  const push = useCallback(
    (kind: Kind, message: string) => {
      const id = nextId++;
      setToasts((list) => [...list.slice(-3), { id, kind, message }]);
      setTimeout(() => dismiss(id), kind === "error" ? 8000 : 4000);
    },
    [dismiss],
  );
  const value = useMemo<ToastApi>(
    () => ({ info: (m) => push("info", m), success: (m) => push("success", m), error: (m) => push("error", m) }),
    [push],
  );

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div className="pointer-events-none fixed bottom-4 right-4 z-50 flex w-96 flex-col gap-2" role="status" aria-live="polite">
        {toasts.map((toast) => (
          <div key={toast.id} className="pointer-events-auto flex items-start gap-3 rounded-lg border border-line bg-panel2 p-3 shadow-xl">
            {toast.kind === "error" ? (
              <AlertTriangle size={18} className="mt-0.5 shrink-0 text-danger" />
            ) : toast.kind === "success" ? (
              <CheckCircle2 size={18} className="mt-0.5 shrink-0 text-ok" />
            ) : (
              <Info size={18} className="mt-0.5 shrink-0 text-accent" />
            )}
            <p className="selectable min-w-0 flex-1 break-words text-sm">{toast.message}</p>
            <button onClick={() => dismiss(toast.id)} className="text-muted hover:text-fg" aria-label="Fermer">
              <X size={16} />
            </button>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}
