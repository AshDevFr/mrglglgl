import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SiteSettings } from "@/bindings/SiteSettings";
import type { Story } from "@/bindings/Story";
import { defaultSite, syntheticAwards, syntheticJournal } from "@/test/render";
import { buildSiteData } from "./siteData";

function story(
  id: number,
  kind: Story["kind"],
  day: string,
  text: string,
  extra: Partial<Story> = {},
): Story {
  return {
    id,
    characterId: 1,
    kind,
    startDay: day,
    endDay: day,
    narrator: "murloc_scribe",
    language: "en",
    provider: "local",
    model: "m",
    promptHash: "h",
    text,
    createdAt: 0,
    check: "passed",
    checkReason: null,
    status: "accepted",
    note: null,
    partial: false,
    ...extra,
  };
}

const commands = vi.hoisted(() => ({
  stories: [] as unknown[],
}));

vi.mock("@/api/commands", () => ({
  listCharacters: async () => [
    {
      character: {
        id: 1,
        name: "Wren",
        surname: "Hollowbrook",
        realm: "Test Realm",
        race: "Tauren",
        class: "HUNTER",
        characterFolder: "Wren-Hollowbrook",
      },
      archived: false,
      milestones: 6,
      toReview: 0,
    },
  ],
  characterJournal: async () => syntheticJournal,
  characterStories: async () => commands.stories,
  characterMilestones: async () => syntheticAwards,
  characterScreenshots: async () => ({
    "2026-11-08": [
      {
        path: "/shots/WoWScrnShot_110826_120000.jpg",
        taken: 1794164491,
        moment: "levelUp",
      },
    ],
  }),
  siteStandInNames: async () => [{ real: "Tamsin", alias: "Sable" }],
}));

const site: SiteSettings = {
  ...defaultSite,
  enabled: true,
  folder: "/sites/wren",
  title: "Wren's travels",
  characters: [1],
};

beforeEach(() => {
  commands.stories = [
    story(1, "day", "2026-11-08", "Tamsin and I took the road to Red Rocks."),
    story(2, "day", "2026-11-09", "A morning so far.", { partial: true }),
    story(3, "week", "2026-11-02", "A week with Tamsin Reedwater by my side."),
  ];
});

const all = (value: unknown) => JSON.stringify(value);

describe("site data", () => {
  it("tells finished weeks as posts, with entries, facts, stamps and captioned photos", async () => {
    const data = await buildSiteData(site, { today: "2026-11-10" });
    expect(data.title).toBe("Wren's travels");
    const wren = data.characters[0];
    expect(wren.slug).toBe("wren-hollowbrook");
    expect(wren.summary).toBe("Tauren Hunter, level 5");
    expect(wren.posts.map((p) => p.slug).sort()).toEqual([
      "2026-w44",
      "2026-w45",
      "2026-w46",
    ]);
    const week = wren.posts.find((p) => p.slug === "2026-w45");
    const day = week?.days.find((d) => d.date === "2026-11-08");
    expect(day?.written).toBe(true);
    expect(day?.html).toContain('<p class="entry">');
    expect(day?.stamps.length).toBeGreaterThan(0);
    expect(day?.stamps[0].svg).toContain("ink-stamp");
    expect(day?.photos).toEqual([
      {
        source: "/shots/WoWScrnShot_110826_120000.jpg",
        file: "2026-11-08-1.jpg",
        caption: expect.stringMatching(/^Level up in Red Rocks/),
      },
    ]);
    expect(week?.chapter_html).toContain("by my side");
    expect(week?.excerpt.length).toBeGreaterThan(10);
    expect(wren.milestones.length).toBe(syntheticAwards.length);
  });

  it("never shows today, nor an entry told so far", async () => {
    const data = await buildSiteData(site, { today: "2026-11-10" });
    const days = data.characters[0].posts.flatMap((p) => p.days);
    expect(days.map((d) => d.date)).not.toContain("2026-11-10");
    const monday = days.find((d) => d.date === "2026-11-09");
    expect(monday?.written).toBe(false);
    expect(all(data)).not.toContain("A morning so far.");
  });

  it("gives other players stand-in names everywhere, and leaves realms out", async () => {
    const data = await buildSiteData(site, { today: "2026-11-10" });
    const text = all(data);
    expect(text).not.toContain("Tamsin");
    expect(text).not.toContain("Reedwater");
    expect(text).not.toContain("Test Realm");
    expect(text).toContain("Sable");
  });

  it("keeps real names when the player says so, still without realms", async () => {
    const data = await buildSiteData(
      { ...site, standInNames: false },
      { today: "2026-11-10" },
    );
    const text = all(data);
    expect(text).toContain("Tamsin");
    expect(text).not.toContain("Reedwater");
    const shown = await buildSiteData(
      { ...site, standInNames: false, hideRealm: false },
      { today: "2026-11-10" },
    );
    expect(all(shown)).toContain("Tamsin Reedwater");
  });
});
