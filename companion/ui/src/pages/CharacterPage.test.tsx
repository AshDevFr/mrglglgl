import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { StoredCharacter } from "@/bindings/StoredCharacter";
import { CharacterHeader } from "@/components/CharacterHeader";
import { JournalPage } from "@/components/JournalPage";
import {
  renderWithProviders,
  syntheticAwards,
  syntheticJournal,
} from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) =>
    `asset://localhost/${encodeURIComponent(path)}`,
  invoke: vi.fn(),
}));

const wren: StoredCharacter = {
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
};

describe("JournalPage", () => {
  it("reads a day as a page: the date, then what happened in sentences", () => {
    renderWithProviders(
      <JournalPage
        journal={syntheticJournal}
        day="2026-11-08"
        awards={syntheticAwards}
      />,
    );
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "Sunday, November 8, 2026",
    );
    const entry = screen.getByText(/Killed 23 Prairie Wolves/);
    expect(entry).toHaveTextContent(
      "Died to a Murloc Streamrunner in Red Rocks.",
    );
    expect(entry).toHaveTextContent("Grouped with Tamsin Reedwater.");
  });

  it("presses the day's milestones onto the page as stamps", () => {
    renderWithProviders(
      <JournalPage
        journal={syntheticJournal}
        day="2026-11-01"
        awards={syntheticAwards}
      />,
    );
    expect(
      screen.getByRole("img", { name: /^Kept the Boot:/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("img", { name: /^Sleeps With the Fishes:/ }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Summary only/)).toBeInTheDocument();
  });

  it("pins screenshots that open full size", async () => {
    renderWithProviders(
      <JournalPage
        journal={syntheticJournal}
        day="2026-11-08"
        awards={[]}
        screenshots={[
          {
            path: "/wow/_forever_/Screenshots/WoWScrnShot_110826_120000.jpg",
            taken: 1794000000,
            moment: "levelUp",
          },
        ]}
      />,
    );
    const photo = screen.getByAltText("Screenshot: level up");
    expect(photo.getAttribute("src")).toMatch(/^asset:\/\/localhost\//);
    fireEvent.click(photo);
    await waitFor(() =>
      expect(screen.getAllByAltText("Screenshot: level up")).toHaveLength(2),
    );
  });

  it("says plainly when nothing happened", () => {
    renderWithProviders(
      <JournalPage journal={syntheticJournal} day="2026-11-10" awards={[]} />,
    );
    expect(screen.getByText("Nothing to report.")).toBeInTheDocument();
  });
});

describe("CharacterHeader", () => {
  it("shows name and surname, level, last zone, boots kept and last synced", () => {
    renderWithProviders(
      <CharacterHeader character={wren} journal={syntheticJournal} />,
    );
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
      "Wren Hollowbrook",
    );
    expect(screen.getByText(/^Level \d+$/)).toBeInTheDocument();
    expect(screen.getByText("Mulgore")).toBeInTheDocument();
    expect(screen.getByText("1 boot kept")).toBeInTheDocument();
    expect(screen.getByText(/^Last synced /)).toBeInTheDocument();
  });

  it("copes before the journal has been read", () => {
    renderWithProviders(
      <CharacterHeader
        character={{ ...wren, name: null, surname: null, lastSynced: null }}
        journal={null}
      />,
    );
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
      "Wren-Hollowbrook",
    );
    expect(screen.getByText("Not synced yet")).toBeInTheDocument();
  });
});
