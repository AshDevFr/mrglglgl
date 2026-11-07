import i18n from "i18next";
import { describe, expect, it } from "vitest";
import { syntheticJournal } from "@/test/render";
import { articleEn, countedName, pluralEn } from "./grammar";
import { type DayFacts, dayFacts } from "./journal";
import { daySentences } from "./sentences";

function facts(overrides: Partial<DayFacts>): DayFacts {
  return {
    levelFrom: null,
    levelTo: null,
    leveledUp: false,
    zones: [],
    subzones: [],
    discovered: [],
    accepted: [],
    acceptedUnnamed: 0,
    turnedIn: [],
    turnedInUnnamed: 0,
    questTags: {},
    kills: [],
    notable: [],
    murlocs: 0,
    anonKills: 0,
    deaths: [],
    fish: 0,
    boots: 0,
    catches: [],
    loot: [],
    seenLoot: [],
    companions: [],
    bosses: [],
    skills: [],
    crafted: [],
    gathered: [],
    money: {
      earned: [],
      spent: [],
      balStart: null,
      balEnd: null,
      auctions: [],
    },
    chat: [],
    achievements: [],
    screenshots: 0,
    onlyFished: false,
    nothingHappened: false,
    compacted: false,
    ...overrides,
  };
}

function say(overrides: Partial<DayFacts>): string[] {
  return daySentences(facts(overrides), i18n.t, "en");
}

const kill = (
  name: string | null,
  count: number,
  classification = "normal",
) => ({
  name,
  npcId: 1,
  count,
  classification,
  murloc: false,
});

describe("English names", () => {
  it("pluralises creature and item names", () => {
    expect(pluralEn("Prairie Wolf")).toBe("Prairie Wolves");
    expect(pluralEn("Carving Knife")).toBe("Carving Knives");
    expect(pluralEn("Defias Footman")).toBe("Defias Footmen");
    expect(pluralEn("Murloc Streamrunner")).toBe("Murloc Streamrunners");
    expect(pluralEn("Harpy")).toBe("Harpies");
    expect(pluralEn("Lynx")).toBe("Lynxes");
    expect(pluralEn("Raw Brilliant Smallfish")).toBe("Raw Brilliant Smallfish");
    expect(pluralEn("Old Boot")).toBe("Old Boots");
  });

  it("chooses a or an, and counts", () => {
    expect(articleEn("Old Boot")).toBe("an");
    expect(articleEn("Prairie Wolf")).toBe("a");
    expect(countedName("en", "Old Boot", 1)).toBe("an Old Boot");
    expect(countedName("en", "Prairie Wolf", 23)).toBe("23 Prairie Wolves");
    expect(countedName("fr", "Loup des prairies", 23)).toBe(
      "23 × Loup des prairies",
    );
  });
});

