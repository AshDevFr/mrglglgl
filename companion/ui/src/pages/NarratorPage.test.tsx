import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Narrator } from "@/bindings/Narrator";
import { renderWithProviders } from "@/test/render";
import { NarratorPage } from "./NarratorPage";

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => vi.fn(),
  useRouter: () => ({ history: { canGoBack: () => false, back: vi.fn() } }),
}));

const blank: Narrator = {
  id: "",
  language: "en",
  name: "",
  opt_in: false,
  persona: "",
  voice: "",
  murloc: "The murloc may show in passing.",
  margin_note: "Add a footnote starting with *.",
  example: {
    intro: "Example:",
    notes_label: "Notes:",
    entry_label: "Entry:",
    notes: "Fished twice.",
    entry: "",
    margin_note: "",
  },
  hints: {},
};

const commands = vi.hoisted(() => ({
  blankNarrator: vi.fn(),
  narrator: vi.fn(),
  saveNarrator: vi.fn(),
}));

vi.mock("@/api/commands", () => ({
  ...commands,
  previewNarrator: vi.fn(),
  cancelPreview: vi.fn(),
  listCharacters: () => Promise.resolve([]),
  characterJournal: () => Promise.resolve(null),
  onNarratorPreview: () => Promise.resolve(() => {}),
}));

describe("NarratorPage", () => {
  it("opens a new narrator blank, with the made-up day to write from", async () => {
    commands.blankNarrator.mockResolvedValue(blank);
    commands.saveNarrator.mockResolvedValue({
      kind: "invalid",
      problems: [{ field: "name", problem: "missing" }],
    });
    renderWithProviders(<NarratorPage narratorId="new" />);
    expect(
      await screen.findByDisplayValue("Fished twice."),
    ).toBeInTheDocument();
    expect(commands.narrator).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(commands.saveNarrator).toHaveBeenCalledWith(blank),
    );
  });
});
