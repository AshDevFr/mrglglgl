import i18n from "i18next";
import { beforeEach, describe, expect, it } from "vitest";
import { asFailure, failureProblem } from "./failure";
import { initI18n } from "./index";

describe("a failure from the backend", () => {
  beforeEach(async () => {
    await initI18n("en-GB");
  });

  it("is said in the player's language, with the step that fits its kind", () => {
    expect(
      failureProblem(i18n.t, "addon.releaseFailed", {
        kind: "serverTrouble",
        status: 504,
      }),
    ).toBe(
      "Could not look up the addon's releases: the server is having trouble right now. This usually passes within minutes: try again a little later.",
    );
  });

  it("names the space a download needs and the space there is", () => {
    expect(
      failureProblem(i18n.t, "models.failed", {
        kind: "noSpace",
        neededGb: "3.81",
        freeGb: "1.25",
      }),
    ).toBe(
      "The download failed: 3.81 GB of free disk space is needed, 1.25 GB is free. Free some disk space, then press Resume download: it continues where it stopped.",
    );
  });

  it("keeps the backend's words and the context's step when nobody planned for it", () => {
    expect(
      failureProblem(i18n.t, "models.failed", {
        kind: "other",
        message: "the downloaded file is 12 bytes, expected 13",
      }),
    ).toBe(
      `The download failed: the downloaded file is 12 bytes, expected 13. ${i18n.t("models.failedNext")}`,
    );
  });

  it("reads a plain string, from a command that still returns one, as unplanned", () => {
    expect(asFailure("disk on fire")).toEqual({
      kind: "other",
      message: "disk on fire",
    });
    expect(asFailure({ kind: "gameRunning" })).toEqual({ kind: "gameRunning" });
  });

  it("speaks French on a French system", async () => {
    await initI18n("fr-FR");
    expect(
      failureProblem(i18n.t, "addon.actionFailed", { kind: "gameRunning" }),
    ).toBe(
      "L'addon n'a pas été modifié : le jeu est lancé. Ferme le jeu, puis réessaie.",
    );
  });
});
