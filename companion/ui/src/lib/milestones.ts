import type { Award } from "@/bindings/Award";

/** How many stamps of one day are pressed on its page, and told to a narrator: the rarest. The
 * app's core uses the same number. */
export const PRESSED_PER_DAY = 3;

/** A day's stamps split in two: the rarest few, pressed on the page, and the rest, listed at its
 * end. Among equals the journal's list order, which awards already come in, is kept. */
export function splitStamps(awards: Award[]): {
  pressed: Award[];
  more: Award[];
} {
  const ranked = awards
    .map((award, index) => ({ award, index }))
    .sort((a, b) => b.award.rarity - a.award.rarity || a.index - b.index)
    .map((r) => r.award);
  return {
    pressed: ranked.slice(0, PRESSED_PER_DAY),
    more: ranked.slice(PRESSED_PER_DAY),
  };
}
