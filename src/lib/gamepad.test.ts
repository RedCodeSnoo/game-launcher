import { describe, expect, it } from "vitest";
import { moveFocus, newMemory, readPad, REPEAT_DELAY, REPEAT_EVERY, type PadState } from "./gamepad";

const idle = (): PadState => ({ buttons: Array(17).fill(false), axes: [0, 0, 0, 0] });
const press = (...ids: number[]): PadState => {
  const s = idle();
  for (const i of ids) s.buttons[i] = true;
  return s;
};

describe("manette", () => {
  it("un bouton déclenche une action à l'appui seulement", () => {
    const m = newMemory();
    expect(readPad(m, press(0), 0)).toEqual(["select"]);
    expect(readPad(m, press(0), 16)).toEqual([]);
    expect(readPad(m, idle(), 32)).toEqual([]);
    expect(readPad(m, press(1), 48)).toEqual(["back"]);
  });

  it("une direction maintenue se répète après un délai", () => {
    const m = newMemory();
    expect(readPad(m, press(15), 0)).toEqual(["right"]);
    expect(readPad(m, press(15), REPEAT_DELAY - 1)).toEqual([]);
    expect(readPad(m, press(15), REPEAT_DELAY)).toEqual(["right"]);
    expect(readPad(m, press(15), REPEAT_DELAY + REPEAT_EVERY - 1)).toEqual([]);
    expect(readPad(m, press(15), REPEAT_DELAY + REPEAT_EVERY)).toEqual(["right"]);
    expect(readPad(m, idle(), 2000)).toEqual([]);
  });

  it("le stick gauche avec zone morte", () => {
    const m = newMemory();
    expect(readPad(m, { buttons: Array(17).fill(false), axes: [0.3, 0] }, 0)).toEqual([]);
    expect(readPad(m, { buttons: Array(17).fill(false), axes: [0, -0.9] }, 10)).toEqual(["up"]);
  });

  it("Guide, ou Vue + Menu ensemble, bascule le mode", () => {
    expect(readPad(newMemory(), press(16), 0)).toEqual(["toggle"]);
    const m = newMemory();
    readPad(m, press(8), 0);
    expect(readPad(m, press(8, 9), 20)).toEqual(["toggle"]);
  });

  it("navigation dans des rangées de longueurs différentes", () => {
    const rows = [1, 8, 3];
    expect(moveFocus(rows, { row: 1, col: 6 }, "down")).toEqual({ row: 2, col: 2 });
    expect(moveFocus(rows, { row: 1, col: 6 }, "up")).toEqual({ row: 0, col: 0 });
    expect(moveFocus(rows, { row: 1, col: 7 }, "right")).toEqual({ row: 1, col: 7 });
    expect(moveFocus(rows, { row: 0, col: 0 }, "left")).toEqual({ row: 0, col: 0 });
    expect(moveFocus(rows, { row: 2, col: 0 }, "nextRow")).toEqual({ row: 2, col: 0 });
  });
});