describe("day sentences", () => {
  it("says where the day was spent", () => {
    expect(say({ zones: ["Mulgore"] })).toEqual(["Spent the day in Mulgore."]);
    expect(say({ zones: ["Mulgore", "The Barrens"] })).toEqual([
      "Travelled through Mulgore and The Barrens.",
    ]);
  });

  it("lists discoveries, a level and quests", () => {
    expect(say({ discovered: ["Thunder Bluff"] })).toEqual([
      "Discovered Thunder Bluff.",
    ]);
    expect(say({ leveledUp: true, levelTo: 6 })).toEqual(["Reached level 6."]);
    expect(say({ leveledUp: false, levelTo: 6 })).toEqual([
      "Nothing to report.",
    ]);
    expect(
      say({
        accepted: ["The Ravaged Caravan"],
        turnedIn: ["The Hunt Continues"],
      }),
    ).toEqual(["Took on The Ravaged Caravan.", "Finished The Hunt Continues."]);
  });

  it("counts kills with plurals and articles", () => {
    expect(say({ kills: [kill("Prairie Wolf", 23)] })).toEqual([
      "Killed 23 Prairie Wolves.",
    ]);
    expect(say({ kills: [kill("Prairie Wolf", 1)] })).toEqual([
      "Killed a Prairie Wolf.",
    ]);
    expect(
      say({
        kills: [
          kill("Prairie Wolf", 3),
          kill("Plainstrider", 2),
          kill("Murloc Streamrunner", 1),
        ],
      }),
    ).toEqual([
      "Killed 3 Prairie Wolves, 2 Plainstriders, and a Murloc Streamrunner.",
    ]);
  });

  it("still tells of kills whose names were never learned", () => {
    expect(say({ kills: [kill(null, 1)] })).toEqual([
      "Killed a creature the journal never learned the name of.",
    ]);
    expect(say({ kills: [kill("Prairie Wolf", 2), kill(null, 3)] })).toEqual([
      "Killed 2 Prairie Wolves and 3 creatures the journal never learned the names of.",
    ]);
  });

  it("names notable foes, singular and plural", () => {
    const cougar = kill("Flatland Cougar", 1, "elite");
    const boar = kill("Mottled Boar", 1, "rare");
    expect(say({ kills: [cougar], notable: [cougar] })[1]).toBe(
      "Among them was an elite Flatland Cougar.",
    );
    expect(say({ kills: [cougar, boar], notable: [cougar, boar] })[1]).toBe(
      "Among them were an elite Flatland Cougar and a rare Mottled Boar.",
    );
    expect(
      say({
        kills: [kill(null, 1, "elite")],
        notable: [kill(null, 1, "elite")],
      }),
    ).toHaveLength(1);
  });

  it("mentions foes the game kept nameless", () => {
    expect(say({ anonKills: 1 })).toEqual([
      "Also defeated a foe the game kept nameless.",
    ]);
    expect(say({ anonKills: 5 })).toEqual([
      "Also defeated 5 foes the game kept nameless.",
    ]);
  });

  it("tells each death, with what and where when known", () => {
    expect(
      say({
        deaths: [
          {
            killer: "Murloc Streamrunner",
            environment: null,
            place: "Red Rocks",
          },
          { killer: "Prairie Wolf", environment: null, place: null },
          { killer: null, environment: "Drowning", place: "Red Cloud Mesa" },
          { killer: null, environment: "Drowning", place: null },
          { killer: null, environment: "Something New", place: "Mulgore" },
          { killer: null, environment: null, place: "Mulgore" },
          { killer: null, environment: null, place: null },
        ],
      }),
    ).toEqual([
      "Died to a Murloc Streamrunner in Red Rocks.",
      "Died to a Prairie Wolf.",
      "Drowned in Red Cloud Mesa.",
      "Drowned.",
      "Died in Mulgore.",
      "Died in Mulgore.",
      "Died.",
    ]);
  });

  it("tells the catch, junk by name, and keeps the boot", () => {
    expect(
      say({
        fish: 7,
        boots: 1,
        catches: [
          {
            name: "Raw Brilliant Smallfish",
            itemId: 6291,
            count: 6,
            junk: false,
            boot: false,
          },
          { name: "Old Boot", itemId: 6150, count: 1, junk: true, boot: true },
        ],
      }),
    ).toEqual(["Caught 6 fish and an Old Boot.", "Kept the boot."]);
    expect(
      say({
        fish: 3,
        boots: 2,
        catches: [
          {
            name: "Raw Brilliant Smallfish",
            itemId: 6291,
            count: 1,
            junk: false,
            boot: false,
          },
          { name: "Old Boot", itemId: 6150, count: 2, junk: true, boot: true },
        ],
      }),
    ).toEqual(["Caught a fish and 2 Old Boots.", "Kept the boots."]);
  });

  it("handles catches with missing names or no list at all", () => {
    expect(
      say({
        fish: 2,
        catches: [{ name: null, itemId: 1, count: 2, junk: true, boot: false }],
      }),
    ).toEqual(["Caught 2 pieces of junk."]);
    expect(say({ fish: 4, catches: [] })).toEqual(["Caught 4 fish."]);
  });

  it("tells loot, company and bosses", () => {
    expect(
      say({
        loot: ["Carving Knife", "Ancient Shield"],
        companions: ["Tamsin Reedwater"],
        bosses: ["Hogger"],
      }),
    ).toEqual([
      "Found a Carving Knife and an Ancient Shield.",
      "Grouped with Tamsin Reedwater.",
      "Defeated Hogger.",
    ]);
  });

  it("tells professions, crafting and gathering", () => {
    expect(
      say({
        skills: [
          { name: "Mining", profession: true, from: 40, to: 41 },
          { name: "Axes", profession: false, from: 40, to: 43 },
          { name: "Daggers", profession: false, from: 12, to: 13 },
        ],
        crafted: [
          { name: "Linen Bandage", count: 5, via: null },
          { name: "Rough Sharpening Stone", count: 1, via: null },
        ],
        gathered: [
          { name: "Copper Ore", count: 3, via: "Mining" },
          { name: "Rough Stone", count: 2, via: "Mining" },
          { name: "Peacebloom", count: 1, via: "Herb Gathering" },
          { name: "Ruined Leather Scraps", count: 1, via: null },
        ],
      }),
    ).toEqual([
      "Mining rose from 40 to 41.",
      "Also practised Axes (40 to 43) and Daggers (12 to 13).",
      "Crafted 5 Linen Bandages and a Rough Sharpening Stone.",
      "Mined Copper Ore and Rough Stone.",
      "Picked Peacebloom.",
      "Gathered Ruined Leather Scraps.",
    ]);
  });

  it("gathers with an unfamiliar skill in plain words", () => {
    expect(
      say({ gathered: [{ name: "Thorium Ore", count: 1, via: "Minage" }] }),
    ).toEqual(["Gathered Thorium Ore."]);
  });

  it("marks a fishing-only day and a day where nothing happened", () => {
    expect(say({ onlyFished: true, fish: 2 }).at(-1)).toBe(
      "Spent the whole day fishing.",
    );
    expect(say({ nothingHappened: true, zones: ["Mulgore"] })).toEqual([
      "Nothing to report.",
    ]);
    expect(say({})).toEqual(["Nothing to report."]);
  });

  it("reads a real day from the shared fixture", () => {
    const sentences = daySentences(
      dayFacts(syntheticJournal, syntheticJournal.days["2026-11-08"]),
      i18n.t,
      "en",
    );
    expect(sentences).toContain("Spent the day in Mulgore.");
    expect(
      sentences.some((s) => s.startsWith("Killed 23 Prairie Wolves")),
    ).toBe(true);
    expect(sentences).toContain("Died to a Murloc Streamrunner in Red Rocks.");
    expect(sentences).toContain("Among them was an elite Flatland Cougar.");
    expect(sentences).toContain("Mining rose from 40 to 41.");
    expect(sentences).toContain("First Aid rose from 30 to 35.");
    expect(sentences).toContain("Crafted 5 Linen Bandages.");
    expect(sentences).toContain("Mined Copper Ore and Rough Stone.");
  });

  it("reads a compacted day from its digest", () => {
    const sentences = daySentences(
      dayFacts(syntheticJournal, syntheticJournal.days["2026-11-01"]),
      i18n.t,
      "en",
    );
    expect(sentences).toContain("Drowned in Red Cloud Mesa.");
    expect(sentences).toContain("Kept the boot.");
    expect(
      sentences.some((s) => s.startsWith("Caught a fish and an Old Boot")),
    ).toBe(true);
  });
});

