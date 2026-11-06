import { describe, expect, it } from "vitest";
import type { CharacterSummary } from "@/bindings/CharacterSummary";
import type { StoredCharacter } from "@/bindings/StoredCharacter";
import { syntheticJournal } from "@/test/render";
import {
  dayFacts,
  daysNewestFirst,
  displayName,
  groupCharacters,
} from "./journal";

function character(overrides: Partial<StoredCharacter>): CharacterSummary {
  return {
    archived: false,
    milestones: 0,
    character: {
      id: 1,
      flavor: "_forever_",
      account: "ACCOUNT1",
      realmFolder: "93",
      characterFolder: "Wren-Hollowbrook",
      journalPath: "/wow/Mrglglgl.lua",
      name: "Wren",
      surname: "Hollowbrook",
      realm: "Test Realm",
      race: "Tauren",
      class: "HUNTER",
      faction: "Horde",
      locale: "enUS",
      lastSynced: 1794400000,
      status: null,
      ...overrides,
    },
  };
}

describe("displayName", () => {
  it("uses first name and surname, or the folder before the journal is read", () => {
    expect(displayName(character({}).character)).toBe("Wren Hollowbrook");
    expect(
      displayName(character({ name: null, surname: null }).character),
    ).toBe("Wren-Hollowbrook");
  });
});

describe("groupCharacters", () => {
  it("groups by account, then realm, sorted by name", () => {
    const groups = groupCharacters([
      character({ id: 1, name: "Wren", account: "B" }),
      character({ id: 2, name: "Tamsin", surname: "Reedwater", account: "A" }),
      character({ id: 3, name: "Brin", surname: "Stonehoof", account: "A" }),
      character({
        id: 4,
        name: "Rowan",
        surname: "Elm",
        account: "A",
        realm: null,
        realmFolder: "93",
      }),
    ]);
    expect(groups.map((g) => g.account)).toEqual(["A", "B"]);
    expect(groups[0].realms.map((r) => r.realm)).toEqual(["93", "Test Realm"]);
    expect(groups[0].realms[1].characters.map((c) => c.character.name)).toEqual(
      ["Brin", "Tamsin"],
    );
  });
});

describe("dayFacts", () => {
  it("reads a raw day's events, kills and quest titles", () => {
    const facts = dayFacts(
      syntheticJournal,
      syntheticJournal.days["2026-11-08"],
    );
    expect(facts.compacted).toBe(false);
    expect(facts.turnedIn).toContain("The Ravaged Caravan");
    expect(facts.kills[0]).toMatchObject({ name: "Prairie Wolf", count: 23 });
    expect(facts.murlocs).toBe(1);
    expect(facts.notable.map((k) => k.name)).toEqual(["Flatland Cougar"]);
    expect(facts.deaths).toContainEqual({
      killer: "Murloc Streamrunner",
      environment: null,
    });
    expect(facts.companions).toEqual(["Tamsin Reedwater"]);
    expect(facts.bosses.length).toBeGreaterThan(0);
  });

  it("reads a compacted day from its digest", () => {
    const facts = dayFacts(
      syntheticJournal,
      syntheticJournal.days["2026-11-01"],
    );
    expect(facts.compacted).toBe(true);
    expect(facts.boots).toBe(1);
    expect(facts.deaths).toEqual([{ killer: null, environment: "Drowning" }]);
    expect(facts.catches.find((c) => c.boot)?.name).toBe("Old Boot");
  });

  it("knows a fishing-only day and a quiet day", () => {
    expect(
      dayFacts(syntheticJournal, syntheticJournal.days["2026-11-09"])
        .onlyFished,
    ).toBe(true);
    expect(
      dayFacts(syntheticJournal, syntheticJournal.days["2026-11-10"])
        .nothingHappened,
    ).toBe(true);
  });

  it("lists days newest first", () => {
    expect(daysNewestFirst(syntheticJournal)).toEqual([
      "2026-11-10",
      "2026-11-09",
      "2026-11-08",
      "2026-11-01",
    ]);
  });
});
