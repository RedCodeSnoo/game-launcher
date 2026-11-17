// Schéma d'un thème (fichier JSON), validation et conversion en variables CSS. Logique pure, testée.

export const PARTICLE_KINDS = ["none", "petals", "leaves", "snow", "fireflies", "bubbles", "pixelstars"] as const;
export const SCENES = ["none", "aurora", "sakura", "kyoto", "autumn", "winter", "ocean", "neon", "retro"] as const;
export const PATTERNS = ["none", "seigaiha", "paper", "frost", "scanlines", "pixel", "asanoha"] as const;
export const ICON_STYLES = ["line", "pixel", "glow"] as const;
export const SOUND_SETS = ["none", "soft", "8bit"] as const;
export const BORDER_STYLES = ["solid", "none", "glow", "pixel", "frost"] as const;

export type ParticleKind = (typeof PARTICLE_KINDS)[number];
export type Scene = (typeof SCENES)[number];
export type Pattern = (typeof PATTERNS)[number];

export interface Palette {
  bg: string;
  panel: string;
  panel2: string;
  line: string;
  fg: string;
  muted: string;
  accent: string;
  accentFg: string;
  ok: string;
  warn: string;
  danger: string;
  /** Voile posé sur les images (bannières, jaquettes) et texte lisible dessus. */
  scrim: string;
  onScrim: string;
}

export interface Theme {
  id: string;
  name: string;
  description: string;
  /** Emoji ou court symbole affiché dans la galerie. */
  emoji?: string;
  scheme: "dark" | "light";
  palette: Palette;
  /** `scale` agrandit tout le texte (polices pixel, petites par nature). */
  fonts: { body: string; display: string; mono?: string; scale: number };
  card: {
    radius: number;
    border: (typeof BORDER_STYLES)[number];
    glass: boolean;
    blur: number;
    shadow: "none" | "soft" | "deep" | "glow" | "hard";
    pattern: Pattern;
  };
  background: {
    /** Fond de la fenêtre, sous la scène (dégradés CSS acceptés). */
    gradient?: string;
    scene: Scene;
    /** Image personnelle (data URL), posée sous la scène. */
    image?: string;
    /** Flou appliqué à l'image personnelle (px). */
    imageBlur?: number;
  };
  particles: { kind: ParticleKind; density: number };
  icons: { stroke: number; style: (typeof ICON_STYLES)[number] };
  sounds: (typeof SOUND_SETS)[number];
  /** Durée de base des transitions (ms). */
  motion: number;
  /** Thème créé par l'utilisateur (éditeur ou import). */
  custom?: boolean;
  /** Thème d'origine d'un thème personnel. */
  base?: string;
}

export interface ValidationResult {
  theme: Theme | null;
  errors: string[];
}

