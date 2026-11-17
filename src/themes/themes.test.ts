import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { contrast, dominantColor, fitAccent, particleCount, seasonTheme, themeToCssVars, validateTheme, type Theme } from "./schema";

const dir = join(__dirname, "builtin");
const raw = readdirSync(dir)
  .filter((f) => f.endsWith(".json"))
  .map((f) => JSON.parse(readFileSync(join(dir, f), "utf8")));
const themes: Theme[] = raw.map((r) => {
  const { theme, errors } = validateTheme(r);
  if (!theme) throw new Error(`${r.id} : ${errors.join(" ")}`);
  return theme;
});

describe("thèmes intégrés", () => {
  it("les 10 thèmes demandés sont présents et valides", () => {
    expect(themes.map((t) => t.id).sort()).toEqual(["autumn", "dark", "kyoto", "light", "neon", "ocean", "oled", "retro", "sakura", "winter"]);
  });

  // WCAG AA : 4,5:1 pour le texte courant, 3:1 pour les éléments d'interface et le texte en accent.
  for (const t of themes) {
    it(`${t.name} : contrastes lisibles (WCAG AA)`, () => {
      const p = t.palette;
      const checks: [string, number, number][] = [
        ["texte / fond", contrast(p.fg, p.bg), 4.5],
        ["texte / surface", contrast(p.fg, p.panel, p.bg), 4.5],
        ["texte / surface 2", contrast(p.fg, p.panel2, p.bg), 4.5],
        ["texte secondaire / surface", contrast(p.muted, p.panel, p.bg), 4.5],
        ["texte secondaire / fond", contrast(p.muted, p.bg), 4.5],
        ["texte sur bouton principal", contrast(p.accentFg, p.accent), 4.5],
        ["accent / surface", contrast(p.accent, p.panel, p.bg), 3],
        ["succès / surface", contrast(p.ok, p.panel, p.bg), 3],
        ["alerte / surface", contrast(p.warn, p.panel, p.bg), 3],
        ["erreur / surface", contrast(p.danger, p.panel, p.bg), 3],
        ["texte sur voile", contrast(p.onScrim, p.scrim), 4.5],
      ];
      const failures = checks.filter(([, ratio, min]) => ratio < min).map(([what, ratio, min]) => `${what} ${ratio.toFixed(2)} < ${min}`);
      expect(failures).toEqual([]);
    });
  }
});

describe("validation des thèmes JSON", () => {
  const base = raw.find((r) => r.id === "sakura");

  it("un thème importé valide est accepté et complété", () => {
    const { theme, errors } = validateTheme({ ...base, id: "mon-theme", name: "Mon thème", particles: undefined, motion: undefined });
    expect(errors).toEqual([]);
    expect(theme?.particles).toEqual({ kind: "none", density: 1 });
    expect(theme?.motion).toBe(160);
  });

  it("les erreurs sont listées et rien n'est accepté", () => {
    const { theme, errors } = validateTheme({ ...base, id: "Mauvais Id", palette: { ...base.palette, fg: "rouge" }, particles: { kind: "confettis", density: 9 } });
    expect(theme).toBeNull();
    expect(errors.join("\n")).toMatch(/id/);
    expect(errors.join("\n")).toMatch(/palette\.fg/);
    expect(errors.join("\n")).toMatch(/particles\.kind/);
    expect(errors.join("\n")).toMatch(/particles\.density/);
    expect(validateTheme("pas un objet").errors.length).toBe(1);
    expect(validateTheme(null).theme).toBeNull();
  });

  it("refuse les fonds qui chargeraient une ressource extérieure", () => {
    const evil = validateTheme({ ...base, id: "x", background: { scene: "none", gradient: "url(https://exemple.test/x.png)" } });
    expect(evil.theme).toBeNull();
    const svg = validateTheme({ ...base, id: "x", background: { scene: "none", image: "data:image/svg+xml;base64,AAAA" } });
    expect(svg.theme).toBeNull();
    const ok = validateTheme({ ...base, id: "x", background: { scene: "none", image: "data:image/png;base64,iVBORw0KGgo=" } });
    expect(ok.errors).toEqual([]);
  });

  it("produit les variables CSS du thème", () => {
    const vars = themeToCssVars(themes.find((t) => t.id === "retro")!);
    expect(vars["--accent"]).toBe("#55ffff");
    expect(vars["--t-radius-card"]).toBe("0px");
    expect(vars["--t-font-display"]).toContain("Press Start 2P");
    expect(vars["--t-font-scale"]).toBe("1.18");
  });
});

describe("outils des thèmes", () => {
  it("thème de la saison", () => {
    expect(seasonTheme(new Date(2026, 3, 10))).toBe("sakura");
    expect(seasonTheme(new Date(2026, 6, 1))).toBe("ocean");
    expect(seasonTheme(new Date(2026, 9, 31))).toBe("autumn");
    expect(seasonTheme(new Date(2026, 11, 25))).toBe("winter");
    expect(seasonTheme(new Date(2027, 1, 1))).toBe("winter");
  });

  it("nombre de particules adapté à la fenêtre et plafonné", () => {
    expect(particleCount(800, 600, 1)).toBeLessThan(particleCount(1920, 1080, 1));
    expect(particleCount(3840, 2160, 3)).toBe(70);
    expect(particleCount(1920, 1080, 0)).toBe(0);
    expect(particleCount(1920, 1080, 1, 0)).toBe(0);
  });

  it("couleur dominante d'une jaquette, en ignorant les gris", () => {
    const px = (r: number, g: number, b: number, n: number) => Array.from({ length: n }, () => [r, g, b, 255]).flat();
    const img = [...px(20, 20, 20, 500), ...px(200, 200, 200, 300), ...px(220, 40, 60, 120), ...px(40, 120, 220, 40)];
    const c = dominantColor(img)!;
    expect(parseInt(c.slice(1, 3), 16)).toBeGreaterThan(180);
    expect(dominantColor(px(128, 128, 128, 100))).toBeNull();
  });

  it("l'accent extrait reste lisible sur la surface du thème", () => {
    expect(contrast(fitAccent("#202060", "#161922", "#0d0f15"), "#161922", "#0d0f15")).toBeGreaterThanOrEqual(3);
    expect(contrast(fitAccent("#ffe680", "#ffffff", "#f4f3f0"), "#ffffff", "#f4f3f0")).toBeGreaterThanOrEqual(3);
  });
});