describe("quests and skills on the page", () => {
  const ev = (kind: object) => ({
    time: 0,
    zone: null,
    subzone: null,
    mapId: null,
    level: null,
    kind,
  });
  const day = (events: object[], skills = {}) =>
    ({
      events,
      kills: {},
      anonKills: 0,
      skills,
      crafted: {},
      gathered: {},
      digest: null,
    }) as unknown as Parameters<typeof dayFacts>[1];
  const journal = {
    ...syntheticJournal,
    quests: {
      1: {
        title: "Fizsprocket's Notes",
        text: null,
        last: null,
        level: 9,
        kind: "elite",
      },
      2: {
        title: "Swoop Hunting",
        text: null,
        last: null,
        level: null,
        kind: null,
      },
    },
  };

  it("tells a quest finished the same day once, and counts the untitled ones", () => {
    const text = daySentences(
      dayFacts(
        journal,
        day([
          ev({ kind: "questAccept", questId: 1 }),
          ev({ kind: "questAccept", questId: 2 }),
          ev({ kind: "questTurnin", questId: 1, xp: 1, money: 1 }),
          ev({ kind: "questTurnin", questId: 764, xp: 1, money: 1 }),
          ev({ kind: "questTurnin", questId: 765, xp: 1, money: 1 }),
        ]),
      ),
      i18n.t,
      "en",
    );
    expect(text).toContain("Took on Swoop Hunting.");
    expect(text).toContain("Finished Fizsprocket's Notes and 2 other quests.");
    expect(text.join(" ")).not.toContain("#");
  });

  it("says how many quests were finished when none has a title", () => {
    const text = daySentences(
      dayFacts(
        journal,
        day([ev({ kind: "questTurnin", questId: 764, xp: 1, money: 1 })]),
      ),
      i18n.t,
      "en",
    );
    expect(text).toContain("Finished 1 quest.");
  });

  it("tells a profession the game lists twice once", () => {
    const skill = (name: string, cat: string, from: number, to: number) => ({
      name,
      category: cat,
      from,
      to,
    });
    const text = daySentences(
      dayFacts(
        journal,
        day([], {
          182: skill("Herbalism", "Professions", 30, 37),
          2944: skill("Herbalism", "Professions", 30, 37),
          393: skill("Skinning", "Professions", 80, 83),
          2947: skill("Skinning", "Professions", 80, 83),
          136: skill("Staves", "Weapon Skills", 31, 36),
        }),
      ),
      i18n.t,
      "en",
    ).join(" ");
    expect(text.match(/Herbalism/g)).toHaveLength(1);
    expect(text.match(/Skinning/g)).toHaveLength(1);
    expect(text).toContain("Also practised Staves (31 to 36).");
  });

  it("tells an item seen in a group loot window as seen, not found", () => {
    expect(
      say({ seenLoot: ["Gnarled Staff"], loot: ["Carving Knife"] }),
    ).toEqual(["Saw a Gnarled Staff drop.", "Found a Carving Knife."]);
  });
});

