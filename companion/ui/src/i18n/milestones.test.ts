import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import en from "./locales/en.json";
import fr from "./locales/fr.json";

type Names = Record<string, { name?: string; description?: string } | string>;

describe("milestone names", () => {
  it("names and describes every milestone in the data file, in each language", () => {
    // Tests run from the ui folder.
    const milestones = readFileSync(
      "../crates/mrglglgl-core/data/milestones.toml",
      "utf8",
    );
    const ids = [...milestones.matchAll(/^id = "(\w+)"/gm)].map((m) => m[1]);
    expect(ids.length).toBeGreaterThan(10);
    for (const catalogue of [en.milestones, fr.milestones] as Names[]) {
      for (const id of ids) {
        const entry = catalogue[id];
        expect(typeof entry === "object" && entry.name, id).toBeTruthy();
        expect(typeof entry === "object" && entry.description, id).toBeTruthy();
      }
    }
  });
});
