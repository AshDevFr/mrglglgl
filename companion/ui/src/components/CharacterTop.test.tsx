import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CharacterTop } from "@/components/CharacterTop";
import { renderWithProviders, syntheticJournal } from "@/test/render";

vi.mock("@tanstack/react-router", () => ({ useNavigate: () => vi.fn() }));
vi.mock("@/api/commands", () => ({
  listCharacters: async () => [
    {
      character: {
        id: 1,
        name: "Wren",
        surname: "Hollowbrook",
        characterFolder: "Wren-Hollowbrook",
        lastSynced: 1794000000,
      },
      archived: false,
      milestones: 6,
      toReview: 2,
    },
  ],
  characterJournal: async () => syntheticJournal,
  writingStatus: async () => ({
    paused: false,
    current: null,
    jobs: [],
    blocked: null,
  }),
  defaultExportFolder: async () => null,
}));

describe("CharacterTop", () => {
  it("keeps the character's details and actions on every tab", async () => {
    renderWithProviders(<CharacterTop characterId={1} current="review" />);
    expect(
      await screen.findByRole("heading", {
        level: 1,
        name: "Wren Hollowbrook",
      }),
    ).toBeInTheDocument();
    expect(await screen.findByText(/^Level \d+$/)).toBeInTheDocument();
    expect(screen.getByText("1 boot kept")).toBeInTheDocument();
    expect(screen.getByText(/^Last synced /)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Export" })).toBeInTheDocument();
    expect(
      screen.getByText(/To review/).closest("[aria-current]"),
    ).toHaveAttribute("aria-current", "page");
  });
});
