// Thème actif : application des variables CSS, aperçu au survol, thèmes perso, options (animations réduites,
// thème dynamique, thème de saison, sons) et état des effets visuels (en pause quand la fenêtre est masquée,
// en arrière-plan ou qu'un jeu tourne).
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { flushSync } from "react-dom";
import { api } from "../api";
import { BUILTIN_THEMES, DEFAULT_THEME_ID } from "./builtin";
import { dominantColor, fitAccent, readableOn, seasonTheme, themeAttributes, themeToCssVars, validateTheme, type Theme } from "./schema";
import { playUiSound } from "./sounds";

export interface ThemeOptions {
  reduceMotion: boolean;
  dynamic: boolean;
  season: boolean;
  sounds: boolean;
}

interface ThemeContextValue {
  themes: Theme[];
  custom: Theme[];
  /** Thème choisi (sans l'aperçu ni la saison). */
  chosenId: string;
  /** Thème affiché. */
  active: Theme;
  setTheme: (id: string) => void;
  preview: (theme: Theme | string | null) => void;
  saveCustom: (theme: Theme) => Promise<void>;
  deleteCustom: (id: string) => Promise<void>;
  options: ThemeOptions;
  setOption: (key: keyof ThemeOptions, value: boolean) => void;
  /** Les effets (particules, scènes animées) peuvent tourner. */
  fxActive: boolean;
  /** Animations réduites (option ou réglage d'accessibilité de Windows). */
  reduced: boolean;
  setGameRunning: (running: boolean) => void;
  /** Jaquette du jeu affiché, pour le thème dynamique. */
  setDynamicImage: (url: string | null) => void;
  dynamicImage: string | null;
}

const Ctx = createContext<ThemeContextValue | null>(null);
const CACHE_KEY = "gl-theme-cache";

function readCache(): Theme | null {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    return raw ? validateTheme(JSON.parse(raw)).theme : null;
  } catch {
    return null;
  }
}

/** Applique un thème sur <html>. `fade` : transition en fondu (View Transitions, sinon fondu CSS). */
export function applyTheme(t: Theme, fade: boolean) {
  const root = document.documentElement;
  const write = () => {
    for (const [k, v] of Object.entries(themeToCssVars(t))) root.style.setProperty(k, v);
    for (const [k, v] of Object.entries(themeAttributes(t))) root.setAttribute(k, v);
    root.style.colorScheme = t.scheme;
  };
  const doc = document as Document & { startViewTransition?: (cb: () => void) => unknown };
  if (fade && doc.startViewTransition && !root.hasAttribute("data-reduced")) doc.startViewTransition(() => flushSync(write));
  else write();
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ ...t, background: { ...t.background, image: undefined } }));
  } catch {
    // stockage indisponible : le thème sera simplement relu au prochain démarrage
  }
}

