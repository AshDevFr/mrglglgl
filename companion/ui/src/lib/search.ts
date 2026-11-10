// Searching one character's journal: the written entries, the weeks' chapters and each day's
// "What happened", all already loaded by the journal page. Plain substring matching, which is
// what a player looking for "Hogger" or "boot" expects, without case or accents getting in the
// way (French names carry them, and players rarely type them).

/** One searchable text: a day's entry or notes, or a week's chapter. */
export interface SearchDoc {
  /** The day, or the week's Monday, which is where a chapter opens. */
  day: string;
  kind: "day" | "week";
  /** The narrator's text, or the plain notes of What happened. */
  source: "entry" | "facts";
  text: string;
}

/** A match, with the text around it as written. */
export interface SearchHit {
  doc: SearchDoc;
  before: string;
  match: string;
  after: string;
}

/** How much text a snippet keeps on each side of the match. */
const CONTEXT = 60;
/** Fewer letters match nearly everything, which helps no one. */
const MIN_QUERY = 2;
export const SEARCH_LIMIT = 100;

/** Lowercase without accents, one character for one, so positions in the folded text are
 * positions in the text as written. */
export function fold(text: string): string {
  let out = "";
  for (const c of text) {
    const plain = c.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
    // A character that folds to more or fewer than one keeps its place, unfolded.
    out += plain.length === c.length ? plain : c;
  }
  return out;
}

/** Cuts the text around a match at word boundaries, marking what was left out. */
function snippet(text: string, at: number, length: number) {
  let start = Math.max(0, at - CONTEXT);
  let end = Math.min(text.length, at + length + CONTEXT);
  if (start > 0) {
    const space = text.indexOf(" ", start);
    if (space !== -1 && space < at) start = space + 1;
  }
  if (end < text.length) {
    const space = text.lastIndexOf(" ", end);
    if (space > at + length) end = space;
  }
  return {
    before: (start > 0 ? "…" : "") + text.slice(start, at),
    match: text.slice(at, at + length),
    after: text.slice(at + length, end) + (end < text.length ? "…" : ""),
  };
}

/** Within a day, the entry first, then the week's chapter, then the notes. */
const RANK = {
  "day-entry": 0,
  "week-entry": 1,
  "day-facts": 2,
  "week-facts": 3,
};

/** Every text holding the query, newest day first. A day's notes are left out when its entry
 * already matched: the entry was written from them, so they would only say it again. */
export function searchJournal(
  docs: SearchDoc[],
  query: string,
  limit = SEARCH_LIMIT,
): SearchHit[] {
  const needle = fold(query.trim());
  if (needle.length < MIN_QUERY) return [];
  const hits: SearchHit[] = [];
  for (const doc of docs) {
    const at = fold(doc.text).indexOf(needle);
    if (at !== -1) hits.push({ doc, ...snippet(doc.text, at, needle.length) });
  }
  const entryDays = new Set(
    hits
      .filter((h) => h.doc.kind === "day" && h.doc.source === "entry")
      .map((h) => h.doc.day),
  );
  return hits
    .filter(
      (h) =>
        !(
          h.doc.kind === "day" &&
          h.doc.source === "facts" &&
          entryDays.has(h.doc.day)
        ),
    )
    .sort(
      (a, b) =>
        b.doc.day.localeCompare(a.doc.day) ||
        RANK[`${a.doc.kind}-${a.doc.source}`] -
          RANK[`${b.doc.kind}-${b.doc.source}`],
    )
    .slice(0, limit);
}
