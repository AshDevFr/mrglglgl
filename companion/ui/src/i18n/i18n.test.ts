import i18n from "i18next";
import { describe, expect, it } from "vitest";
import { initI18n } from "./index";

describe("i18n", () => {
  it("falls back to English for a language with no catalogue", async () => {
    await initI18n("fr-FR");
    expect(i18n.t("app.title")).toBe("Mrglglgl");
    expect(i18n.t("app.version", { version: "1.2.3" })).toBe("Version 1.2.3");
  });
});
