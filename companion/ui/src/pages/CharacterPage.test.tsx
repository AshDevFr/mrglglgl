import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { StoredCharacter } from "@/bindings/StoredCharacter";
import { CharacterHeader } from "@/components/CharacterHeader";
import { JournalPage } from "@/components/JournalPage";
import { ChallengeList } from "@/pages/MilestonesPage";
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
  it("reads a day as a page: the date, then what happened as labelled lists", () => {
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
    const list = screen.getByLabelText("What happened, as recorded");
    const row = (label: string) => {
      const term = within(list).getByText(label, { selector: "dt" });
      return term.nextElementSibling as HTMLElement;
    };
    expect(row("Killed")).toHaveTextContent("Prairie Wolf × 23");
    expect(row("Deaths")).toHaveTextContent(
      "Murloc Streamrunner, at Red Rocks",
    );
    expect(row("Grouped with")).toHaveTextContent("Tamsin Reedwater");
    expect(row("Level")).toHaveTextContent("5 → 6");
    expect(row("Discovered")).toHaveTextContent("Thunder Bluff");
    expect(row("Earned")).toHaveTextContent("Auction house: 18s 40c");
    expect(row("Spent")).toHaveTextContent("Repairs: 1s 28c");
    expect(row("Gold")).toHaveTextContent("23s 50c → 43s 94c");
    expect(row("Auction house")).toHaveTextContent("Sold Copper Ore");
    expect(row("Legacy Challenges")).toHaveTextContent(
      "Explore Moonglade (Kalimdor)",
    );
    expect(row("Chat")).toHaveTextContent(
      "[Say] You: The wolves grow bold this season.",
    );
    expect(row("Chat")).toHaveTextContent(
      "[Say] Tamsin: Then we thin the pack before nightfall.",
    );
    expect(row("Chat")).toHaveTextContent(
      "[Emote] *checks the string of her bow.*",
    );
  });

  it("presses the rarest stamps of a busy day and lists only the rest beside it", () => {
    renderWithProviders(
      <JournalPage
        journal={syntheticJournal}
        day="2026-11-08"
        awards={syntheticAwards}
      />,
    );
    const day = syntheticAwards.filter((a) => a.day === "2026-11-08");
    expect(day.length).toBeGreaterThan(3);
    expect(screen.getAllByRole("img", { name: /: / })).toHaveLength(3);
    expect(
      screen.getByRole("img", { name: /^Mrglglgl'd:/ }),
    ).toBeInTheDocument();
    // The list beside the page holds only the milestones without a stamp: a stamp opens to
    // its own name and description.
    const rest = screen.getByRole("heading", { name: "Also earned this day" })
      .parentElement as HTMLElement;
    const items = within(rest).getAllByRole("listitem");
    expect(items).toHaveLength(day.length - 3);
    expect(rest).not.toHaveTextContent("Mrglglgl'd");
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

describe("ChallengeList", () => {
  it("lists every Legacy Challenge completed with its day, newest first", () => {
    renderWithProviders(
      <ChallengeList
        challenges={{
          855: {
            name: "Explore Moonglade",
            category: "Kalimdor",
            day: "2026-11-06",
          },
          900: { name: "Druid Initiate", category: "Druid", day: "2026-11-07" },
        }}
      />,
    );
    const items = screen.getAllByRole("listitem");
    expect(items[0]).toHaveTextContent("Druid Initiate (Druid), completed");
    expect(items[1]).toHaveTextContent("Explore Moonglade (Kalimdor)");
    expect(
      screen.getByRole("heading", { name: "2 Legacy Challenges completed" }),
    ).toBeInTheDocument();
  });
});
