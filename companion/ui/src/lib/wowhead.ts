import { create } from "zustand";

const KEY = "mrglglgl.wowheadLinks";

function saved(): boolean {
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}

/** Whether names in "What happened" link to Wowhead: a display preference kept on this computer,
 * like the paper and zoom. Off unless the player turns it on. */
export const useWowheadLinks = create<{
  on: boolean;
  set: (on: boolean) => void;
}>((set) => ({
  on: saved(),
  set: (on) => {
    try {
      localStorage.setItem(KEY, on ? "1" : "0");
    } catch {
      // Still applies for this session.
    }
    set({ on });
  },
}));

/** Wowhead's search for a name, e.g. https://www.wowhead.com/search?q=Fizsprocket%27s+Notes */
export function wowheadSearch(name: string): string {
  return `https://www.wowhead.com/search?q=${encodeURIComponent(name).replace(/%20/g, "+").replace(/'/g, "%27")}`;
}
