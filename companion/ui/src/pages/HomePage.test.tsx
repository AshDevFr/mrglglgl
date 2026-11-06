import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  defaultSite,
  defaultWriting,
  renderWithProviders,
} from "@/test/render";
import { HomePage } from "./HomePage";

const commands = vi.hoisted(() => ({
  characters: [] as unknown[],
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
    }),
  models: () =>
    Promise.resolve({
      chosenId: null,
      advice: { kind: "recommend", id: "x" },
      models: [],
    }),
  saveSettings: vi.fn(),
}));

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

  it("asks the reader to pick a character once there are some", async () => {
    commands.characters = [{ character: { id: 1 } }];
    renderWithProviders(<HomePage />);
    expect(await screen.findByText(/Choose a character/)).toBeInTheDocument();
  });
});
