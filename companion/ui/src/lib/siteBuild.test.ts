import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { defaultSite } from "@/test/render";

const api = vi.hoisted(() => ({
  buildSite: vi.fn(async () => ({ written: 3, unchanged: 2, removed: 1 })),
  getSettings: vi.fn(),
}));
vi.mock("@/api/commands", () => api);
vi.mock("@/lib/siteData", () => ({
  buildSiteData: vi.fn(async () => ({ title: "t", characters: [] })),
}));

import { lastBuild, rebuildSite, scheduleRebuild } from "./siteBuild";

const on = {
  ...defaultSite,
  enabled: true,
  folder: "/sites/wren",
  characters: [1],
};

beforeEach(() => {
  vi.useFakeTimers();
  api.buildSite.mockClear();
});
afterEach(() => vi.useRealTimers());

describe("site builds", () => {
  it("records what the last build did", async () => {
    await rebuildSite(on);
    expect(lastBuild()).toMatchObject({
      report: { written: 3, unchanged: 2, removed: 1 },
      error: null,
    });
  });

  it("records a failed build in plain words", async () => {
    api.buildSite.mockRejectedValueOnce("the folder already has index.html");
    await rebuildSite(on);
    expect(lastBuild()).toMatchObject({
      report: null,
      error: "the folder already has index.html",
    });
  });

  it("rebuilds once, a few seconds after the last change, only when the site is on", async () => {
    api.getSettings.mockResolvedValue({ settings: { site: on } });
    scheduleRebuild();
    scheduleRebuild();
    scheduleRebuild();
    await vi.advanceTimersByTimeAsync(4000);
    expect(api.buildSite).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(2000);
    expect(api.buildSite).toHaveBeenCalledTimes(1);

    api.getSettings.mockResolvedValue({
      settings: { site: { ...on, enabled: false } },
    });
    scheduleRebuild();
    await vi.advanceTimersByTimeAsync(6000);
    expect(api.buildSite).toHaveBeenCalledTimes(1);
  });
});
