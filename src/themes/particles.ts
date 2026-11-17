// Particules des thèmes sur un <canvas> unique : pétales, feuilles d'érable, flocons, lucioles, bulles, étoiles
// en pixels. Budget CPU : 30 images/s au plus, nombre de particules proportionnel à la fenêtre et plafonné,
// rendu à la résolution 1:1 (pas de suréchantillonnage) et arrêt complet de la boucle quand c'est en pause.
import { particleCount, type ParticleKind } from "./schema";

interface P {
  x: number;
  y: number;
  vx: number;
  vy: number;
  size: number;
  rot: number;
  vr: number;
  phase: number;
  color: string;
  depth: number;
}

const COLORS: Record<Exclude<ParticleKind, "none">, string[]> = {
  petals: ["#f7b6c8", "#f9cbd7", "#f29bb4", "#fde1e8"],
  leaves: ["#d65a24", "#b83a1d", "#e8a033", "#9c2a22", "#c97a2b"],
  snow: ["#ffffff", "#f2f8ff", "#e3effa"],
  fireflies: ["#ffd76b", "#fff1a8", "#ffc14d"],
  bubbles: ["#bdf3ee", "#e2fbff", "#9fe4f0"],
  pixelstars: ["#ffffff", "#55ffff", "#ff55ff", "#ffff55"],
};

const FRAME_MS = 1000 / 30;

function rand(a: number, b: number) {
  return a + Math.random() * (b - a);
}

export class ParticleField {
  private ctx: CanvasRenderingContext2D | null;
  private items: P[] = [];
  private raf = 0;
  private last = 0;
  private running = false;
  private w = 0;
  private h = 0;

  constructor(
    private canvas: HTMLCanvasElement,
    private kind: ParticleKind,
    private density: number,
  ) {
    this.ctx = canvas.getContext("2d");
    this.resize();
  }

  resize() {
    this.w = this.canvas.width = this.canvas.clientWidth || window.innerWidth;
    this.h = this.canvas.height = this.canvas.clientHeight || window.innerHeight;
    const n = this.kind === "none" ? 0 : particleCount(this.w, this.h, this.density);
    while (this.items.length < n) this.items.push(this.spawn(true));
    this.items.length = n;
    this.draw(0);
  }

  private spawn(anywhere: boolean): P {
    const k = this.kind as Exclude<ParticleKind, "none">;
    const palette = COLORS[k] ?? ["#ffffff"];
    const depth = rand(0.45, 1);
    const p: P = {
      x: rand(0, this.w),
      y: anywhere ? rand(0, this.h) : -20,
      vx: 0,
      vy: 0,
      size: 1,
      rot: rand(0, Math.PI * 2),
      vr: rand(-0.02, 0.02),
      phase: rand(0, Math.PI * 2),
      color: palette[Math.floor(Math.random() * palette.length)],
      depth,
    };
    switch (k) {
      case "petals":
        Object.assign(p, { size: rand(5, 9) * depth, vy: rand(0.35, 0.8) * depth, vx: rand(0.15, 0.5), vr: rand(-0.025, 0.025) });
        break;
      case "leaves":
        Object.assign(p, { size: rand(8, 15) * depth, vy: rand(0.45, 1) * depth, vx: rand(-0.2, 0.3), vr: rand(-0.04, 0.04) });
        break;
      case "snow":
        Object.assign(p, { size: rand(1.2, 3.2) * depth, vy: rand(0.3, 0.9) * depth, vx: rand(-0.2, 0.2) });
        break;
      case "fireflies":
        Object.assign(p, { size: rand(1.4, 2.6), vx: rand(-0.25, 0.25), vy: rand(-0.2, 0.2), y: anywhere ? rand(this.h * 0.35, this.h) : rand(this.h * 0.35, this.h) });
        break;
      case "bubbles":
        Object.assign(p, { size: rand(2, 7) * depth, vy: -rand(0.25, 0.7) * depth, y: anywhere ? rand(0, this.h) : this.h + 10 });
        break;
      case "pixelstars":
        Object.assign(p, { size: Math.round(rand(1, 3.4)), vx: -rand(0.3, 1.6) * depth, x: anywhere ? rand(0, this.w) : this.w + 4, y: rand(0, this.h) });
        break;
    }
    return p;
  }

