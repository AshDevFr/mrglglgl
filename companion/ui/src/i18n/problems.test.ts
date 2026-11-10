import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import i18n from "i18next";
import { describe, expect, it } from "vitest";
import en from "./locales/en.json";
import { PROBLEMS, problem } from "./problems";

/** Every catalogue key, flattened: "writing.blocked.noModel". */
function keys(node: unknown, prefix = ""): string[] {
  if (typeof node !== "object" || node === null) return [prefix];
  return Object.entries(node).flatMap(([k, v]) =>
    keys(v, prefix ? `${prefix}.${k}` : k),
  );
}

const ALL = keys(en);
const text = (key: string) =>
  key
    .split(".")
    .reduce<unknown>((n, k) => (n as Record<string, unknown>)?.[k], en);

/** Keys that look like an error or a reason the journal waits, by where and how they are named. */
const ERROR_LIKE =
  /(^errors\.|[eE]rror$|[fF]ailed$|notEnough|^sync\.status\.|^writing\.blocked\.|^writing\.reason\.|^writing\.outcome\.|setAside$|foundNone$)/;

describe("the problem inventory", () => {
  it("gives every problem a plain next step", () => {
    for (const key of PROBLEMS) {
      const message = text(key);
      const next = text(`${key}Next`);
      expect(typeof message, key).toBe("string");
      expect(typeof next, `${key}Next`).toBe("string");
      expect((next as string).trim().length, `${key}Next`).toBeGreaterThan(10);
      expect(next, `${key}Next ends a sentence`).toMatch(/[.!]$/);
      expect(next, `${key}Next adds something`).not.toBe(message);
    }
  });

  it("lists every error-like message in the catalogue", () => {
    const unlisted = ALL.filter(
      (k) =>
        !k.endsWith("Next") &&
        ERROR_LIKE.test(k) &&
        !(PROBLEMS as readonly string[]).includes(k),
    );
    expect(unlisted).toEqual([]);
  });

  it("has no next step without its problem", () => {
    const orphans = ALL.filter(
      (k) =>
        k.endsWith("Next") &&
        !(PROBLEMS as readonly string[]).includes(k.slice(0, -"Next".length)),
    );
    expect(orphans).toEqual([]);
  });

  it("is shown with its next step everywhere", () => {
    // A component that calls t() with a problem key directly would show it without the step.
    const sources: string[] = [];
    const walk = (dir: string) => {
      for (const name of readdirSync(dir)) {
        const path = join(dir, name);
        if (statSync(path).isDirectory()) walk(path);
        else if (/\.tsx?$/.test(name) && !/\.test\.tsx?$/.test(name))
          sources.push(path);
      }
    };
    walk(join(__dirname, ".."));
    const prefixes = [
      "sync.status.",
      "writing.blocked.",
      "writing.reason.",
      "writing.outcome.",
    ];
    const offenders: string[] = [];
    for (const path of sources) {
      const code = readFileSync(path, "utf8");
      for (const key of PROBLEMS) {
        if (
          new RegExp(
            `\\bt\\(\\s*["'\`]${key.replace(/\./g, "\\.")}["'\`]`,
          ).test(code)
        ) {
          offenders.push(`${path}: ${key}`);
        }
      }
      for (const prefix of prefixes) {
        if (code.includes(`t(\`${prefix}`))
          offenders.push(`${path}: ${prefix}*`);
      }
    }
    expect(offenders).toEqual([]);
  });

  it("renders the message and the step together", () => {
    expect(problem(i18n.t, "writing.blocked.noModel")).toBe(
      "No writing model is downloaded yet. Download one in Settings, AI, under Writing models.",
    );
    expect(problem(i18n.t, "settings.error", { message: "disk full" })).toBe(
      "The settings could not be saved: disk full. Check that the folder above exists, then save again.",
    );
  });
});
