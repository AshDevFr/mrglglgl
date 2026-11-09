import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, log } from "./log";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const mocked = vi.mocked(tauriInvoke);

describe("the window's log", () => {
  beforeEach(() => {
    mocked.mockReset();
    vi.stubEnv("MODE", "development");
    vi.spyOn(console, "warn").mockImplementation(() => {});
    vi.spyOn(console, "error").mockImplementation(() => {});
  });
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("logs a failed command with its name and error, and passes the error on", async () => {
    mocked.mockImplementation((command) =>
      command === "log_from_window"
        ? Promise.resolve(undefined)
        : Promise.reject("network is down"),
    );
    await expect(invoke("addon_release", { force: true })).rejects.toBe(
      "network is down",
    );
    expect(mocked).toHaveBeenCalledWith("log_from_window", {
      level: "warn",
      message: expect.stringMatching(
        /^command addon_release failed after \d+ms: network is down$/,
      ),
    });
  });

  it("logs nothing for a command that works", async () => {
    mocked.mockResolvedValue(42);
    await expect(invoke("app_info")).resolves.toBe(42);
    expect(mocked).toHaveBeenCalledTimes(1);
  });

  it("never throws when the log itself cannot be written", () => {
    mocked.mockRejectedValue(new Error("no app"));
    expect(() => log("error", "something broke")).not.toThrow();
  });

  it("writes nothing to the file in the unit tests", () => {
    vi.stubEnv("MODE", "test");
    log("error", "something broke");
    expect(mocked).not.toHaveBeenCalled();
  });
});
