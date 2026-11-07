import i18n from "i18next";
import { describe, expect, it } from "vitest";
import { initI18n } from "./index";
import en from "./locales/en.json";
import fr from "./locales/fr.json";

type Catalogue = { [key: string]: string | Catalogue };

function flat(catalogue: Catalogue, prefix = ""): Map<string, string> {
  const out = new Map<string, string>();
  for (const [key, value] of Object.entries(catalogue)) {
    if (typeof value === "string") out.set(prefix + key, value);
    else for (const [k, v] of flat(value, `${prefix}${key}.`)) out.set(k, v);
  }
  return out;
}

const placeholders = (text: string) =>
  (text.match(/{{\s*\w+\s*}}/g) ?? []).sort().join();

describe("i18n", () => {
  it("falls back to English for a language with no catalogue", async () => {
    await initI18n("de-DE");
    expect(i18n.t("app.title")).toBe("Mrglglgl");
    expect(i18n.t("app.version", { version: "1.2.3" })).toBe("Version 1.2.3");
  });

  it("speaks French on a French system", async () => {
    await initI18n("fr-FR");
    expect(i18n.t("facts.fishing")).toBe(fr.facts.fishing);
    expect(i18n.t("facts.fishing")).not.toBe(en.facts.fishing);
    await initI18n("en-US");
  });

  it("translates every string into French, with the same placeholders", () => {
    const english = flat(en as Catalogue);
    const french = flat(fr as Catalogue);
    for (const [key, text] of english) {
      expect(french.has(key), `missing ${key}`).toBe(true);
      expect(placeholders(french.get(key) ?? ""), key).toBe(placeholders(text));
    }
    for (const key of french.keys())
      expect(english.has(key), `extra ${key}`).toBe(true);
  });
});
