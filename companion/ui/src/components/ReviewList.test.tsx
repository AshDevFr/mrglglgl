import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ReviewItem } from "@/bindings/ReviewItem";
import type { Story } from "@/bindings/Story";
import { ReviewList } from "@/components/ReviewList";
import { renderWithProviders, syntheticJournal } from "@/test/render";

const commands = vi.hoisted(() => ({
  items: [] as unknown[],
  acceptDraft: vi.fn(() => Promise.resolve(true)),
  discardDraft: vi.fn(() => Promise.resolve(true)),
  writeDrafts: vi.fn(() => Promise.resolve()),
}));

vi.mock("@/api/commands", () => ({
  reviewList: () => Promise.resolve(commands.items),
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

  it("says when there is nothing to review", async () => {
    commands.items = [];
    renderWithProviders(<ReviewList characterId={1} />);
    expect(await screen.findByText(/Nothing to review/)).toBeInTheDocument();
  });
});
