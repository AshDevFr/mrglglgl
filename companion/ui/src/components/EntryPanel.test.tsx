import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Job } from "@/bindings/Job";
import type { NarratorInfo } from "@/bindings/NarratorInfo";
import type { Story } from "@/bindings/Story";
import { applyEvent, type Live, slot } from "@/lib/writing";
import { renderWithProviders } from "@/test/render";
import { EntryPanel } from "./EntryPanel";

const narrators: NarratorInfo[] = [
  {
    id: "murloc_scribe",
    name: "The murloc scribe",
    optIn: false,
    custom: false,
  },
  { id: "grumpy_dwarf", name: "The grumpy dwarf", optIn: false, custom: false },
];

function story(id: number, text: string, extra: Partial<Story> = {}): Story {
  return {
    id,
    characterId: 1,
    kind: "day",
    startDay: "2026-11-08",
    endDay: "2026-11-08",
    narrator: "murloc_scribe",
    language: "en",
    provider: "local",
    model: "qwen3.5-4b",
    promptHash: "h",
    text,
    createdAt: 1_794_200_000,
    check: "passed",
    checkReason: null,
    status: "accepted",
    note: null,
    partial: false,
    ...extra,
  };
}

function job(extra: Partial<Job>): Job {
  return {
    characterId: 1,
    kind: "day",
    startDay: "2026-11-08",
    state: "pending",
    attempts: 0,
    lastError: null,
    narrator: null,
    draft: false,
    note: null,
    partial: false,
    position: 3,
    ...extra,
  };
}

function panel(props: Partial<Parameters<typeof EntryPanel>[0]> = {}) {
  const handlers = { onWrite: vi.fn(), onCancel: vi.fn(), onRetry: vi.fn() };
  renderWithProviders(
    <EntryPanel
      kind="day"
      day="2026-11-08"
      finished
      versions={[]}
      blocked={null}
      narrators={narrators}
      facts={<p className="entry">Killed 23 Prairie Wolves.</p>}
      {...handlers}
      {...props}
    />,
  );
  return handlers;
}

describe("the writer's events", () => {
  it("streams, restarts and finishes per day, ignoring other characters", () => {
    let map = {};
    const at = { characterId: 1, kind: "day" as const, startDay: "2026-11-08" };
    map = applyEvent(map, 1, { type: "started", ...at });
    map = applyEvent(map, 1, { type: "chunk", ...at, text: "The wolves " });
    map = applyEvent(map, 1, { type: "chunk", ...at, text: "were many." });
    map = applyEvent(map, 1, {
      type: "chunk",
      ...at,
      characterId: 2,
      text: "x",
    });
    const live = (map as Record<string, Live>)[slot("day", "2026-11-08")];
    expect(live).toEqual({
      text: "The wolves were many.",
      active: true,
      outcome: null,
      message: null,
    });
    map = applyEvent(map, 1, { type: "restarted", ...at });
    expect((map as Record<string, Live>)[slot("day", "2026-11-08")].text).toBe(
      "",
    );
    map = applyEvent(map, 1, {
      type: "finished",
      ...at,
      outcome: "cancelled",
      message: null,
    });
    expect(
      (map as Record<string, Live>)[slot("day", "2026-11-08")],
    ).toMatchObject({
      active: false,
      outcome: "cancelled",
    });
  });
});

