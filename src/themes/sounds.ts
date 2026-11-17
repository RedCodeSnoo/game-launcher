// Sons d'interface optionnels, synthétisés (Web Audio) : aucun fichier audio.
let ctx: AudioContext | null = null;

export type SoundSet = "none" | "soft" | "8bit";

export function playUiSound(set: SoundSet, kind: "click" | "nav" = "click") {
  if (set === "none") return;
  try {
    ctx ??= new AudioContext();
    const now = ctx.currentTime;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    if (set === "8bit") {
      osc.type = "square";
      osc.frequency.setValueAtTime(kind === "nav" ? 660 : 880, now);
      osc.frequency.setValueAtTime(kind === "nav" ? 990 : 1320, now + 0.045);
      gain.gain.setValueAtTime(0.035, now);
      gain.gain.setValueAtTime(0, now + 0.09);
    } else {
      osc.type = "sine";
      osc.frequency.setValueAtTime(kind === "nav" ? 520 : 680, now);
      osc.frequency.exponentialRampToValueAtTime(kind === "nav" ? 780 : 900, now + 0.06);
      gain.gain.setValueAtTime(0.0001, now);
      gain.gain.exponentialRampToValueAtTime(0.05, now + 0.01);
      gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.12);
    }
    osc.connect(gain).connect(ctx.destination);
    osc.start(now);
    osc.stop(now + 0.14);
  } catch {
    // Audio indisponible : sans conséquence.
  }
}
