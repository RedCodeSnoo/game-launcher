// Manette (Gamepad API, disposition « standard » : Xbox, PlayStation…) : transforme l'état des boutons en
// actions de navigation, avec répétition quand on maintient une direction. Logique pure, testée.

export type PadAction = "up" | "down" | "left" | "right" | "select" | "back" | "details" | "menu" | "toggle" | "prevRow" | "nextRow";

/** État simplifié d'une manette à un instant. */
export interface PadState {
  buttons: boolean[];
  axes: number[];
}

const DEADZONE = 0.55;
/** Délai avant répétition, puis intervalle de répétition (ms). */
export const REPEAT_DELAY = 380;
export const REPEAT_EVERY = 110;

const BUTTONS: [number, PadAction][] = [
  [0, "select"], // A / Croix
  [1, "back"], // B / Rond
  [3, "details"], // Y / Triangle
  [9, "menu"], // Menu / Options
  [16, "toggle"], // Guide / PS
  [4, "prevRow"], // LB / L1
  [5, "nextRow"], // RB / R1
];

function directions(s: PadState): Set<PadAction> {
  const d = new Set<PadAction>();
  const [x = 0, y = 0] = s.axes;
  if (s.buttons[12] || y < -DEADZONE) d.add("up");
  if (s.buttons[13] || y > DEADZONE) d.add("down");
  if (s.buttons[14] || x < -DEADZONE) d.add("left");
  if (s.buttons[15] || x > DEADZONE) d.add("right");
  return d;
}

/** Mémoire d'une manette entre deux lectures. */
export interface PadMemory {
  prev: PadState | null;
  /** Direction maintenue et date de la prochaine répétition. */
  held: { dir: PadAction; next: number } | null;
}

export function newMemory(): PadMemory {
  return { prev: null, held: null };
}

/**
 * Actions déclenchées entre l'état précédent et l'état courant : boutons au moment de l'appui, directions à l'appui
 * puis répétées tant qu'elles sont maintenues. « Vue + Menu » ensemble = basculer (comme le bouton Guide, souvent
 * réservé par Windows à la Xbox Game Bar).
 */
export function readPad(mem: PadMemory, cur: PadState, now: number): PadAction[] {
  const out: PadAction[] = [];
  const prev = mem.prev;
  const pressed = (i: number) => !!cur.buttons[i] && !(prev?.buttons[i] ?? false);
  for (const [i, action] of BUTTONS) if (pressed(i)) out.push(action);
  if ((pressed(8) && cur.buttons[9]) || (pressed(9) && cur.buttons[8])) {
    out.splice(out.indexOf("menu"), out.includes("menu") ? 1 : 0);
    out.push("toggle");
  }
  const now_dirs = directions(cur);
  const before = prev ? directions(prev) : new Set<PadAction>();
  const fresh = [...now_dirs].find((d) => !before.has(d));
  if (fresh) {
    out.push(fresh);
    mem.held = { dir: fresh, next: now + REPEAT_DELAY };
  } else if (mem.held && now_dirs.has(mem.held.dir)) {
    if (now >= mem.held.next) {
      out.push(mem.held.dir);
      mem.held.next = now + REPEAT_EVERY;
    }
  } else {
    mem.held = null;
  }
  mem.prev = { buttons: [...cur.buttons], axes: [...cur.axes] };
  return out;
}

/** Déplacement dans une grille de rangées de longueurs différentes (la colonne est conservée au mieux). */
export function moveFocus(rows: number[], pos: { row: number; col: number }, action: PadAction): { row: number; col: number } {
  if (rows.length === 0) return pos;
  let { row, col } = pos;
  switch (action) {
    case "left":
      col = Math.max(0, col - 1);
      break;
    case "right":
      col = Math.min(rows[row] - 1, col + 1);
      break;
    case "up":
    case "prevRow":
      row = Math.max(0, row - 1);
      break;
    case "down":
    case "nextRow":
      row = Math.min(rows.length - 1, row + 1);
      break;
    default:
      return pos;
  }
  col = Math.min(col, Math.max(0, rows[row] - 1));
  return { row, col };
}
