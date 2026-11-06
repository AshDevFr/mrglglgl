import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(join(__dirname, "paper.css"), "utf8");

/** The colour tokens of one block of the stylesheet. */
function tokens(selector: string): Record<string, string> {
  const start = css.indexOf(`${selector} {`);
  const block = css.slice(start, css.indexOf("}", start));
  return Object.fromEntries(
    [...block.matchAll(/--([\w-]+):\s*(#[0-9a-fA-F]{6})/g)].map((m) => [
      m[1],
      m[2],
    ]),
  );
}

const light = tokens(":root");
const dark = { ...light, ...tokens('[data-mantine-color-scheme="dark"]') };

/** WCAG relative luminance of an sRGB colour. */
function luminance(hex: string): number {
  const channel = (i: number) => {
    const c = Number.parseInt(hex.slice(1 + i * 2, 3 + i * 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(0) + 0.7152 * channel(1) + 0.0722 * channel(2);
}

/** WCAG contrast ratio between two colours. */
function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

// Text pairs need 4.5:1 (WCAG AA for body text). The stamp ink also colours the margin note,
// the written-day dot and focus rings, so it is held to the same bar.
const TEXT_PAIRS: [string, string][] = [
  ["ink", "page"],
  ["ink", "desk"],
  ["ink-faded", "page"],
  ["ink-faded", "desk"],
  ["stamp", "page"],
  ["stamp", "desk"],
];

describe("contrast", () => {
  it("computes the WCAG ratio", () => {
    expect(contrast("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrast("#777777", "#ffffff")).toBeCloseTo(4.48, 2);
  });

  for (const [name, theme] of [
    ["light", light],
    ["dark", dark],
  ] as const) {
    it(`keeps every text colour readable in the ${name} theme`, () => {
      const failing = TEXT_PAIRS.filter(([fg, bg]) => {
        expect(theme[fg], `${name} --${fg}`).toBeDefined();
        expect(theme[bg], `${name} --${bg}`).toBeDefined();
        return contrast(theme[fg], theme[bg]) < 4.5;
      }).map(
        ([fg, bg]) =>
          `${fg} on ${bg}: ${contrast(theme[fg], theme[bg]).toFixed(2)}`,
      );
      expect(failing).toEqual([]);
    });
  }
});
