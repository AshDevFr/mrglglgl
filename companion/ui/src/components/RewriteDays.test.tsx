import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { NarratorInfo } from "@/bindings/NarratorInfo";
import type { Story } from "@/bindings/Story";
import { renderWithProviders } from "@/test/render";
import { RewriteDays, writtenDays } from "./RewriteDays";

const commands = vi.hoisted(() => ({
  rewriteDays: vi.fn(async () => {}),
}));
vi.mock("@/api/commands", () => commands);

const narrators: NarratorInfo[] = [
  {
    id: "murloc_scribe",
    name: "The murloc scribe",
    icon: "fish-monster",
    optIn: false,
    custom: false,
  },
];

let next = 1;
const story = (day: string, extra: Partial<Story> = {}): Story => ({
  id: next++,
  characterId: 1,
  kind: "day",
  startDay: day,
  endDay: day,
  narrator: "murloc_scribe",
  language: "en",
  provider: "local",
  model: "qwen3.5-4b",
  promptHash: "h",
  text: "Mrgl.",
  createdAt: 1_794_200_000,
  check: "passed",
  checkReason: null,
  status: "accepted",
  note: null,
  partial: false,
  ...extra,
});

// Two weeks: Mon 2 and Wed 4 November, and Mon 9 November, which the player edited.
const stories = [
  story("2026-11-02"),
  story("2026-11-04"),
  story("2026-11-04"),
  story("2026-11-09", { check: "edited" }),
  // Not entries: a draft, and a day told only so far.
  story("2026-11-05", { status: "draft" }),
  story("2026-11-10", { partial: true }),
];

function dialog(review: boolean) {
  const onQueued = vi.fn();
  renderWithProviders(
    <RewriteDays
      characterId={7}
      opened
      onClose={vi.fn()}
      stories={stories}
      narrators={narrators}
      review={review}
      onQueued={onQueued}
    />,
  );
  return onQueued;
}

describe("RewriteDays", () => {
  it("lists each day with an entry once, newest first, and marks the player's own", () => {
    expect(writtenDays(stories).map((d) => [d.day, d.edited])).toEqual([
      ["2026-11-09", true],
      ["2026-11-04", false],
      ["2026-11-02", false],
    ]);
  });

  it("selects all days but the edited ones, and sends them to To review in review mode", async () => {
    const onQueued = dialog(true);
    fireEvent.click(screen.getByLabelText("All written days (2)"));
    expect(
      screen.getByRole("switch", { name: /Send them to To review/ }),
    ).toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: "Rewrite 2 days" }));
    await waitFor(() =>
      expect(commands.rewriteDays).toHaveBeenCalledWith(
        7,
        ["2026-11-04", "2026-11-02"],
        null,
        true,
      ),
    );
    expect(onQueued).toHaveBeenCalledWith(2);
  });

  it("takes a week, an edited day ticked by hand, and replaces entries when told to", async () => {
    dialog(true);
    fireEvent.click(screen.getByLabelText(/Week of Monday, November 2/));
    fireEvent.click(screen.getByLabelText("Monday, November 9, 2026"));
    fireEvent.click(
      screen.getByRole("switch", { name: /Send them to To review/ }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Rewrite 3 days" }));
    await waitFor(() =>
      expect(commands.rewriteDays).toHaveBeenLastCalledWith(
        7,
        ["2026-11-09", "2026-11-04", "2026-11-02"],
        null,
        false,
      ),
    );
  });
});
