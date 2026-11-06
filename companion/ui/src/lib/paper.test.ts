import { beforeEach, describe, expect, it } from "vitest";
import { applyPaper, PAPERS, savedPaper } from "./paper";

beforeEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.paper;
});

describe("paper", () => {
  it("starts on the tide-marked paper and remembers a choice", () => {
    expect(savedPaper()).toBe("tide");
    applyPaper("parchment");
    expect(document.documentElement.dataset.paper).toBe("parchment");
    expect(savedPaper()).toBe("parchment");
    expect(PAPERS).toEqual(["tide", "parchment"]);
  });

  it("ignores a remembered paper it does not offer", () => {
    localStorage.setItem("mrglglgl.paper", "velvet");
    expect(savedPaper()).toBe("tide");
  });
});
