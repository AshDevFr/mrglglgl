import { useQuery } from "@tanstack/react-query";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { reviewList } from "@/api/commands";
import type { ReviewItem } from "@/bindings/ReviewItem";
import type { Story } from "@/bindings/Story";
import { ReviewList } from "@/components/ReviewList";
import { renderWithProviders, syntheticJournal } from "@/test/render";

const commands = vi.hoisted(() => ({
  review: true,
  items: [] as unknown[],
  all: [] as unknown[],
  acceptDraft: vi.fn(() => Promise.resolve(true)),
  discardDraft: vi.fn(() => Promise.resolve(true)),
  writeDrafts: vi.fn(() => Promise.resolve()),
}));

vi.mock("@/lib/siteBuild", () => ({ scheduleRebuild: vi.fn() }));

vi.mock("@/api/commands", () => ({
  reviewList: () => Promise.resolve(commands.items),
  getSettings: () =>
    Promise.resolve({
      settings: { writing: { review: commands.review } },
    }),
  reviewAll: () => Promise.resolve(commands.all),
  characterScreenshots: () =>
    Promise.resolve({
      "2026-11-09": [
        { path: "/shots/level.png", taken: 1_794_200_000, moment: "levelUp" },
      ],
    }),
  screenshotUrl: (path: string) => `asset://${path}`,
  listCharacters: () =>
    Promise.resolve([
      {
        character: {
          id: 1,
          name: "Wren",
          surname: null,
          characterFolder: "Wren",
        },
      },
      {
        character: {
          id: 2,
          name: "Brakka",
          surname: null,
          characterFolder: "Brakka",
        },
      },
    ]),
  characterJournal: () => Promise.resolve(syntheticJournal),
  listNarrators: () =>
    Promise.resolve([
      { id: "murloc_scribe", name: "The murloc scribe", optIn: false },
      { id: "grumpy_dwarf", name: "The grumpy dwarf", optIn: false },
    ]),
  acceptDraft: commands.acceptDraft,
  discardDraft: commands.discardDraft,
  writeDrafts: commands.writeDrafts,
  onWriting: () => Promise.resolve(() => {}),
}));

function draft(id: number, day: string, text: string): Story {
  return {
    id,
    characterId: 1,
    kind: "day",
    startDay: day,
    endDay: day,
    narrator: "grumpy_dwarf",
    language: "en",
    provider: "local",
    model: "m",
    promptHash: "h",
    text,
    createdAt: 1_794_200_000,
    check: "passed",
    checkReason: null,
    status: "draft",
    note: "shorter",
    partial: false,
  };
}

function item(day: string, extra: Partial<ReviewItem> = {}): ReviewItem {
  return {
    kind: "day",
    startDay: day,
    endDay: day,
    drafts: [],
    hasEntry: false,
    discarded: false,
    job: null,
    ...extra,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  commands.review = true;
  commands.items = [
    item("2026-11-09", { drafts: [draft(7, "2026-11-09", "The wolves won.")] }),
    item("2026-11-08"),
    item("2026-11-01", { discarded: true }),
  ];
});

const row = async (name: RegExp) => screen.findByRole("article", { name });

