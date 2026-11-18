// Images à partager (B-04, B-05) : récap de l'année et profil de joueur, dessinés sur un canvas aux couleurs du
// thème actif. Aucune ressource extérieure : jaquettes du cache local, polices embarquées.
import { imageUrl } from "../api";
import { formatSeconds } from "./format";

export interface CardData {
  kind: "recap" | "profile";
  heading: string;
  subheading: string;
  bigValue: string;
  bigLabel: string;
  stats: { label: string; value: string }[];
  games: { title: string; coverPath: string | null; value: string }[];
  profile: { emoji: string; title: string; description: string };
  badges: string[];
  footer: string;
}

const W = 1080;
const H = 1350;

function cssVar(name: string, fallback: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

function font(kind: "display" | "body", size: number, weight = 600): string {
  const family = cssVar(kind === "display" ? "--t-font-display" : "--t-font-body", "system-ui");
  return `${weight} ${size}px ${family}`;
}

function loadImage(path: string | null): Promise<HTMLImageElement | null> {
  const url = imageUrl(path);
  if (!url) return Promise.resolve(null);
  return new Promise((resolve) => {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => resolve(img);
    img.onerror = () => resolve(null);
    img.src = url;
  });
}

function roundRect(c: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  c.beginPath();
  c.moveTo(x + r, y);
  c.arcTo(x + w, y, x + w, y + h, r);
  c.arcTo(x + w, y + h, x, y + h, r);
  c.arcTo(x, y + h, x, y, r);
  c.arcTo(x, y, x + w, y, r);
  c.closePath();
}

function fitText(c: CanvasRenderingContext2D, text: string, max: number): string {
  if (c.measureText(text).width <= max) return text;
  let t = text;
  while (t.length > 1 && c.measureText(`${t}…`).width > max) t = t.slice(0, -1);
  return `${t}…`;
}

/** Dessine la carte et renvoie une image PNG (data URL). */
export async function renderCard(d: CardData): Promise<string> {
  await document.fonts.ready;
  const canvas = document.createElement("canvas");
  canvas.width = W;
  canvas.height = H;
  const c = canvas.getContext("2d")!;
  const bg = cssVar("--bg", "#0d0f15");
  const panel = cssVar("--panel2", "#1f2330");
  const fg = cssVar("--fg", "#e9ecf3");
  const muted = cssVar("--muted", "#9aa2b6");
  const accent = cssVar("--accent", "#2a63f5");
  const accentFg = cssVar("--accent-fg", "#ffffff");

  // Fond : couleur du thème, halo d'accent, motif de points.
  c.fillStyle = bg;
  c.fillRect(0, 0, W, H);
  const halo = c.createRadialGradient(W * 0.85, H * 0.05, 0, W * 0.85, H * 0.05, W * 0.9);
  halo.addColorStop(0, `${accent}66`);
  halo.addColorStop(1, `${accent}00`);
  c.fillStyle = halo;
  c.fillRect(0, 0, W, H);
  const halo2 = c.createRadialGradient(0, H, 0, 0, H, W * 0.8);
  halo2.addColorStop(0, `${accent}33`);
  halo2.addColorStop(1, `${accent}00`);
  c.fillStyle = halo2;
  c.fillRect(0, 0, W, H);
  c.fillStyle = `${fg}0d`;
  for (let y = 24; y < H; y += 36) for (let x = 24 + ((y / 36) % 2) * 18; x < W; x += 36) c.fillRect(x, y, 3, 3);

  const pad = 72;
  c.textBaseline = "alphabetic";
  c.fillStyle = accent;
  c.font = font("body", 28, 700);
  c.fillText(d.subheading.toUpperCase(), pad, 120);
  c.fillStyle = fg;
  c.font = font("display", 76, 700);
  c.fillText(fitText(c, d.heading, W - pad * 2), pad, 205);

  // Grand chiffre.
  c.font = font("display", 150, 800);
  c.fillStyle = fg;
  c.fillText(d.bigValue, pad, 400);
  c.font = font("body", 32, 500);
  c.fillStyle = muted;
  c.fillText(d.bigLabel, pad, 448);

  // Statistiques en tuiles.
  const tileW = (W - pad * 2 - 24 * 2) / 3;
  d.stats.slice(0, 3).forEach((s, i) => {
    const x = pad + i * (tileW + 24);
    roundRect(c, x, 490, tileW, 130, 22);
    c.fillStyle = `${panel}e6`;
    c.fill();
    c.fillStyle = fg;
    c.font = font("display", 46, 700);
    c.fillText(fitText(c, s.value, tileW - 40), x + 24, 560);
    c.fillStyle = muted;
    c.font = font("body", 24, 500);
    c.fillText(fitText(c, s.label, tileW - 40), x + 24, 598);
  });

  // Jeux (jaquettes).
  const covers = await Promise.all(d.games.slice(0, 5).map((g) => loadImage(g.coverPath)));
  const cw = (W - pad * 2 - 18 * 4) / 5;
  const ch = cw * 1.5;
  d.games.slice(0, 5).forEach((g, i) => {
    const x = pad + i * (cw + 18);
    const y = 670;
    c.save();
    roundRect(c, x, y, cw, ch, 16);
    c.clip();
    const img = covers[i];
    if (img) {
      c.drawImage(img, x, y, cw, ch);
    } else {
      const grad = c.createLinearGradient(x, y, x + cw, y + ch);
      grad.addColorStop(0, accent);
      grad.addColorStop(1, panel);
      c.fillStyle = grad;
      c.fillRect(x, y, cw, ch);
      c.fillStyle = accentFg;
      c.font = font("display", 54, 800);
      c.textAlign = "center";
      c.fillText(g.title.trim().charAt(0).toUpperCase(), x + cw / 2, y + ch / 2 + 18);
      c.textAlign = "left";
    }
    c.restore();
    c.fillStyle = accent;
    c.font = font("display", 26, 800);
    c.fillText(`#${i + 1}`, x + 4, y + ch + 38);
    c.fillStyle = fg;
    c.font = font("body", 22, 600);
    c.fillText(fitText(c, g.title, cw - 4), x + 4, y + ch + 68);
    c.fillStyle = muted;
    c.font = font("body", 20, 500);
    c.fillText(g.value, x + 4, y + ch + 94);
  });

  // Profil de joueur.
  const py = 1098;
  roundRect(c, pad, py, W - pad * 2, 150, 26);
  c.fillStyle = `${panel}f0`;
  c.fill();
  c.strokeStyle = `${accent}99`;
  c.lineWidth = 3;
  c.stroke();
  c.font = `86px "Segoe UI Emoji", system-ui`;
  c.fillText(d.profile.emoji, pad + 30, py + 108);
  c.fillStyle = fg;
  c.font = font("display", 44, 700);
  c.fillText(fitText(c, d.profile.title, W - pad * 2 - 190), pad + 150, py + 70);
  c.fillStyle = muted;
  c.font = font("body", 26, 500);
  c.fillText(fitText(c, d.profile.description, W - pad * 2 - 190), pad + 150, py + 112);

  c.fillStyle = muted;
  c.font = font("body", 22, 500);
  c.fillText(fitText(c, d.badges.length ? `Badges : ${d.badges.join(" · ")}` : "", W - pad * 2), pad, 1295);
  c.textAlign = "right";
  c.fillStyle = accent;
  c.font = font("display", 24, 700);
  c.fillText(d.footer, W - pad, 1295);
  return canvas.toDataURL("image/png");
}

export interface ProfileCardData {
  name: string;
  /** Photo recadrée (data URL), sinon l'emoji. */
  photo: string | null;
  emoji: string;
  title: string;
  description: string;
  stats: { label: string; value: string }[];
  top: { title: string; coverPath: string | null }[];
  badges: { icon: string; name: string }[];
}

function loadUrl(url: string | null): Promise<HTMLImageElement | null> {
  if (!url) return Promise.resolve(null);
  return new Promise((resolve) => {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => resolve(img);
    img.onerror = () => resolve(null);
    img.src = url;
  });
}

/** Image du profil (B-05) : à montrer à n'importe qui. 1080 × 1350, couleurs et polices du thème actif. */
export async function renderProfileCard(d: ProfileCardData): Promise<string> {
  await document.fonts.ready;
  const canvas = document.createElement("canvas");
  canvas.width = W;
  canvas.height = H;
  const c = canvas.getContext("2d")!;
  const bg = cssVar("--bg", "#0d0f15");
  const panel = cssVar("--panel2", "#1f2330");
  const fg = cssVar("--fg", "#e9ecf3");
  const muted = cssVar("--muted", "#9aa2b6");
  const accent = cssVar("--accent", "#2a63f5");
  const accentFg = cssVar("--accent-fg", "#ffffff");
  const pad = 64;

  // Fond : bandeau d'accent en haut, halo, fin quadrillage de points.
  c.fillStyle = bg;
  c.fillRect(0, 0, W, H);
  const band = c.createLinearGradient(0, 0, W, 330);
  band.addColorStop(0, `${accent}aa`);
  band.addColorStop(1, `${accent}22`);
  c.fillStyle = band;
  c.fillRect(0, 0, W, 330);
  const fade = c.createLinearGradient(0, 200, 0, 330);
  fade.addColorStop(0, `${bg}00`);
  fade.addColorStop(1, bg);
  c.fillStyle = fade;
  c.fillRect(0, 200, W, 130);
  c.fillStyle = `${fg}0b`;
  for (let y = 20; y < H; y += 34) for (let x = 20 + ((y / 34) % 2) * 17; x < W; x += 34) c.fillRect(x, y, 3, 3);

  // Photo (ou emoji) dans un anneau d'accent.
  const r = 118;
  const cx = pad + r;
  const cy = 200;
  c.beginPath();
  c.arc(cx, cy, r + 10, 0, Math.PI * 2);
  c.fillStyle = accent;
  c.fill();
  c.beginPath();
  c.arc(cx, cy, r + 4, 0, Math.PI * 2);
  c.fillStyle = bg;
  c.fill();
  const photo = await loadUrl(d.photo);
  c.save();
  c.beginPath();
  c.arc(cx, cy, r, 0, Math.PI * 2);
  c.clip();
  if (photo) {
    c.drawImage(photo, cx - r, cy - r, r * 2, r * 2);
  } else {
    c.fillStyle = panel;
    c.fillRect(cx - r, cy - r, r * 2, r * 2);
    c.font = `130px "Segoe UI Emoji", system-ui`;
    c.textAlign = "center";
    c.textBaseline = "middle";
    c.fillText(d.emoji, cx, cy + 8);
    c.textAlign = "left";
    c.textBaseline = "alphabetic";
  }
  c.restore();

  const tx = cx + r + 44;
  const tw = W - tx - pad;
  c.fillStyle = fg;
  c.font = font("display", 72, 800);
  c.fillText(fitText(c, d.name, tw), tx, 190);
  c.font = font("body", 30, 700);
  c.fillStyle = accent;
  c.fillText(fitText(c, d.title.toUpperCase(), tw), tx, 240);
  c.font = font("body", 24, 500);
  c.fillStyle = muted;
  c.fillText(fitText(c, d.description, tw), tx, 280);

  // Statistiques.
  const gap = 18;
  const n = Math.min(4, d.stats.length);
  const tileW = (W - pad * 2 - gap * (n - 1)) / n;
  d.stats.slice(0, 4).forEach((s, i) => {
    const x = pad + i * (tileW + gap);
    roundRect(c, x, 362, tileW, 118, 20);
    c.fillStyle = `${panel}ee`;
    c.fill();
    c.fillStyle = fg;
    c.font = font("display", 40, 800);
    c.fillText(fitText(c, s.value, tileW - 36), x + 22, 420);
    c.fillStyle = muted;
    c.font = font("body", 18, 600);
    c.fillText(fitText(c, s.label.toUpperCase(), tileW - 36), x + 22, 456);
  });

  // Top 10 : deux rangées de cinq jaquettes numérotées.
  c.fillStyle = fg;
  c.font = font("display", 38, 800);
  c.fillText("Mes 10 jeux préférés", pad, 548);
  const covers = await Promise.all(d.top.slice(0, 10).map((g) => loadImage(g.coverPath)));
  const cg = 16;
  const cw = (W - pad * 2 - cg * 4) / 5;
  const ch = cw * 1.5;
  for (let i = 0; i < 10; i++) {
    const x = pad + (i % 5) * (cw + cg);
    const y = 578 + Math.floor(i / 5) * (ch + 22);
    const g = d.top[i];
    c.save();
    roundRect(c, x, y, cw, ch, 14);
    c.clip();
    if (g) {
      const img = covers[i];
      if (img) {
        c.drawImage(img, x, y, cw, ch);
      } else {
        // Pas de jaquette : teinte propre au jeu (le titre est écrit en bas).
        let h = 0;
        for (const ch2 of g.title) h = (h * 31 + ch2.charCodeAt(0)) % 360;
        const grad = c.createLinearGradient(x, y, x + cw, y + ch);
        grad.addColorStop(0, `hsl(${h} 45% 38%)`);
        grad.addColorStop(1, `hsl(${(h + 50) % 360} 50% 16%)`);
        c.fillStyle = grad;
        c.fillRect(x, y, cw, ch);
      }
      const shade = c.createLinearGradient(0, y + ch - 90, 0, y + ch);
      shade.addColorStop(0, "#00000000");
      shade.addColorStop(1, "#000000b0");
      c.fillStyle = shade;
      c.fillRect(x, y + ch - 90, cw, 90);
    } else {
      c.fillStyle = `${panel}99`;
      c.fillRect(x, y, cw, ch);
    }
    c.restore();
    // Numéro dans une pastille.
    c.beginPath();
    c.arc(x + 30, y + 30, 21, 0, Math.PI * 2);
    c.fillStyle = i < 3 ? accent : `${bg}dd`;
    c.fill();
    c.fillStyle = i < 3 ? accentFg : fg;
    c.font = font("display", 22, 800);
    c.textAlign = "center";
    c.fillText(String(i + 1), x + 30, y + 38);
    c.textAlign = "left";
    if (g) {
      c.fillStyle = "#ffffff";
      c.font = font("body", 19, 700);
      c.fillText(fitText(c, g.title, cw - 20), x + 10, y + ch - 14);
    }
  }

  // Badges et signature.
  const by = 578 + 2 * (ch + 22) + 14;
  let bx = pad;
  c.font = font("body", 22, 700);
  for (const b of d.badges.slice(0, 6)) {
    const label = `${b.icon} ${b.name}`;
    const w = c.measureText(label).width + 36;
    if (bx + w > W - pad) break;
    roundRect(c, bx, by, w, 46, 23);
    c.fillStyle = `${accent}26`;
    c.fill();
    c.strokeStyle = `${accent}88`;
    c.lineWidth = 2;
    c.stroke();
    c.fillStyle = fg;
    c.font = `600 22px "Segoe UI Emoji", ${cssVar("--t-font-body", "system-ui")}`;
    c.fillText(label, bx + 18, by + 31);
    bx += w + 12;
  }
  c.textAlign = "right";
  c.fillStyle = accent;
  c.font = font("display", 24, 800);
  c.fillText("GameLauncher", W - pad, H - 40);
  c.textAlign = "left";
  return canvas.toDataURL("image/png");
}

export const hoursLabel = (s: number) => formatSeconds(s);