  start() {
    if (this.running || this.items.length === 0) return;
    this.running = true;
    this.last = performance.now();
    const loop = (now: number) => {
      if (!this.running) return;
      this.raf = requestAnimationFrame(loop);
      const dt = now - this.last;
      if (dt < FRAME_MS) return;
      this.last = now;
      this.step(Math.min(dt, 100) / 16.7, now);
      this.draw(now);
    };
    this.raf = requestAnimationFrame(loop);
  }

  stop() {
    this.running = false;
    cancelAnimationFrame(this.raf);
  }

  destroy() {
    this.stop();
    this.ctx?.clearRect(0, 0, this.w, this.h);
  }

  private step(k: number, now: number) {
    const t = now / 1000;
    for (let i = 0; i < this.items.length; i++) {
      const p = this.items[i];
      switch (this.kind) {
        case "petals":
        case "leaves":
          p.x += (p.vx + Math.sin(t * 0.9 + p.phase) * 0.45) * k;
          p.y += p.vy * k;
          p.rot += p.vr * k;
          break;
        case "snow":
          p.x += (p.vx + Math.sin(t * 0.6 + p.phase) * 0.25) * k;
          p.y += p.vy * k;
          break;
        case "fireflies":
          p.x += (p.vx + Math.sin(t * 0.7 + p.phase) * 0.18) * k;
          p.y += (p.vy + Math.cos(t * 0.5 + p.phase) * 0.15) * k;
          break;
        case "bubbles":
          p.x += Math.sin(t * 1.4 + p.phase) * 0.2 * k;
          p.y += p.vy * k;
          break;
        case "pixelstars":
          p.x += p.vx * k;
          break;
      }
      const out = p.y > this.h + 24 || p.y < -30 || p.x > this.w + 30 || p.x < -30;
      if (out) this.items[i] = this.spawn(false);
      if (this.kind === "fireflies" && out) this.items[i].x = rand(0, this.w);
      if (this.kind === "pixelstars" && out) this.items[i].x = this.w + 2;
    }
  }

  private draw(now: number) {
    const c = this.ctx;
    if (!c) return;
    c.clearRect(0, 0, this.w, this.h);
    const t = now / 1000;
    for (const p of this.items) {
      switch (this.kind) {
        case "petals":
          drawPetal(c, p);
          break;
        case "leaves":
          drawMaple(c, p);
          break;
        case "snow":
          c.globalAlpha = 0.55 + 0.4 * p.depth;
          c.fillStyle = p.color;
          c.beginPath();
          c.arc(p.x, p.y, p.size, 0, Math.PI * 2);
          c.fill();
          break;
        case "fireflies": {
          const glow = 0.35 + 0.65 * Math.max(0, Math.sin(t * 1.6 + p.phase));
          const sprite = glowSprite(p.color);
          const r = p.size * 6;
          c.globalAlpha = glow;
          c.drawImage(sprite, p.x - r, p.y - r, r * 2, r * 2);
          break;
        }
        case "bubbles":
          c.globalAlpha = 0.35 * p.depth + 0.15;
          c.strokeStyle = p.color;
          c.lineWidth = 1;
          c.beginPath();
          c.arc(p.x, p.y, p.size, 0, Math.PI * 2);
          c.stroke();
          c.globalAlpha *= 0.8;
          c.beginPath();
          c.arc(p.x - p.size * 0.35, p.y - p.size * 0.35, p.size * 0.22, 0, Math.PI * 2);
          c.fillStyle = p.color;
          c.fill();
          break;
        case "pixelstars": {
          const twinkle = p.size > 2 ? 0.6 + 0.4 * Math.abs(Math.sin(t * 2 + p.phase)) : 0.75;
          c.globalAlpha = twinkle;
          c.fillStyle = p.color;
          c.fillRect(Math.round(p.x), Math.round(p.y), p.size, p.size);
          break;
        }
      }
    }
    c.globalAlpha = 1;
  }
}