const HEX = /^#([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const PALETTE_KEYS: (keyof Palette)[] = ["bg", "panel", "panel2", "line", "fg", "muted", "accent", "accentFg", "ok", "warn", "danger", "scrim", "onScrim"];
/** Polices embarquées (licence OFL) ; une autre police tombe sur la police système. */
export const FONTS = ["Inter", "Noto Sans JP", "Noto Serif JP", "Fraunces", "Orbitron", "Rajdhani", "Press Start 2P", "VT323", "system-ui"] as const;
const MAX_IMAGE_CHARS = 6_000_000;

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/** Valide un objet (JSON importé, thème perso) et le complète par des valeurs par défaut sûres. */
export function validateTheme(input: unknown): ValidationResult {
  const errors: string[] = [];
  if (!isObject(input)) return { theme: null, errors: ["Le fichier ne contient pas un objet JSON."] };
  const str = (v: unknown, field: string, max = 80): string => {
    if (typeof v !== "string" || v.trim() === "") {
      errors.push(`« ${field} » est obligatoire.`);
      return "";
    }
    if (v.length > max) errors.push(`« ${field} » est trop long.`);
    return v.trim();
  };
  const oneOf = <T extends string>(v: unknown, allowed: readonly T[], field: string, fallback: T): T => {
    if (v === undefined) return fallback;
    if (typeof v === "string" && (allowed as readonly string[]).includes(v)) return v as T;
    errors.push(`« ${field} » doit valoir ${allowed.join(", ")}.`);
    return fallback;
  };
  const num = (v: unknown, field: string, min: number, max: number, fallback: number): number => {
    if (v === undefined) return fallback;
    if (typeof v !== "number" || !Number.isFinite(v) || v < min || v > max) {
      errors.push(`« ${field} » doit être un nombre entre ${min} et ${max}.`);
      return fallback;
    }
    return v;
  };

  const id = str(input.id, "id", 40);
  if (id && !/^[a-z0-9][a-z0-9-]*$/.test(id)) errors.push("« id » ne doit contenir que des minuscules, chiffres et tirets.");
  const name = str(input.name, "name", 40);
  const description = typeof input.description === "string" ? input.description.slice(0, 200) : "";

  const palette = {} as Palette;
  if (!isObject(input.palette)) errors.push("« palette » est obligatoire.");
  for (const key of PALETTE_KEYS) {
    const v = isObject(input.palette) ? input.palette[key] : undefined;
    if (typeof v !== "string" || !HEX.test(v)) errors.push(`« palette.${key} » doit être une couleur #rrggbb.`);
    else palette[key] = v;
  }

  const fonts = isObject(input.fonts) ? input.fonts : {};
  const font = (v: unknown, fallback: string) => (typeof v === "string" && v.length <= 40 && /^[\w\s-]+$/.test(v) ? v : fallback);
  const card = isObject(input.card) ? input.card : {};
  const background = isObject(input.background) ? input.background : {};
  const particles = isObject(input.particles) ? input.particles : {};
  const icons = isObject(input.icons) ? input.icons : {};

  let gradient: string | undefined;
  if (background.gradient !== undefined) {
    // Uniquement des dégradés et des couleurs : pas d'url() ni d'expression arbitraire dans un fichier importé.
    if (typeof background.gradient === "string" && background.gradient.length <= 600 && /^[\w\s#%,.()-]+$/.test(background.gradient) && !/url\s*\(/i.test(background.gradient)) {
      gradient = background.gradient;
    } else errors.push("« background.gradient » doit être un dégradé CSS simple.");
  }
  let image: string | undefined;
  if (background.image !== undefined) {
    if (typeof background.image === "string" && /^data:image\/(png|jpeg|webp);base64,[A-Za-z0-9+/=]+$/.test(background.image) && background.image.length <= MAX_IMAGE_CHARS) {
      image = background.image;
    } else errors.push("« background.image » doit être une image PNG, JPEG ou WebP intégrée (4 Mo au plus).");
  }

  const theme: Theme = {
    id,
    name,
    description,
    emoji: typeof input.emoji === "string" ? input.emoji.slice(0, 4) : undefined,
    scheme: oneOf(input.scheme, ["dark", "light"] as const, "scheme", "dark"),
    palette,
    fonts: { body: font(fonts.body, "Inter"), display: font(fonts.display, font(fonts.body, "Inter")), mono: typeof fonts.mono === "string" ? font(fonts.mono, "") || undefined : undefined, scale: num(fonts.scale, "fonts.scale", 0.8, 1.5, 1) },
    card: {
      radius: num(card.radius, "card.radius", 0, 28, 12),
      border: oneOf(card.border, BORDER_STYLES, "card.border", "solid"),
      glass: card.glass === true,
      blur: num(card.blur, "card.blur", 0, 40, 0),
      shadow: oneOf(card.shadow, ["none", "soft", "deep", "glow", "hard"] as const, "card.shadow", "soft"),
      pattern: oneOf(card.pattern, PATTERNS, "card.pattern", "none"),
    },
    background: {
      gradient,
      scene: oneOf(background.scene, SCENES, "background.scene", "none"),
      image,
      imageBlur: num(background.imageBlur, "background.imageBlur", 0, 40, 0),
    },
    particles: { kind: oneOf(particles.kind, PARTICLE_KINDS, "particles.kind", "none"), density: num(particles.density, "particles.density", 0, 3, 1) },
    icons: { stroke: num(icons.stroke, "icons.stroke", 1, 3, 1.75), style: oneOf(icons.style, ICON_STYLES, "icons.style", "line") },
    sounds: oneOf(input.sounds, SOUND_SETS, "sounds", "soft"),
    motion: num(input.motion, "motion", 0, 600, 160),
    custom: input.custom === true,
    base: typeof input.base === "string" ? input.base.slice(0, 40) : undefined,
  };
  return errors.length ? { theme: null, errors } : { theme, errors };
}

// ---------------------------------------------------------------------------
// Couleurs : contraste (WCAG) et utilitaires
// ---------------------------------------------------------------------------

export type Rgba = [number, number, number, number];

export function parseHex(hex: string): Rgba {
  let h = hex.replace("#", "");
  if (h.length === 3 || h.length === 4) h = [...h].map((c) => c + c).join("");
  const n = (i: number) => parseInt(h.slice(i, i + 2), 16);
  return [n(0), n(2), n(4), h.length === 8 ? n(6) / 255 : 1];
}

export function toHex([r, g, b]: Rgba | [number, number, number]): string {
  return "#" + [r, g, b].map((v) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, "0")).join("");
}

