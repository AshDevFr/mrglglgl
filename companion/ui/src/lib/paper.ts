/** The papers the journal can be written on: tide-marked rag paper, or parchment. */
export const PAPERS = ["tide", "parchment"] as const;
export type Paper = (typeof PAPERS)[number];

const KEY = "mrglglgl.paper";

/** The paper chosen on this computer; a display preference, kept in the window's own storage. */
export function savedPaper(): Paper {
  try {
    const value = localStorage.getItem(KEY);
    return PAPERS.find((p) => p === value) ?? "tide";
  } catch {
    return "tide";
  }
}

/** Lays the app on `paper` and remembers it. The stylesheet keys its colours on the attribute. */
export function applyPaper(paper: Paper): void {
  document.documentElement.dataset.paper = paper;
  try {
    localStorage.setItem(KEY, paper);
  } catch {
    // The paper still applies for this session.
  }
}
