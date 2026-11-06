import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { CharacterSummary } from "@/bindings/CharacterSummary";
import type { Settings } from "@/bindings/Settings";
import { MilestoneList } from "@/pages/MilestonesPage";
import {
  defaultSite,
  defaultWriting,
  renderWithProviders,
  syntheticAwards,
} from "@/test/render";
import { CharacterTree } from "./CharacterTree";
import { SettingsForm } from "./SettingsForm";

const wren: CharacterSummary = {
  archived: false,
  milestones: 2,
  toReview: 0,
  character: {
    id: 1,
    flavor: "_forever_",
    account: "ACCOUNT1",
    realmFolder: "93",
    characterFolder: "Wren-Hollowbrook",
    journalPath: "/wow/Mrglglgl.lua",
    name: "Wren",
    surname: "Hollowbrook",
    realm: "Test Realm",
    race: "Tauren",
    class: "HUNTER",
    faction: "Horde",
    locale: "enUS",
    lastSynced: 1794400000,
    status: null,
  },
};

describe("CharacterTree", () => {
  it("lists account, realm and characters, marks archived ones and when each synced", () => {
    const onSelect = vi.fn();
    const gone: CharacterSummary = {
      ...wren,
      archived: true,
      character: {
        ...wren.character,
        id: 2,
        name: "Tamsin",
        surname: "Reedwater",
        lastSynced: null,
      },
    };
    renderWithProviders(
      <CharacterTree
        characters={[wren, gone]}
        selectedId={1}
        onSelect={onSelect}
      />,
    );
    expect(screen.getByText("ACCOUNT1")).toBeInTheDocument();
    expect(screen.getByText("Test Realm")).toBeInTheDocument();
    expect(screen.getByText(/^Last synced /)).toBeInTheDocument();
    expect(screen.getByText("Not synced yet")).toBeInTheDocument();
    expect(screen.getByText("Archived")).toBeInTheDocument();
    fireEvent.click(screen.getByText("Tamsin Reedwater"));
    expect(onSelect).toHaveBeenCalledWith(2);
  });
});

describe("MilestoneList", () => {
  it("shows each stamp with the day it was earned", () => {
    renderWithProviders(<MilestoneList awards={syntheticAwards} />);
    expect(screen.getByText("Kept the Boot")).toBeInTheDocument();
    expect(
      screen.getByText("Fished up a boot. Kept it.", { selector: "p" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("img", {
        name: "Kept the Boot: Fished up a boot. Kept it.",
      }),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/^Earned /).length).toBe(syntheticAwards.length);
  });

  it("has a murloc-flavoured empty state", () => {
    renderWithProviders(<MilestoneList awards={[]} />);
    expect(
      screen.getByText(/No stamps yet\. The murloc suggests/),
    ).toBeInTheDocument();
  });
});

describe("SettingsForm", () => {
  it("saves the folder, backup period and language in plain words", async () => {
    const onSave = vi.fn((_: Settings) => Promise.resolve());
    renderWithProviders(
      <SettingsForm
        view={{
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
        }}
        onSave={onSave}
      />,
    );
    expect(
      screen.getByText("/Applications/World of Warcraft"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/only when you log out or type \/reload/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/changes only the game's files/),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/World of Warcraft folder/), {
      target: { value: "  /Games/World of Warcraft  " },
    });
    fireEvent.click(screen.getByText("Save"));
    await waitFor(() =>
      expect(screen.getByText("Settings saved.")).toBeInTheDocument(),
    );
    expect(onSave).toHaveBeenCalledWith({
      wowRootOverride: "/Games/World of Warcraft",
      backupRetention: "days90",
      uiLanguage: null,
      site: defaultSite,
      writing: defaultWriting,
    });
  });

  it("keeps backups by age, explaining that the archive keeps full history anyway", async () => {
    const onSave = vi.fn((_: Settings) => Promise.resolve());
    renderWithProviders(
      <SettingsForm
        view={{
          settings: {
            wowRootOverride: null,
            backupRetention: "days90",
            uiLanguage: null,
            site: defaultSite,
            writing: defaultWriting,
          },
          roots: [],
        }}
        onSave={onSave}
      />,
    );
    expect(
      screen.getByText(/full history stays in the app whatever you choose/),
    ).toBeInTheDocument();
    expect(screen.getByText(/only a safety net/)).toBeInTheDocument();

    const select = screen
      .getAllByLabelText("Keep backups for")
      .find((el) => el.tagName === "INPUT");
    if (!select) throw new Error("no backup period input");
    fireEvent.click(select);
    fireEvent.click(await screen.findByRole("option", { name: "Forever" }));
    fireEvent.click(screen.getByText("Save"));
    await waitFor(() => expect(onSave).toHaveBeenCalled());
    expect(onSave.mock.calls[0][0].backupRetention).toBe("forever");
  });
});