/** Couleur semi-transparente posée sur un fond opaque. */
export function composite(top: Rgba, under: Rgba): Rgba {
  const a = top[3];
  return [top[0] * a + under[0] * (1 - a), top[1] * a + under[1] * (1 - a), top[2] * a + under[2] * (1 - a), 1];
}

function luminance([r, g, b]: Rgba): number {
  const c = (v: number) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * c(r) + 0.7152 * c(g) + 0.0722 * c(b);
}

/** Rapport de contraste WCAG entre deux couleurs (la seconde sert de fond si la première est transparente). */
export function contrast(fg: string, bg: string, page?: string): number {
  const base = page ? parseHex(page) : ([0, 0, 0, 1] as Rgba);
  const b = composite(parseHex(bg), base);
  const f = composite(parseHex(fg), b);
  const [l1, l2] = [luminance(f), luminance(b)].sort((x, y) => y - x);
  return (l1 + 0.05) / (l2 + 0.05);
}

/** Texte lisible (noir ou blanc) sur une couleur. */
export function readableOn(hex: string): string {
  return contrast("#ffffff", hex) >= contrast("#111111", hex) ? "#ffffff" : "#111111";
}

// ---------------------------------------------------------------------------
// Variables CSS
// ---------------------------------------------------------------------------

const SHADOWS: Record<Theme["card"]["shadow"], string> = {
  none: "none",
  soft: "0 1px 2px rgb(0 0 0 / 0.12), 0 6px 18px rgb(0 0 0 / 0.12)",
  deep: "0 2px 4px rgb(0 0 0 / 0.25), 0 14px 32px rgb(0 0 0 / 0.35)",
  glow: "0 0 0 1px color-mix(in srgb, var(--accent) 45%, transparent), 0 0 18px color-mix(in srgb, var(--accent) 35%, transparent)",
  hard: "4px 4px 0 0 color-mix(in srgb, var(--accent) 70%, black)",
};

function fontStack(name: string, generic: string): string {
  return name === "system-ui" ? `"Segoe UI Variable", "Segoe UI", system-ui, ${generic}` : `"${name}", "Segoe UI", system-ui, ${generic}`;
}

/** Variables CSS d'un thème (appliquées sur :root). */
export function themeToCssVars(t: Theme): Record<string, string> {
  const p = t.palette;
  const vars: Record<string, string> = {
    "--bg": p.bg,
    "--panel": p.panel,
    "--panel2": p.panel2,
    "--line": p.line,
    "--fg": p.fg,
    "--muted": p.muted,
    "--accent": p.accent,
    "--accent-fg": p.accentFg,
    "--ok": p.ok,
    "--warn": p.warn,
    "--danger": p.danger,
    "--scrim": p.scrim,
    "--on-scrim": p.onScrim,
    "--t-font-body": fontStack(t.fonts.body, "sans-serif"),
    "--t-font-display": fontStack(t.fonts.display, t.fonts.display.includes("Serif") || t.fonts.display === "Fraunces" ? "serif" : "sans-serif"),
    "--t-font-mono": t.fonts.mono ? fontStack(t.fonts.mono, "monospace") : "ui-monospace, Consolas, monospace",
    "--t-font-scale": String(t.fonts.scale),
    // Les polices d'affichage très larges (pixel, Orbitron) sont ramenées à une taille comparable.
    "--t-display-adjust": t.fonts.display === "Press Start 2P" ? "0.36" : t.fonts.display === "Orbitron" ? "0.47" : "none",
    "--t-display-tracking": t.fonts.display === "Orbitron" ? "0.04em" : t.fonts.display === "Fraunces" || t.fonts.display.includes("Serif") ? "0.01em" : "normal",
    "--t-radius-card": `${t.card.radius}px`,
    "--t-radius-ctl": `${Math.round(t.card.radius * 0.55)}px`,
    "--t-blur": `${t.card.glass ? t.card.blur : 0}px`,
    "--t-card-shadow": SHADOWS[t.card.shadow],
    "--t-dur": `${t.motion}ms`,
    "--t-icon-stroke": String(t.icons.stroke),
    "--t-app-bg": t.background.gradient ?? p.bg,
  };
  return vars;
}

