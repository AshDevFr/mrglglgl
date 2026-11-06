import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  defaultSite,
  defaultWriting,
  renderWithProviders,
} from "@/test/render";
import { HomePage } from "./HomePage";

const commands = vi.hoisted(() => ({
  characters: [] as unknown[],
  addons: [] as unknown[],
}));

vi.mock("@/api/commands", () => ({
  appInfo: () => Promise.resolve({ version: "9.9.9" }),
  listCharacters: () => Promise.resolve(commands.characters),
  getSettings: () =>
    Promise.resolve({
      settings: {
        wowRootOverride: null,
        backupRetention: "days90",
        uiLanguage: null,
        site: defaultSite,
        writing: defaultWriting,
      },
      roots: [
        { path: "/Applications/World of Warcraft", source: "defaultPath" },
      ],
      unreadable: [],
    }),
  models: () =>
    Promise.resolve({
      chosenId: null,
      advice: { kind: "recommend", id: "x" },
      models: [],
    }),
  saveSettings: vi.fn(),
  addonViews: () => Promise.resolve(commands.addons),
}));

function addon(state: string, extra: Record<string, unknown> = {}) {
  return {
    flavorPath: "/Applications/World of Warcraft/_forever_",
    ownership: state === "notInstalled" ? "notInstalled" : "managed",
    rollback: null,
    status: {
      flavor: "_forever_",
      folder:
        "/Applications/World of Warcraft/_forever_/Interface/AddOns/Mrglglgl",
      clientVersion: "1.60.1.70790",
      installed:
        state === "notInstalled"
          ? null
          : {
              version: "0.1.0",
              interface: 16001,
              schema: 1,
              development: false,
            },
      lastRun:
        state === "recording"
          ? {
              character: "Wren",
              version: "0.1.0",
              interface: 16001,
              login: 1794000000,
            }
          : null,
      state,
      problems: [],
      ...extra,
    },
  };
}

describe("HomePage", () => {
  it("has a murloc-flavoured empty state and explains when journals arrive", async () => {
    commands.characters = [];
    renderWithProviders(<HomePage />);
    expect(
      await screen.findByText(/No journals yet\. The murloc is waiting/),
    ).toBeInTheDocument();
    expect(
      screen.getAllByText(/log out or type \/reload/).length,
    ).toBeGreaterThan(0);
    expect(await screen.findByText("Version 9.9.9")).toBeInTheDocument();
  });

  it("warns when the addon is not installed, and opens its settings", async () => {
    commands.characters = [{ character: { id: 1 } }];
    commands.addons = [addon("notInstalled")];
    const onOpenSettings = vi.fn();
    renderWithProviders(<HomePage onOpenSettings={onOpenSettings} />);
    expect(
      await screen.findByText("The addon is not installed"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByText("Install the addon"));
    expect(onOpenSettings).toHaveBeenCalled();
    expect(sessionStorage.getItem("mrglglgl.settingsTab")).toBe("addon");
  });

  it("shows the addon in one quiet line while it records", async () => {
    commands.characters = [{ character: { id: 1 } }];
    commands.addons = [addon("recording")];
    renderWithProviders(<HomePage />);
    expect(
      await screen.findByText(
        /^Addon: Version 0\.1\.0 · Recording · last noted by Wren on/,
      ),
    ).toBeInTheDocument();
    expect(screen.queryByText("The addon is not installed")).toBeNull();
  });

  it("asks the reader to pick a character once there are some", async () => {
    commands.characters = [{ character: { id: 1 } }];
    renderWithProviders(<HomePage />);
    expect(await screen.findByText(/Choose a character/)).toBeInTheDocument();
  });
});
