import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ModelsView } from "@/bindings/ModelsView";
import type { SettingsView } from "@/bindings/SettingsView";
import {
  defaultWriting,
  renderWithProviders,
  syntheticAwards,
} from "@/test/render";
import { CharacterTree } from "./CharacterTree";
import { FirstRun } from "./FirstRun";
import { WeekNav } from "./WeekNav";

const commands = vi.hoisted(() => ({
  getSettings: vi.fn(),
  models: vi.fn(),
  saveSettings: vi.fn(async () => ({})),
  downloadModel: vi.fn(),
}));
vi.mock("@/api/commands", () => commands);

function settings(roots: SettingsView["roots"]): SettingsView {
  return {
    settings: {
      wowRootOverride: null,
      backupRetention: "days90",
      uiLanguage: null,
      writing: defaultWriting,
    },
    roots,
  };
}

const qwen = {
  id: "qwen3.5-4b",
  name: "Qwen3.5 4B",
  sizeBytes: 2_740_937_888,
  licence: "Apache-2.0",
  languages: ["en", "fr"],
  minRamGb: 8,
  recommended: true,
  proseNote: null,
  installed: false,
  partialBytes: 0,
};

function catalogue(extra: Partial<ModelsView> = {}): ModelsView {
  return {
    chosenId: "qwen3.5-4b",
    advice: { kind: "recommend", id: "qwen3.5-4b" },
    models: [qwen],
    ...extra,
  };
}

describe("first run", () => {
  beforeEach(() => {
    commands.saveSettings.mockClear();
    commands.downloadModel.mockClear();
  });

  it("shows the game it found, how journals arrive, and offers a model without downloading", async () => {
    commands.getSettings.mockResolvedValue(
      settings([
        { path: "/Applications/World of Warcraft", source: "defaultPath" },
      ]),
    );
    commands.models.mockResolvedValue(catalogue());
    const onOpenSettings = vi.fn();
    renderWithProviders(<FirstRun onOpenSettings={onOpenSettings} />);
    expect(
      await screen.findByText(
        "Found your game at /Applications/World of Warcraft.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/only when you log out or type \/reload/),
    ).toBeInTheDocument();
    expect(
      await screen.findByText(
        /can download Qwen3\.5 4B \(2\.74 GB\) to write your journal on this computer\. Nothing is downloaded until you choose to\./,
      ),
    ).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Choose a writing model" }),
    );
    expect(onOpenSettings).toHaveBeenCalled();
    expect(commands.downloadModel).not.toHaveBeenCalled();
  });

  it("asks for the game folder when none is found, and saves it", async () => {
    commands.getSettings.mockResolvedValue(settings([]));
    commands.models.mockResolvedValue(catalogue());
    renderWithProviders(<FirstRun onOpenSettings={vi.fn()} />);
    expect(
      await screen.findByText(
        "No World of Warcraft folder found. Enter the game folder above (the one containing _forever_), then save.",
      ),
    ).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("World of Warcraft folder"), {
      target: { value: " /Games/World of Warcraft " },
    });
    fireEvent.click(screen.getByRole("button", { name: "Use this folder" }));
    await waitFor(() =>
      expect(commands.saveSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          wowRootOverride: "/Games/World of Warcraft",
        }),
      ),
    );
  });

  it("says plainly when the computer cannot run a model, with the way out", async () => {
    commands.getSettings.mockResolvedValue(settings([]));
    commands.models.mockResolvedValue(
      catalogue({ advice: { kind: "notEnoughMemory", neededGb: 8, hasGb: 4 } }),
    );
    renderWithProviders(<FirstRun onOpenSettings={vi.fn()} />);
    expect(
      await screen.findByText(
        /the writing models need 8 GB\. Use an endpoint instead/,
      ),
    ).toBeInTheDocument();
  });

  it("says writing is set up once a model is there", async () => {
    commands.getSettings.mockResolvedValue(settings([]));
    commands.models.mockResolvedValue(
      catalogue({ models: [{ ...qwen, installed: true }] }),
    );
    renderWithProviders(<FirstRun onOpenSettings={vi.fn()} />);
    expect(await screen.findByText("Writing is set up.")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Choose a writing model" }),
    ).toBeNull();
  });
});

describe("keyboard", () => {
  it("reaches every character in the sidebar as a button", () => {
    renderWithProviders(
      <CharacterTree
        characters={[
          {
            archived: false,
            milestones: syntheticAwards.length,
            character: {
              id: 1,
              flavor: "_forever_",
              account: "ACCOUNT1",
              realmFolder: "93",
              characterFolder: "Wren",
              journalPath: "/wow/Mrglglgl.lua",
              name: "Wren",
              surname: "Hollowbrook",
              realm: "Silverpine",
              race: "Tauren",
              class: "HUNTER",
              faction: "Horde",
              locale: "enUS",
              lastSynced: null,
              status: null,
            },
          },
        ]}
        selectedId={null}
        onSelect={vi.fn()}
      />,
    );
    const wren = screen.getByRole("button", { name: /Wren Hollowbrook/ });
    expect(wren.tagName).toBe("BUTTON");
    expect(screen.getByRole("button", { name: /Silverpine/ }).tagName).toBe(
      "BUTTON",
    );
  });

  it("moves between days with the arrow keys, Home and End", () => {
    const onSelect = vi.fn();
    renderWithProviders(
      <WeekNav
        selected="2026-11-09"
        written={new Set(["2026-11-09", "2026-11-10"])}
        today="2026-11-12"
        onSelect={onSelect}
      />,
    );
    const monday = screen.getByRole("button", {
      name: /Monday, November 9, 2026$/,
    });
    fireEvent.keyDown(monday, { key: "ArrowRight" });
    expect(onSelect).toHaveBeenLastCalledWith("2026-11-10");
    fireEvent.keyDown(monday, { key: "End" });
    expect(onSelect).toHaveBeenLastCalledWith("2026-11-12");
    fireEvent.keyDown(monday, { key: "ArrowLeft" });
    expect(onSelect).toHaveBeenCalledTimes(2);
    fireEvent.keyDown(monday, { key: "Home" });
    expect(onSelect).toHaveBeenLastCalledWith("2026-11-09");
  });
});
