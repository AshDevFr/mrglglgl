import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Narrator } from "@/bindings/Narrator";
import type { PreviewEvent } from "@/bindings/PreviewEvent";
import { NarratorEditor } from "@/components/NarratorEditor";
import { renderWithProviders } from "@/test/render";

const commands = vi.hoisted(() => ({
  saveNarrator: vi.fn(),
  previewNarrator: vi.fn(),
  cancelPreview: vi.fn(() => Promise.resolve(true)),
  listener: null as ((e: PreviewEvent) => void) | null,
}));

vi.mock("@/api/commands", () => ({
  saveNarrator: commands.saveNarrator,
  previewNarrator: commands.previewNarrator,
  cancelPreview: commands.cancelPreview,
  listCharacters: () => Promise.resolve([]),
  characterJournal: () => Promise.resolve(null),
  onNarratorPreview: (handler: (e: PreviewEvent) => void) => {
    commands.listener = handler;
    return Promise.resolve(() => {});
  },
}));

const heron: Narrator = {
  id: "custom_heron",
  language: "en",
  name: "The heron",
  opt_in: false,
  point_of_view: "adventurer",
  persona: "You are a heron who keeps this journal.",
  voice: "Patient, still, fond of water.",
  murloc: "No murloc.",
  margin_note: "Add a footnote starting with *.",
  example: {
    intro: "Example:",
    notes_label: "Notes:",
    entry_label: "Entry:",
    notes: "Fished twice.",
    entry: "I stood in the shallows and waited.",
    margin_note: "* Mrgl.",
  },
  hints: { only_fished: "A perfect day." },
};

beforeEach(() => {
  vi.clearAllMocks();
});

describe("NarratorEditor", () => {
  it("puts the voice up front and folds the rest under More", () => {
    renderWithProviders(<NarratorEditor initial={heron} />);
    expect(screen.getByLabelText("Name")).toHaveValue("The heron");
    expect(screen.getByLabelText(/Who is writing/)).toHaveValue(heron.persona);
    expect(screen.getByLabelText(/^Voice/)).toHaveValue(heron.voice);
    expect(screen.getByLabelText(/Example entry/)).toHaveValue(
      heron.example.entry,
    );
    const more = screen.getByText("More").closest("details");
    expect(more).not.toHaveAttribute("open");
    expect(screen.getByLabelText(/Only fishing/)).toHaveValue("A perfect day.");
  });

  it("saves the edited narrator", async () => {
    commands.saveNarrator.mockResolvedValue({ kind: "saved", narrator: heron });
    const onSaved = vi.fn();
    renderWithProviders(<NarratorEditor initial={heron} onSaved={onSaved} />);
    fireEvent.change(screen.getByLabelText(/^Voice/), {
      target: { value: "Quiet as reeds." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(commands.saveNarrator).toHaveBeenCalledWith({
        ...heron,
        voice: "Quiet as reeds.",
      }),
    );
    await waitFor(() => expect(onSaved).toHaveBeenCalledWith(heron));
  });

  it("lets the narrator write about the adventurer instead of as them", async () => {
    commands.saveNarrator.mockResolvedValue({ kind: "saved", narrator: heron });
    renderWithProviders(<NarratorEditor initial={heron} />);
    fireEvent.click(
      screen.getByRole("radio", { name: "About the adventurer" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(commands.saveNarrator).toHaveBeenCalledWith({
        ...heron,
        point_of_view: "observer",
      }),
    );
  });

  it("names each field to fix", async () => {
    commands.saveNarrator.mockResolvedValue({
      kind: "invalid",
      problems: [
        { field: "persona", problem: "missing" },
        { field: "example.margin_note", problem: "noStar" },
      ],
    });
    renderWithProviders(<NarratorEditor initial={heron} />);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(
      await screen.findByText("Write something here."),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Start it with \* as entries' margin notes do/),
    ).toBeInTheDocument();
    expect(screen.getByText(/Some fields need fixing/)).toBeInTheDocument();
  });

  it("previews the unsaved narrator on the sample day without a journal", async () => {
    let finish: (v: unknown) => void = () => {};
    commands.previewNarrator.mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    renderWithProviders(<NarratorEditor initial={heron} />);
    fireEvent.change(screen.getByLabelText(/^Voice/), {
      target: { value: "Quiet as reeds." },
    });
    fireEvent.click(await screen.findByRole("button", { name: "Preview" }));
    await waitFor(() =>
      expect(commands.previewNarrator).toHaveBeenCalledWith(
        { ...heron, voice: "Quiet as reeds." },
        { kind: "sample" },
      ),
    );
    commands.listener?.({ type: "chunk", text: "The water " });
    expect(await screen.findByText(/The water/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    expect(commands.cancelPreview).toHaveBeenCalled();
    finish({
      outcome: "written",
      text: "The water was kind.",
      check: "passed",
      reason: null,
    });
    expect(await screen.findByText("The water was kind.")).toBeInTheDocument();
  });
});
