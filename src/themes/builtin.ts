// Thèmes intégrés : un fichier JSON par thème dans ./builtin (générés par scripts/make-themes.py).
import { validateTheme, type Theme } from "./schema";

const files = import.meta.glob<{ default: unknown }>("./builtin/*.json", { eager: true });

export const BUILTIN_THEMES: Theme[] = Object.values(files)
  .map((m) => m.default as { order?: number })
  .sort((a, b) => (a.order ?? 99) - (b.order ?? 99))
  .map((raw) => {
    const { theme, errors } = validateTheme(raw);
    if (!theme) throw new Error(`Thème intégré invalide : ${errors.join(" ")}`);
    return theme;
  });

export const DEFAULT_THEME_ID = "dark";

export function builtinTheme(id: string): Theme | undefined {
  return BUILTIN_THEMES.find((t) => t.id === id);
}
