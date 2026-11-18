// Glisser-déposer à la souris (pointer events), géré par l'application.
//
// Sous Windows, Tauri intercepte le glisser-déposer natif pour accepter les .exe déposés dans la fenêtre :
// le glisser-déposer HTML5 ne reçoit alors jamais l'événement « drop ». Ce module ne dépend que des
// événements de pointeur, qui fonctionnent partout.
import { useEffect, useRef, useState } from "react";

export interface DragPayload {
  gameId: string;
  title: string;
  /** Adresse de la jaquette pour l'aperçu qui suit le pointeur. */
  cover?: string;
  /** Origine (ex. « top:3 ») : permet de réordonner une liste. */
  from?: string;
}

/** Distance (px) à parcourir avant que le glissement commence : en dessous, c'est un clic. */
export const DRAG_THRESHOLD = 6;

/** Le glissement doit-il commencer ? (pure, testée) */
export function passedThreshold(start: { x: number; y: number }, now: { x: number; y: number }): boolean {
  return Math.hypot(now.x - start.x, now.y - start.y) >= DRAG_THRESHOLD;
}

/** Glissement en cours, lu par les zones de dépôt. */
let current: DragPayload | null = null;
const listeners = new Set<(p: DragPayload | null) => void>();
const setCurrent = (p: DragPayload | null) => {
  current = p;
  listeners.forEach((l) => l(p));
};

export function useDragging(): DragPayload | null {
  const [p, setP] = useState(current);
  useEffect(() => {
    listeners.add(setP);
    return () => void listeners.delete(setP);
  }, []);
  return p;
}

function ghost(p: DragPayload): HTMLElement {
  const el = document.createElement("div");
  el.className = "drag-ghost";
  if (p.cover) {
    const img = document.createElement("img");
    img.src = p.cover;
    img.alt = "";
    el.appendChild(img);
  }
  const label = document.createElement("span");
  label.textContent = p.title;
  el.appendChild(label);
  document.body.appendChild(el);
  return el;
}

const targetAt = (x: number, y: number) => document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-drop]") ?? null;

/**
 * À brancher sur `onPointerDown` d'un élément glissable. Un simple clic reste un clic ; au-delà du seuil,
 * un aperçu suit le pointeur et la zone `[data-drop]` survolée reçoit l'événement `game-drop`.
 */
export function startDrag(e: React.PointerEvent, payload: DragPayload) {
  if (e.button !== 0 || e.pointerType === "touch") return;
  const start = { x: e.clientX, y: e.clientY };
  let el: HTMLElement | null = null;
  let over: HTMLElement | null = null;

  const setOver = (t: HTMLElement | null) => {
    if (t === over) return;
    over?.classList.remove("drop-over");
    over = t;
    over?.classList.add("drop-over");
  };
  const move = (ev: PointerEvent) => {
    if (!el) {
      if (!passedThreshold(start, { x: ev.clientX, y: ev.clientY })) return;
      el = ghost(payload);
      document.body.classList.add("dragging");
      window.getSelection()?.removeAllRanges();
      setCurrent(payload);
    }
    el.style.transform = `translate(${ev.clientX + 12}px, ${ev.clientY + 12}px)`;
    setOver(targetAt(ev.clientX, ev.clientY));
  };
  const finish = (ev: PointerEvent | null) => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    window.removeEventListener("keydown", key, true);
    if (!el) return;
    const target = ev ? targetAt(ev.clientX, ev.clientY) : null;
    setOver(null);
    el.remove();
    document.body.classList.remove("dragging");
    setCurrent(null);
    // Le clic qui suit un glissement n'ouvre pas la fiche du jeu.
    const swallow = (c: MouseEvent) => {
      c.stopPropagation();
      c.preventDefault();
    };
    window.addEventListener("click", swallow, { capture: true, once: true });
    setTimeout(() => window.removeEventListener("click", swallow, { capture: true }), 0);
    target?.dispatchEvent(new CustomEvent<DragPayload>("game-drop", { detail: payload }));
  };
  const up = (ev: PointerEvent) => finish(ev);
  const key = (ev: KeyboardEvent) => {
    if (ev.key === "Escape") {
      ev.stopPropagation();
      finish(null);
    }
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
  window.addEventListener("keydown", key, true);
}

/** Zone de dépôt : renvoie la ref à poser sur l'élément (avec l'attribut `data-drop`). */
export function useDrop<T extends HTMLElement>(onDrop: ((p: DragPayload) => void) | undefined) {
  const ref = useRef<T>(null);
  const cb = useRef(onDrop);
  cb.current = onDrop;
  useEffect(() => {
    const el = ref.current;
    if (!el || !onDrop) return;
    const h = (e: Event) => cb.current?.((e as CustomEvent<DragPayload>).detail);
    el.addEventListener("game-drop", h);
    return () => el.removeEventListener("game-drop", h);
  }, [onDrop === undefined]); // eslint-disable-line react-hooks/exhaustive-deps
  return ref;
}

/** Déplace l'élément `from` à la position `to` (réordonnancement d'une liste). */
export function reorder<T>(list: T[], from: number, to: number): T[] {
  if (from === to || from < 0 || from >= list.length) return list;
  const out = [...list];
  const [item] = out.splice(from, 1);
  out.splice(Math.min(Math.max(to, 0), out.length), 0, item);
  return out;
}