// Premier rendu : thème du dernier lancement, sans attendre le backend (pas de flash).
applyTheme(readCache() ?? BUILTIN_THEMES.find((t) => t.id === DEFAULT_THEME_ID)!, false);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [chosenId, setChosenId] = useState(readCache()?.id ?? DEFAULT_THEME_ID);
  const [custom, setCustom] = useState<Theme[]>([]);
  const [previewTheme, setPreviewTheme] = useState<Theme | null>(null);
  const [options, setOptions] = useState<ThemeOptions>({ reduceMotion: false, dynamic: false, season: false, sounds: false });
  const [systemReduced, setSystemReduced] = useState(() => window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false);
  const [visible, setVisible] = useState(() => !document.hidden);
  const [focused, setFocused] = useState(() => document.hasFocus());
  const [gameRunning, setGameRunning] = useState(false);
  const [dynamicImage, setDynamicImage] = useState<string | null>(null);
  const [dynamicAccent, setDynamicAccent] = useState<string | null>(null);
  const loaded = useRef(false);

  useEffect(() => {
    api
      .getSettings()
      .then((s) => {
        const list: Theme[] = [];
        try {
          for (const raw of JSON.parse(s.custom_themes ?? "[]") as unknown[]) {
            const t = validateTheme(raw).theme;
            if (t) list.push({ ...t, custom: true });
          }
        } catch {
          // réglage illisible : ignoré
        }
        setCustom(list);
        // Ancien réglage (« dark » / « light ») ou identifiant de thème.
        if (s.theme) setChosenId(s.theme);
        setOptions({ reduceMotion: s.reduce_motion === "1", dynamic: s.dynamic_theme === "1", season: s.season_theme === "1", sounds: s.ui_sounds === "1" });
      })
      .catch(() => {})
      .finally(() => {
        loaded.current = true;
      });
  }, []);

  // État de la fenêtre et réglage d'accessibilité de Windows (« Afficher les animations »).
  useEffect(() => {
    const mq = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    const onMq = () => setSystemReduced(mq?.matches ?? false);
    const onVis = () => setVisible(!document.hidden);
    const onFocus = () => setFocused(true);
    const onBlur = () => setFocused(false);
    mq?.addEventListener("change", onMq);
    document.addEventListener("visibilitychange", onVis);
    window.addEventListener("focus", onFocus);
    window.addEventListener("blur", onBlur);
    return () => {
      mq?.removeEventListener("change", onMq);
      document.removeEventListener("visibilitychange", onVis);
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("blur", onBlur);
    };
  }, []);

  const themes = useMemo(() => [...BUILTIN_THEMES, ...custom], [custom]);
  const effectiveId = options.season ? seasonTheme(new Date()) : chosenId;
  const base = themes.find((t) => t.id === effectiveId) ?? BUILTIN_THEMES.find((t) => t.id === DEFAULT_THEME_ID)!;
  const shown = previewTheme ?? base;
  const active = useMemo<Theme>(() => {
    if (!dynamicAccent || previewTheme) return shown;
    return { ...shown, palette: { ...shown.palette, accent: dynamicAccent, accentFg: readableOn(dynamicAccent) } };
  }, [shown, dynamicAccent, previewTheme]);
  const reduced = options.reduceMotion || systemReduced;
  const fxActive = !reduced && visible && focused && !gameRunning;

  // Fondu enchaîné seulement pour un vrai changement de thème : jamais pour un aperçu au survol (ni en sortant de
  // l'aperçu), ni quand seul l'accent dynamique change. Sinon chaque survol refaisait un fondu de toute la fenêtre.
  const lastApplied = useRef<{ id: string; preview: boolean } | null>(null);
  useEffect(() => {
    const isPreview = previewTheme !== null;
    const prev = lastApplied.current;
    const fade = prev !== null && !isPreview && !prev.preview && prev.id !== active.id;
    applyTheme(active, fade);
    lastApplied.current = { id: active.id, preview: isPreview };
  }, [active, previewTheme]);

  useEffect(() => {
    const root = document.documentElement;
    root.toggleAttribute("data-reduced", reduced);
    root.toggleAttribute("data-fx-paused", !fxActive);
  }, [reduced, fxActive]);

  // Thème dynamique : couleur dominante de la jaquette du jeu affiché.
  useEffect(() => {
    if (!options.dynamic || !dynamicImage) {
      setDynamicAccent(null);
      return;
    }
    let cancelled = false;
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => {
      try {
        const canvas = document.createElement("canvas");
        canvas.width = canvas.height = 48;
        const c = canvas.getContext("2d", { willReadFrequently: true });
        if (!c) return;
        c.drawImage(img, 0, 0, 48, 48);
        const color = dominantColor(c.getImageData(0, 0, 48, 48).data);
        if (!cancelled) setDynamicAccent(color ? fitAccent(color, base.palette.panel, base.palette.bg) : null);
      } catch {
        if (!cancelled) setDynamicAccent(null); // image d'une autre origine : pas de couleur
      }
    };
    img.onerror = () => !cancelled && setDynamicAccent(null);
    img.src = dynamicImage;
    return () => {
      cancelled = true;
    };
  }, [options.dynamic, dynamicImage, base]);

  // Sons d'interface : un léger clic sur les boutons.
  useEffect(() => {
    if (!options.sounds || active.sounds === "none") return;
    const onClick = (e: MouseEvent) => {
      const el = (e.target as HTMLElement | null)?.closest("button, [role=button], [role=tab]");
      if (el && !(el as HTMLButtonElement).disabled) playUiSound(active.sounds, el.closest("nav, [role=tablist]") ? "nav" : "click");
    };
    document.addEventListener("click", onClick, true);
    return () => document.removeEventListener("click", onClick, true);
  }, [options.sounds, active.sounds]);

  const persistCustom = useCallback(async (list: Theme[]) => {
    setCustom(list);
    await api.setSetting("custom_themes", JSON.stringify(list));
  }, []);

  const value: ThemeContextValue = {
    themes,
    custom,
    chosenId,
    active,
    setTheme: (id) => {
      setPreviewTheme(null);
      setChosenId(id);
      if (loaded.current) void api.setSetting("theme", id).catch(() => {});
      if (options.season) {
        setOptions((o) => ({ ...o, season: false }));
        void api.setSetting("season_theme", "0").catch(() => {});
      }
    },
    preview: (t) => setPreviewTheme(t === null ? null : typeof t === "string" ? (themes.find((x) => x.id === t) ?? null) : t),
    saveCustom: async (t) => persistCustom([...custom.filter((x) => x.id !== t.id), { ...t, custom: true }]),
    deleteCustom: async (id) => {
      await persistCustom(custom.filter((x) => x.id !== id));
      if (chosenId === id) setChosenId(DEFAULT_THEME_ID);
    },
    options,
    setOption: (key, v) => {
      setOptions((o) => ({ ...o, [key]: v }));
      const settingKey = { reduceMotion: "reduce_motion", dynamic: "dynamic_theme", season: "season_theme", sounds: "ui_sounds" }[key];
      void api.setSetting(settingKey, v ? "1" : "0").catch(() => {});
    },
    fxActive,
    reduced,
    setGameRunning,
    setDynamicImage,
    dynamicImage: options.dynamic ? dynamicImage : null,
  };
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useTheme(): ThemeContextValue {
  const v = useContext(Ctx);
  if (!v) throw new Error("useTheme hors de ThemeProvider");
  return v;
}
