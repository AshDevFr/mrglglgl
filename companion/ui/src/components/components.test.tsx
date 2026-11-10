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

const dialog = vi.hoisted(() => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => dialog);

import { SettingsForm } from "./SettingsForm";

const wren: CharacterSummary = {
  archived: false,
  milestones: 2,
  toReview: 0,
  level: 12,
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
    enabled: true,
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

describe("CharacterTree details", () => {
  it("says each character's level, race and class", () => {
    const unknown: CharacterSummary = {
      ...wren,
      level: null,
      character: {
        ...wren.character,
        id: 3,
        name: "Ash",
        surname: null,
        race: "Troll",
        class: "EVOKERISH",
      },
    };
    renderWithProviders(
      <CharacterTree
        characters={[wren, unknown]}
        selectedId={1}
        onSelect={() => {}}
      />,
    );
    expect(screen.getByText("Level 12 Tauren Hunter")).toBeInTheDocument();
    // No level read yet, and a class the app does not know: its token, made readable.
    expect(screen.getByText("Troll Evokerish")).toBeInTheDocument();
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
  const settings = (patch: Partial<Settings> = {}): Settings => ({
    wowRootOverride: null,
    backupRetention: "days90",
    uiLanguage: null,
    site: defaultSite,
    writing: defaultWriting,
    ...patch,
  });

  it("saves the folder as soon as it is chosen, with nothing to press", async () => {
    const onSave = vi.fn((_: Settings) => Promise.resolve());
    renderWithProviders(
      <SettingsForm
        view={{
          settings: settings(),
          roots: [
            { path: "/Applications/World of Warcraft", source: "defaultPath" },
          ],
          unreadable: [],
        }}
        onSave={onSave}
        readStored={async () => settings()}
      />,
    );
    expect(screen.getByText("Found automatically:")).toBeInTheDocument();
    expect(
      screen.getByText("/Applications/World of Warcraft"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/only when you log out or type \/reload/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/changes only the game's files/),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Save" })).toBeNull();

    dialog.open.mockResolvedValueOnce("/Games/World of Warcraft");
    fireEvent.click(screen.getByText("Choose another folder…"));
    expect(await screen.findByText("Chosen by you:")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("Saved.")).toBeInTheDocument());
    expect(onSave).toHaveBeenCalledWith(
      settings({ wowRootOverride: "/Games/World of Warcraft" }),
    );
    expect(screen.queryByText("Save to use this.")).toBeNull();
  });

  it("keeps backups by age, explaining that the archive keeps full history anyway", async () => {
    const onSave = vi.fn((_: Settings) => Promise.resolve());
    renderWithProviders(
      <SettingsForm
        view={{ settings: settings(), roots: [], unreadable: [] }}
        onSave={onSave}
        readStored={async () => settings()}
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
    await waitFor(() => expect(onSave).toHaveBeenCalled());
    expect(onSave.mock.calls[0][0].backupRetention).toBe("forever");
  });

  it("saves writing preferences once typing pauses, keeping the other settings as stored", async () => {
    const onSave = vi.fn((_: Settings) => Promise.resolve());
    const stored = settings({ wowRootOverride: "/Games/World of Warcraft" });
    renderWithProviders(
      <SettingsForm
        section="writing"
        view={{ settings: settings(), roots: [], unreadable: [] }}
        onSave={onSave}
        readStored={async () => stored}
      />,
    );
    expect(screen.queryByLabelText(/World of Warcraft folder/)).toBeNull();
    fireEvent.change(screen.getByLabelText(/Writing preferences/), {
      target: { value: "No em dashes." },
    });
    expect(onSave).not.toHaveBeenCalled();
    await waitFor(() => expect(onSave).toHaveBeenCalled());
    const saved = onSave.mock.calls[0][0];
    expect(saved.writing.preferences).toBe("No em dashes.");
    expect(saved.wowRootOverride).toBe("/Games/World of Warcraft");
  });

  it("saves a switch at once without undoing a model chosen meanwhile", async () => {
    const onSave = vi.fn((_: Settings) => Promise.resolve());
    // The form was drawn before a model was picked above it.
    const stored = settings({
      writing: { ...defaultWriting, modelId: "qwen" },
    });
    renderWithProviders(
      <SettingsForm
        section="writing"
        view={{ settings: settings(), roots: [], unreadable: [] }}
        onSave={onSave}
        readStored={async () => stored}
      />,
    );
    fireEvent.click(screen.getByLabelText(/Write a weekly chapter/));
    await waitFor(() => expect(onSave).toHaveBeenCalled());
    const saved = onSave.mock.calls[0][0];
    expect(saved.writing.weekly).toBe(true);
    expect(saved.writing.modelId).toBe("qwen");
  });

  it("waits for its own Save before writing with an endpoint", async () => {
    const onSave = vi.fn((_: Settings) => Promise.resolve());
    renderWithProviders(
      <SettingsForm
        section="ai"
        view={{ settings: settings(), roots: [], unreadable: [] }}
        onSave={onSave}
        readStored={async () => settings()}
      />,
    );
    fireEvent.click(screen.getByLabelText(/An endpoint you run or pay for/));
    expect(screen.getByText("Not saved yet.")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText(/Endpoint address/), {
      target: { value: "http://localhost:11434/v1" },
    });
    expect(onSave).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Save who writes" }));
    await waitFor(() => expect(onSave).toHaveBeenCalled());
    const saved = onSave.mock.calls[0][0];
    expect(saved.writing.provider).toBe("endpoint");
    expect(saved.writing.endpointUrl).toBe("http://localhost:11434/v1");
  });

  it("never pins the folder it found, and finds it again on request", async () => {
    const onSave = vi.fn((_: Settings) => Promise.resolve());
    const view = (override: string | null, found: boolean) => ({
      settings: settings({ wowRootOverride: override }),
      roots: found
        ? [
            {
              path: override ?? "/Applications/World of Warcraft",
              source: "override" as const,
            },
          ]
        : [],
      unreadable: [],
    });
    const { unmount } = renderWithProviders(
      <SettingsForm
        view={view("/Games/Gone", false)}
        onSave={onSave}
        readStored={async () => settings({ wowRootOverride: "/Games/Gone" })}
      />,
    );
    expect(screen.getByText("/Games/Gone")).toBeInTheDocument();
    expect(
      screen.getByText(/The folder you chose has no World of Warcraft in it/),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByText("Find it automatically again"));
    await waitFor(() => expect(onSave).toHaveBeenCalled());
    expect(onSave.mock.calls[0][0].wowRootOverride).toBeNull();
    unmount();

    // Changing anything else while the folder is found keeps it automatic.
    const again = vi.fn((_: Settings) => Promise.resolve());
    renderWithProviders(
      <SettingsForm
        view={view(null, true)}
        onSave={again}
        readStored={async () => settings()}
      />,
    );
    const language = screen
      .getAllByLabelText("Language")
      .find((el) => el.tagName === "INPUT");
    if (!language) throw new Error("no language input");
    fireEvent.click(language);
    fireEvent.click(await screen.findByRole("option", { name: "French" }));
    await waitFor(() => expect(again).toHaveBeenCalled());
    expect(again.mock.calls[0][0].wowRootOverride).toBeNull();
  });
});