describe("ReviewList", () => {
  it("shows each day's draft, or why there is none", async () => {
    renderWithProviders(<ReviewList characterId={1} />);
    const drafted = await row(/November 9, 2026/);
    expect(drafted).toHaveTextContent("The wolves won.");
    expect(drafted).toHaveTextContent("Draft by The grumpy dwarf");
    expect(drafted).toHaveTextContent("You asked: “shorter”");
    expect(await row(/November 8, 2026/)).toHaveTextContent("Not written yet.");
    expect(await row(/November 1, 2026/)).toHaveTextContent(
      "You threw the last draft away.",
    );
  });

  it("shows what happened as the journal's list, not as prose", async () => {
    renderWithProviders(<ReviewList characterId={1} />);
    const drafted = await row(/November 9, 2026/);
    // The journal arrives after the list.
    await waitFor(() =>
      expect(drafted.querySelector("dl.fact-list dt")).not.toBeNull(),
    );
  });

  it("shows the day's screenshots, as the journal pins them", async () => {
    renderWithProviders(<ReviewList characterId={1} />);
    const drafted = await row(/November 9, 2026/);
    await waitFor(() =>
      expect(
        drafted.querySelector('img[src="asset:///shots/level.png"]'),
      ).not.toBeNull(),
    );
    // A day without screenshots shows none.
    expect((await row(/November 8, 2026/)).querySelector("img")).toBeNull();
  });

  it("accepts and discards a draft", async () => {
    renderWithProviders(<ReviewList characterId={1} />);
    const drafted = await row(/November 9, 2026/);
    fireEvent.click(within(drafted).getByRole("button", { name: "Accept" }));
    await waitFor(() =>
      expect(commands.acceptDraft).toHaveBeenCalledWith(1, 7),
    );
    fireEvent.click(within(drafted).getByRole("button", { name: "Discard" }));
    await waitFor(() =>
      expect(commands.discardDraft).toHaveBeenCalledWith(1, 7),
    );
  });

  it("rewrites a draft with a note", async () => {
    renderWithProviders(<ReviewList characterId={1} />);
    const drafted = await row(/November 9, 2026/);
    fireEvent.click(within(drafted).getByRole("button", { name: "Rewrite" }));
    fireEvent.change(within(drafted).getByLabelText(/What should change/), {
      target: { value: "more about the boot" },
    });
    fireEvent.click(
      within(drafted).getByRole("button", { name: "Write the draft" }),
    );
    await waitFor(() =>
      expect(commands.writeDrafts).toHaveBeenCalledWith(
        1,
        [{ kind: "day", startDay: "2026-11-09" }],
        null,
        "more about the boot",
      ),
    );
  });

  it("writes the selected days in one go", async () => {
    renderWithProviders(<ReviewList characterId={1} />);
    await row(/November 8, 2026/);
    fireEvent.click(screen.getByRole("checkbox", { name: /November 8, 2026/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /November 1, 2026/ }));
    fireEvent.click(screen.getByRole("button", { name: "Write 2 selected" }));
    await waitFor(() =>
      expect(commands.writeDrafts).toHaveBeenCalledWith(
        1,
        [
          { kind: "day", startDay: "2026-11-08" },
          { kind: "day", startDay: "2026-11-01" },
        ],
        null,
        null,
      ),
    );
  });

  it("says that writing here goes straight to the journal without review mode", async () => {
    commands.review = false;
    renderWithProviders(<ReviewList characterId={1} />);
    expect(
      await screen.findByText(
        /Review mode is off, so what you write here goes straight into the journal/,
      ),
    ).toBeInTheDocument();
  });

  it("shares the character's review cache with the journal page without breaking", async () => {
    // The journal page caches the same list under the same key, in its own shape.
    function JournalPageQuery() {
      useQuery({ queryKey: ["review", 1], queryFn: () => reviewList(1) });
      return null;
    }
    renderWithProviders(
      <>
        <JournalPageQuery />
        <ReviewList characterId={1} />
      </>,
    );
    expect(await row(/November 9, 2026/)).toHaveTextContent("The wolves won.");
  });

  it("says when there is nothing to review", async () => {
    commands.items = [];
    renderWithProviders(<ReviewList characterId={1} />);
    expect(await screen.findByText(/Nothing to review/)).toBeInTheDocument();
  });
});

describe("ReviewList for every character", () => {
  beforeEach(() => {
    commands.all = [
      {
        characterId: 1,
        items: [
          item("2026-11-09", {
            drafts: [draft(7, "2026-11-09", "The wolves won.")],
          }),
          item("2026-11-08"),
        ],
      },
      {
        characterId: 2,
        items: [
          item("2026-11-09", {
            drafts: [draft(9, "2026-11-09", "Brakka fished.")],
          }),
          item("2026-11-07"),
        ],
      },
    ];
  });

  const group = (name: string) => screen.findByRole("region", { name });

  it("lists each character's days under their name", async () => {
    renderWithProviders(<ReviewList />);
    expect(await group("Wren")).toHaveTextContent("The wolves won.");
    expect(await group("Brakka")).toHaveTextContent("Brakka fished.");
  });

  it("accepts a draft for the character it belongs to", async () => {
    renderWithProviders(<ReviewList />);
    const brakka = await group("Brakka");
    fireEvent.click(within(brakka).getByRole("button", { name: "Accept" }));
    await waitFor(() =>
      expect(commands.acceptDraft).toHaveBeenCalledWith(2, 9),
    );
  });

  it("writes a selection across characters, each with its own", async () => {
    renderWithProviders(<ReviewList />);
    const wren = await group("Wren");
    const brakka = await group("Brakka");
    fireEvent.click(
      within(wren).getByRole("checkbox", { name: /November 8, 2026/ }),
    );
    fireEvent.click(
      within(brakka).getByRole("checkbox", { name: /November 7, 2026/ }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Write 2 selected" }));
    await waitFor(() => expect(commands.writeDrafts).toHaveBeenCalledTimes(2));
    expect(commands.writeDrafts).toHaveBeenCalledWith(
      1,
      [{ kind: "day", startDay: "2026-11-08" }],
      null,
      null,
    );
    expect(commands.writeDrafts).toHaveBeenCalledWith(
      2,
      [{ kind: "day", startDay: "2026-11-07" }],
      null,
      null,
    );
  });

  it("says when no character has anything to review", async () => {
    commands.all = [];
    renderWithProviders(<ReviewList />);
    expect(
      await screen.findByText(/Nothing to review for any character/),
    ).toBeInTheDocument();
  });
});