describe("money", () => {
  it("tells money earned and spent with where most of it went, and auction sales", () => {
    const day = syntheticJournal.days["2026-11-08"];
    const told = daySentences(dayFacts(syntheticJournal, day), i18n.t, "en");
    expect(told).toContain("Earned 22s 67c, mostly at the auction house.");
    expect(told).toContain("Spent 2s 23c, mostly on repairs.");
    expect(told).toContain("Sold Copper Ore at the auction house.");
  });

  it("leaves out where when no source holds half", () => {
    expect(
      say({
        money: {
          earned: [
            { source: "loot", copper: 40 },
            { source: "quest", copper: 35 },
            { source: "vendor", copper: 30 },
          ],
          spent: [],
          balStart: null,
          balEnd: null,
          auctions: [],
        },
      }),
    ).toEqual(["Earned 1s 5c."]);
  });
});

describe("chat", () => {
  it("never tells chat in the sentences, which the website and exports use", () => {
    const day = syntheticJournal.days["2026-11-08"];
    const told = daySentences(dayFacts(syntheticJournal, day), i18n.t, "en");
    expect(told.join(" ")).not.toMatch(/wolves grow bold|thin the pack/);
  });
});

describe("Legacy Challenges", () => {
  it("tells a challenge completed by name", () => {
    const day = syntheticJournal.days["2026-11-08"];
    const told = daySentences(dayFacts(syntheticJournal, day), i18n.t, "en");
    expect(told).toContain(
      'Completed the Legacy Challenge "Explore Moonglade".',
    );
  });
});
