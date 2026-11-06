import { beforeEach, describe, expect, it, vi } from "vitest";

const setZoom = vi.hoisted(() => vi.fn(() => Promise.resolve()));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ setZoom }),
}));

import { applyZoom, savedZoom, stepZoom, ZOOMS } from "./zoom";

beforeEach(() => {
  localStorage.clear();
  setZoom.mockClear();
});

describe("interface zoom", () => {
  it("starts at 100% and steps through the offered sizes, stopping at the ends", () => {
    expect(savedZoom()).toBe(100);
    expect(stepZoom(100, 1)).toBe(110);
    expect(stepZoom(100, -1)).toBe(90);
    expect(stepZoom(ZOOMS[ZOOMS.length - 1], 1)).toBe(ZOOMS[ZOOMS.length - 1]);
    expect(stepZoom(ZOOMS[0], -1)).toBe(ZOOMS[0]);
    expect(stepZoom(125, 0)).toBe(100);
  });

  it("applies a size to the window and remembers it", async () => {
    await applyZoom(125);
    expect(setZoom).toHaveBeenCalledWith(1.25);
    expect(savedZoom()).toBe(125);
  });

  it("ignores a remembered size it does not offer", () => {
    localStorage.setItem("mrglglgl.zoom", "333");
    expect(savedZoom()).toBe(100);
  });
});