describe("EntryPanel", () => {
  it("shows the entry being written as it streams, and cancels it", () => {
    const { onCancel } = panel({
      live: {
        text: "The wolves were many",
        active: true,
        outcome: null,
        message: null,
      },
      job: job({ state: "running", position: null }),
    });
    expect(screen.getByText("The wolves were many")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(
      /Mrgl, writing…|Drying the pages…|Translating from Murloc…/,
    );
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalled();
  });

  it("says plainly whether the user cancelled or the time ran out", () => {
    panel({
      live: { text: "", active: false, outcome: "cancelled", message: null },
    });
    expect(
      screen.getByText(
        "Cancelled. Nothing was saved. Press Write now when you want this one written.",
      ),
    ).toBeInTheDocument();
  });

  it("says a failure is tried again on its own while attempts are left", () => {
    panel({
      live: { text: "", active: false, outcome: "timedOut", message: null },
      job: job({ state: "pending", attempts: 1 }),
    });
    expect(
      screen.getByText(
        /^Stopped: writing took too long\. Nothing was saved\. It is tried again on its own in a moment/,
      ),
    ).toBeInTheDocument();
  });

  it("says to press Try again once a failure gave up for good", () => {
    panel({
      live: {
        text: "",
        active: false,
        outcome: "failed",
        message: "the model stopped",
      },
      job: job({ state: "failed", attempts: 3, position: null }),
    });
    expect(
      screen.getByText(
        /^The murloc could not write this: the model stopped\. Press Try again/,
      ),
    ).toBeInTheDocument();
  });

  it("shows the newest version, keeps the older ones, and folds the facts away", () => {
    panel({
      versions: [
        story(2, "The plains were quiet.\n\n* Mrgl. No fish today."),
        story(1, "An older telling.", { narrator: "grumpy_dwarf" }),
      ],
    });
    expect(screen.getByText("The plains were quiet.")).toBeInTheDocument();
    expect(screen.getByText("Mrgl. No fish today.")).toHaveClass("margin-note");
    expect(screen.getByText("Version 2 of 2")).toBeInTheDocument();
    expect(screen.getByText("What happened")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Older" }));
    expect(screen.getByText("An older telling.")).toBeInTheDocument();
    expect(screen.getByText("Version 1 of 2")).toBeInTheDocument();
    expect(screen.getByText(/Written by The grumpy dwarf/)).toBeInTheDocument();
  });

  it("names a narrator the player removed plainly", () => {
    panel({
      versions: [story(1, "The water was kind.", { narrator: "custom_heron" })],
    });
    expect(
      screen.getByText(/Written by a narrator you removed/),
    ).toBeInTheDocument();
  });

  it("writes again with the narrator picked for this day", async () => {
    const { onWrite } = panel({
      versions: [story(1, "The plains were quiet.")],
    });
    const input = screen
      .getAllByLabelText("Narrator for this day")
      .find((el) => el.tagName === "INPUT");
    if (!input) throw new Error("no narrator input");
    fireEvent.click(input);
    fireEvent.click(
      await screen.findByRole("option", { name: "The grumpy dwarf" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Write again" }));
    expect(onWrite).toHaveBeenCalledWith("grumpy_dwarf");
  });

  it("shows the facts and the place in line while the murloc has not written yet", () => {
    panel({ job: job({ position: 3 }), blocked: "gameRunning" });
    expect(screen.getByText("Killed 23 Prairie Wolves.")).toBeInTheDocument();
    expect(
      screen.getByText(
        /The murloc hasn't written this one yet\. It is number 3 in line\. Writing waits while the game is running, so the game keeps running smoothly\. Close the game, or allow writing while playing in Settings\./,
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Write now" })).toBeEnabled();
  });

  it("shows the facts instead of an entry that failed the fact check, and why", () => {
    panel({
      versions: [
        story(1, "I sailed to Booty Bay.", {
          check: "failed",
          checkReason: "mentions Booty Bay not in the day's notes",
        }),
      ],
    });
    expect(
      screen.queryByText("I sailed to Booty Bay."),
    ).not.toBeInTheDocument();
    expect(screen.getByText("Killed 23 Prairie Wolves.")).toBeInTheDocument();
    expect(
      screen.getByText(
        /set aside because it mentions Booty Bay not in the day's notes/,
      ),
    ).toBeInTheDocument();
  });

  it("offers a retry for a failed job, in plain words", () => {
    const { onRetry } = panel({
      job: job({
        state: "failed",
        lastError: "could not reach gpu-box",
        position: null,
      }),
    });
    expect(
      screen.getByText(
        "Writing failed: could not reach gpu-box. Press Try again; if it fails again, check who writes in Settings.",
      ),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(onRetry).toHaveBeenCalled();
  });

  it("does not offer to write a day that is still going", () => {
    panel({ finished: false });
    expect(
      screen.getByText(
        "This day isn't over yet. The murloc writes it once it is.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Write now" }),
    ).not.toBeInTheDocument();
  });

  it("offers to tell today so far, and says the full entry still comes", () => {
    const onWriteSoFar = vi.fn();
    panel({ finished: false, onWriteSoFar });
    fireEvent.click(
      screen.getByRole("button", { name: "Write what happened so far" }),
    );
    expect(onWriteSoFar).toHaveBeenCalledWith(null);
  });

  it("marks an entry told so far as such", () => {
    panel({
      finished: false,
      onWriteSoFar: vi.fn(),
      versions: [story(1, "The morning was kind.", { partial: true })],
    });
    expect(screen.getByText("The morning was kind.")).toBeInTheDocument();
    expect(
      screen.getByText(/Told so far by The murloc scribe/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/The full entry is written once the day is over/),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Tell it again so far" }),
    ).toBeInTheDocument();
  });

  it("shows a day told so far waiting in line", () => {
    panel({ finished: false, onWriteSoFar: vi.fn(), job: job({}) });
    expect(screen.getByText(/number 3 in line/)).toBeInTheDocument();
  });
});

describe("EntryPanel, edited by hand", () => {
  const handlers = { onWrite: vi.fn(), onCancel: vi.fn(), onRetry: vi.fn() };

  it("saves the player's own version and keeps the box open when saving fails", async () => {
    const onEdit = vi
      .fn<(s: Story, text: string) => Promise<void>>()
      .mockRejectedValueOnce("disk full")
      .mockResolvedValueOnce(undefined);
    const base = story(1, "The model's words.");
    renderWithProviders(
      <EntryPanel
        {...handlers}
        kind="day"
        day="2026-11-08"
        finished
        versions={[base]}
        blocked={null}
        narrators={[]}
        onEdit={onEdit}
      />,
    );
    fireEvent.click(screen.getByText("Edit"));
    const box = screen.getByLabelText("Your version of the entry");
    expect(box).toHaveValue("The model's words.");
    expect(
      screen.getByText(/does not check your words against the day's notes/),
    ).toBeInTheDocument();
    fireEvent.change(box, { target: { value: "My own words." } });
    fireEvent.click(screen.getByText("Save my version"));
    expect(
      await screen.findByText(/Your version was not saved: disk full\./),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Your version of the entry")).toHaveValue(
      "My own words.",
    );
    fireEvent.click(screen.getByText("Save my version"));
    await waitFor(() => expect(onEdit).toHaveBeenCalledTimes(2));
    expect(onEdit).toHaveBeenLastCalledWith(base, "My own words.");
    await waitFor(() =>
      expect(screen.queryByLabelText("Your version of the entry")).toBeNull(),
    );
  });

  it("says an edited version was not checked against the notes", () => {
    renderWithProviders(
      <EntryPanel
        {...handlers}
        kind="day"
        day="2026-11-08"
        finished
        versions={[
          { ...story(2, "Mine."), check: "edited", provider: "player" },
        ]}
        blocked={null}
        narrators={[]}
        onEdit={async () => {}}
      />,
    );
    expect(screen.getByText("Mine.")).toBeInTheDocument();
    expect(
      screen.getByText(
        /Edited by you from .* entry, .*Not checked against the notes/,
      ),
    ).toBeInTheDocument();
  });

  it("offers no edit for a day told only so far, or a day still going", () => {
    renderWithProviders(
      <EntryPanel
        {...handlers}
        kind="day"
        day="2026-11-10"
        finished={false}
        versions={[{ ...story(3, "So far."), partial: true }]}
        blocked={null}
        narrators={[]}
        onWriteSoFar={() => {}}
        onEdit={async () => {}}
      />,
    );
    expect(screen.queryByText("Edit")).toBeNull();
  });
});
