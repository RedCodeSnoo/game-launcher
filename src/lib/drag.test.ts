import { describe, expect, it } from "vitest";
import { passedThreshold, reorder } from "./drag";

describe("glisser-déposer", () => {
  it("un petit mouvement reste un clic", () => {
    expect(passedThreshold({ x: 10, y: 10 }, { x: 13, y: 12 })).toBe(false);
    expect(passedThreshold({ x: 10, y: 10 }, { x: 16, y: 10 })).toBe(true);
  });
  it("réordonne une liste", () => {
    expect(reorder(["a", "b", "c", "d"], 0, 2)).toEqual(["b", "c", "a", "d"]);
    expect(reorder(["a", "b", "c", "d"], 3, 0)).toEqual(["d", "a", "b", "c"]);
    expect(reorder(["a", "b"], 1, 1)).toEqual(["a", "b"]);
    expect(reorder(["a", "b"], 5, 0)).toEqual(["a", "b"]);
  });
});