/** Attributs posés sur <html> : ils activent les détails propres au thème dans le CSS. */
export function themeAttributes(t: Theme): Record<string, string> {
  return {
    "data-theme": t.id,
    "data-scheme": t.scheme,
    "data-glass": t.card.glass ? "1" : "0",
    "data-border": t.card.border,
    "data-pattern": t.card.pattern,
    "data-icons": t.icons.style,
    "data-scene": t.background.scene,
  };
}

// ---------------------------------------------------------------------------
// Saisons, particules, couleur dominante
// ---------------------------------------------------------------------------

/** Thème de la saison (hémisphère nord) : printemps Sakura, été Océan, automne Automne, hiver Hiver. */
export function seasonTheme(date: Date): string {
  const m = date.getMonth(); // 0 = janvier
  if (m >= 2 && m <= 4) return "sakura";
  if (m >= 5 && m <= 7) return "ocean";
  if (m >= 8 && m <= 10) return "autumn";
  return "winter";
}

/** Nombre de particules : proportionnel à la surface de la fenêtre, plafonné (budget CPU). */
export function particleCount(width: number, height: number, density: number, intensity = 1): number {
  if (density <= 0 || intensity <= 0) return 0;
  const n = Math.round(((width * height) / 38_000) * density * intensity);
  return Math.max(4, Math.min(70, n));
}

/**
 * Couleur dominante « vive » d'une image (pixels RGBA) : histogramme grossier, pondéré par la saturation,
 * en ignorant les pixels presque noirs, presque blancs ou transparents. `null` si l'image est grise.
 */
export function dominantColor(data: ArrayLike<number>): string | null {
  const buckets = new Map<number, { w: number; r: number; g: number; b: number }>();
  for (let i = 0; i + 3 < data.length; i += 4) {
    const [r, g, b, a] = [data[i], data[i + 1], data[i + 2], data[i + 3]];
    if (a < 128) continue;
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    if (max < 40 || min > 225) continue;
    const sat = max === 0 ? 0 : (max - min) / max;
    if (sat < 0.25) continue;
    const key = ((r >> 5) << 6) | ((g >> 5) << 3) | (b >> 5);
    const w = sat * sat;
    const e = buckets.get(key) ?? { w: 0, r: 0, g: 0, b: 0 };
    e.w += w;
    e.r += r * w;
    e.g += g * w;
    e.b += b * w;
    buckets.set(key, e);
  }
  let best: { w: number; r: number; g: number; b: number } | null = null;
  for (const e of buckets.values()) if (!best || e.w > best.w) best = e;
  if (!best || best.w === 0) return null;
  return toHex([best.r / best.w, best.g / best.w, best.b / best.w]);
}

function toHsl([r, g, b]: Rgba): [number, number, number] {
  const [R, G, B] = [r / 255, g / 255, b / 255];
  const max = Math.max(R, G, B);
  const min = Math.min(R, G, B);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h = max === R ? (G - B) / d + (G < B ? 6 : 0) : max === G ? (B - R) / d + 2 : (R - G) / d + 4;
  return [h * 60, s, l];
}

function fromHsl(h: number, s: number, l: number): string {
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return toHex([f(0) * 255, f(8) * 255, f(4) * 255]);
}

/**
 * Ajuste une couleur d'accent extraite d'une jaquette : teinte conservée, saturation relevée (au moins 55 %),
 * luminosité déplacée juste assez pour rester lisible (3:1) sur la surface du thème.
 */
export function fitAccent(color: string, panel: string, page: string): string {
  const [h, s0, l0] = toHsl(parseHex(color));
  const s = Math.max(s0, 0.55);
  const dark = contrast("#ffffff", page) > contrast("#000000", page);
  let l = l0;
  let out = fromHsl(h, s, l);
  for (let i = 0; i < 40 && contrast(out, panel, page) < 3; i++) {
    l = dark ? Math.min(0.95, l + 0.02) : Math.max(0.05, l - 0.02);
    out = fromHsl(h, s, l);
  }
  return out;
}
