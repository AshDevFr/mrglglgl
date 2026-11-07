import i18n from "i18next";
import { describe, expect, it } from "vitest";
import { syntheticJournal } from "@/test/render";
import { dayFactRows } from "./factList";
import { dayFacts, questTag } from "./journal";
import { wowheadSearch } from "./wowhead";

/** The English catalogue, loaded before each test by the test setup. */
const t = ((...args: Parameters<typeof i18n.t>) =>
  i18n.t(...args)) as unknown as typeof i18n.t;

describe("quest tags", () => {
  it("writes a quest's level and kind the way players do", () => {
    expect(questTag(9, null)).toBe("[9]");
    expect(questTag(9, "elite")).toBe("[9+]");
    expect(questTag(18, "dungeon")).toBe("[18D]");
    expect(questTag(40, "raid")).toBe("[40R]");
    expect(questTag(12, "group")).toBe("[12G]");
    expect(questTag(null, "elite")).toBeNull();
  });

  it("shows the tag beside the quest, and searches the bare title", () => {
    const facts = dayFacts(
      syntheticJournal,
      syntheticJournal.days["2026-11-08"],
    );
    const quests = dayFactRows(facts, t).find((r) => r.key === "turnedIn");
    expect(quests?.items).toEqual([
      { text: "The Ravaged Caravan [6]", name: "The Ravaged Caravan" },
    ]);
  });

  it("never links other players' names", () => {
    const facts = dayFacts(
      syntheticJournal,
      syntheticJournal.days["2026-11-08"],
    );
    const group = dayFactRows(facts, t).find((r) => r.key === "companions");
    expect(group?.items.every((i) => i.name === undefined)).toBe(true);
  });
});

describe("Wowhead search", () => {
  it("builds the search address Wowhead uses", () => {
    expect(wowheadSearch("Fizsprocket's Notes")).toBe(
      "https://www.wowhead.com/search?q=Fizsprocket%27s+Notes",
    );
  });
});
