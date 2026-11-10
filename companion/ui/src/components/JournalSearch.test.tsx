import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Story } from "@/bindings/Story";
import { renderWithProviders, syntheticJournal } from "@/test/render";
import { SearchField, SearchResults } from "./JournalSearch";

const story = (
  id: number,
  day: string,
  text: string,
  extra: Partial<Story> = {},
) =>
  ({
    id,
    characterId: 1,
    kind: "day",
    startDay: day,
    endDay: day,
    narrator: "murloc_scribe",
    text,
    status: "accepted",
    check: "passed",
    partial: false,
    ...extra,
  }) as Story;

describe("JournalSearch", () => {
  it("lists the days that mention the words, and opens the one chosen", () => {
    const onOpen = vi.fn();
    renderWithProviders(
      <SearchResults
        journal={syntheticJournal}
        stories={[
          story(1, "2026-11-01", "Wren kept the boot, for reasons of her own."),
          // A draft waiting for review is not in the journal yet.
          story(2, "2026-11-09", "Another boot, says the draft.", {
            status: "draft",
          }),
        ]}
        query="BOOT"
        onOpen={onOpen}
      />,
    );
    expect(screen.getByRole("status")).toHaveTextContent(/match/);
    expect(screen.getByText("boot", { selector: "mark" })).toBeInTheDocument();
    expect(screen.queryByText(/says the draft/)).toBeNull();
    fireEvent.click(
      screen.getByText(/kept the/).closest("button") as HTMLElement,
    );
    expect(onOpen).toHaveBeenCalledWith("2026-11-01");
  });

  it("also finds what happened on days the murloc has not written", () => {
    renderWithProviders(
      <SearchResults
        journal={syntheticJournal}
        stories={[]}
        query="hogger"
        onOpen={vi.fn()}
      />,
    );
    expect(
      screen.getAllByText("What happened", { exact: false }).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByText(/hogger/i, { selector: "mark" }).length,
    ).toBeGreaterThan(0);
  });

  it("says when nothing matches, and asks for two letters first", () => {
    const { unmount } = renderWithProviders(
      <SearchResults
        journal={syntheticJournal}
        stories={[]}
        query="zzqq"
        onOpen={vi.fn()}
      />,
    );
    expect(screen.getByRole("status")).toHaveTextContent(
      "Nothing found for “zzqq”.",
    );
    unmount();
    renderWithProviders(
      <SearchResults
        journal={syntheticJournal}
        stories={[]}
        query="z"
        onOpen={vi.fn()}
      />,
    );
    expect(screen.getByRole("status")).toHaveTextContent(
      "Type at least two letters.",
    );
  });

  it("takes the cursor on ⌘ or Ctrl F, and empties on Escape", () => {
    const onChange = vi.fn();
    renderWithProviders(<SearchField value="boot" onChange={onChange} />);
    const field = screen.getByLabelText("Search this journal");
    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    expect(field).toHaveFocus();
    fireEvent.keyDown(field, { key: "Escape" });
    expect(onChange).toHaveBeenCalledWith("");
  });
});