const sprites = new Map<string, HTMLCanvasElement>();

/** Halo lumineux pré-rendu une fois par couleur (évite un dégradé par particule et par image). */
function glowSprite(color: string): HTMLCanvasElement {
  let s = sprites.get(color);
  if (s) return s;
  s = document.createElement("canvas");
  s.width = s.height = 32;
  const c = s.getContext("2d");
  if (c) {
    const g = c.createRadialGradient(16, 16, 0, 16, 16, 16);
    g.addColorStop(0, color);
    g.addColorStop(0.25, color);
    g.addColorStop(1, "rgba(255, 210, 100, 0)");
    c.fillStyle = g;
    c.fillRect(0, 0, 32, 32);
  }
  sprites.set(color, s);
  return s;
}

/** Pétale de cerisier : forme en goutte avec l'encoche caractéristique au bout. */
function drawPetal(c: CanvasRenderingContext2D, p: P) {
  const s = p.size;
  c.save();
  c.translate(p.x, p.y);
  c.rotate(p.rot);
  c.scale(1, 0.55 + 0.45 * Math.abs(Math.sin(p.rot * 2 + p.phase)));
  c.globalAlpha = 0.55 + 0.35 * p.depth;
  c.fillStyle = p.color;
  c.beginPath();
  c.moveTo(0, -s);
  c.bezierCurveTo(s * 0.9, -s * 0.8, s * 0.8, s * 0.55, s * 0.18, s);
  c.lineTo(0, s * 0.72);
  c.lineTo(-s * 0.18, s);
  c.bezierCurveTo(-s * 0.8, s * 0.55, -s * 0.9, -s * 0.8, 0, -s);
  c.fill();
  c.globalAlpha *= 0.5;
  c.fillStyle = "#ffffff";
  c.beginPath();
  c.ellipse(-s * 0.15, -s * 0.35, s * 0.18, s * 0.4, 0.3, 0, Math.PI * 2);
  c.fill();
  c.restore();
}

/** Feuille d'érable stylisée à cinq lobes, tracée en polygone. */
const MAPLE: [number, number][] = [
  [0, -1], [0.16, -0.55], [0.45, -0.72], [0.38, -0.32], [0.95, -0.38], [0.62, -0.05], [0.8, 0.12], [0.3, 0.16], [0.36, 0.5], [0.06, 0.32],
  [0, 0.62], [-0.06, 0.32], [-0.36, 0.5], [-0.3, 0.16], [-0.8, 0.12], [-0.62, -0.05], [-0.95, -0.38], [-0.38, -0.32], [-0.45, -0.72], [-0.16, -0.55],
];

function drawMaple(c: CanvasRenderingContext2D, p: P) {
  const s = p.size;
  c.save();
  c.translate(p.x, p.y);
  c.rotate(p.rot);
  c.scale(1, 0.6 + 0.4 * Math.abs(Math.cos(p.rot * 1.5 + p.phase)));
  c.globalAlpha = 0.6 + 0.35 * p.depth;
  c.fillStyle = p.color;
  c.beginPath();
  MAPLE.forEach(([x, y], i) => (i === 0 ? c.moveTo(x * s, y * s) : c.lineTo(x * s, y * s)));
  c.closePath();
  c.fill();
  c.globalAlpha *= 0.55;
  c.strokeStyle = "rgba(60, 20, 10, 0.55)";
  c.lineWidth = Math.max(0.6, s * 0.06);
  c.beginPath();
  c.moveTo(0, s * 0.95);
  c.lineTo(0, -s * 0.6);
  c.stroke();
  c.restore();
}
