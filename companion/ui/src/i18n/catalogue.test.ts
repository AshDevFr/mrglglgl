import { readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import milestoneIds from "@/test/data/milestone-ids.json";
import en from "./locales/en.json";

const catalogue = en as unknown as Record<string, Record<string, unknown>>;

describe("the English catalogue", () => {
  it("names and describes every milestone the backend can award", () => {
    for (const id of milestoneIds) {
      const entry = catalogue.milestones[id] as {
        name?: string;
        description?: string;
      };
      expect(entry?.name, id).toBeTruthy();
      expect(entry?.description, id).toBeTruthy();
    }
  });

  it("explains every status a character can have", () => {
    const status = (catalogue.sync as Record<string, Record<string, string>>)
      .status;
    for (const key of ["older", "newer", "unreadable"]) {
      expect(status[key], key).toBeTruthy();
    }
  });
});

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name: string) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

describe("bundled assets", () => {
  it("ships no images, sounds or fonts of its own in the frontend source", () => {
    // The look is CSS; the reading font comes from an open-licence package. Any asset added
    // here must be the project's own, never extracted from the game.
    const root = join(import.meta.dirname, "..");
    const media = files(root).filter((f) =>
      /\.(png|jpe?g|gif|webp|blp|tga|svg|mp3|ogg|wav|ttf|otf|woff2?)$/i.test(f),
    );
    expect(media).toEqual([]);
  });
});
