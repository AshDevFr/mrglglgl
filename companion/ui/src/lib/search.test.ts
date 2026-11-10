import { describe, expect, it } from "vitest";
import { fold, type SearchDoc, searchJournal } from "./search";

const docs: SearchDoc[] = [
  {
    day: "2026-11-01",
    kind: "day",
    source: "entry",
    text: "The plains of Mulgore rolled out under a sky the colour of old parchment.",
  },
  {
    day: "2026-11-01",
    kind: "day",
    source: "facts",
    text: "Visited Mulgore. Fished up a boot.",
  },
  {
    day: "2026-11-08",
    kind: "day",
    source: "facts",
    text: "Slew Hogger. Visited Mulgore and Thunder Bluff.",
  },
  {
    day: "2026-11-02",
    kind: "week",
    source: "entry",
    text: "A week that began on the road to Red Rocks and ended in a stream.",
  },
  {
    day: "2026-11-05",
    kind: "day",
    source: "entry",
    text: "Elle a trouvé la Forêt d'Elwynn très calme.",
  },
];

describe("fold", () => {
  it("drops case and accents without changing the length", () => {
    expect(fold("Forêt d'Élwynn")).toBe("foret d'elwynn");
    expect(fold("Forêt d'Élwynn")).toHaveLength("Forêt d'Élwynn".length);
  });
});

describe("searchJournal", () => {
  it("finds days by their entry or what happened, newest first", () => {
    const hits = searchJournal(docs, "mulgore");
    expect(hits.map((h) => [h.doc.day, h.doc.source])).toEqual([
      ["2026-11-08", "facts"],
      // The entry matched, so the same day's notes are not listed again.
      ["2026-11-01", "entry"],
    ]);
  });

  it("ignores case and accents in the query and the text alike", () => {
    expect(searchJournal(docs, "FORET")[0].doc.day).toBe("2026-11-05");
    expect(searchJournal(docs, "forêt")[0].doc.day).toBe("2026-11-05");
  });

  it("cuts a snippet around the match, keeping the text as written", () => {
    const [hit] = searchJournal(docs, "old parch");
    expect(hit.match).toBe("old parch");
    expect(`${hit.before}${hit.match}${hit.after}`).toContain(
      "the colour of old parchment.",
    );
    const [accented] = searchJournal(docs, "foret");
    expect(accented.match).toBe("Forêt");
  });

  it("finds a week's chapter, opened on its Monday", () => {
    const [hit] = searchJournal(docs, "red rocks");
    expect(hit.doc).toMatchObject({ day: "2026-11-02", kind: "week" });
  });

  it("asks for at least two letters, and stops at the limit", () => {
    expect(searchJournal(docs, " m ")).toEqual([]);
    expect(searchJournal(docs, "the", 1)).toHaveLength(1);
  });
});
