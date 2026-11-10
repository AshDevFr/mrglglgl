import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CharacterWritingForm } from "@/components/CharacterWritingForm";
import { defaultWriting, renderWithProviders } from "@/test/render";

const commands = vi.hoisted(() => ({
  own: { narrator: null, meddling: null, preferences: "" } as Record<
    string,
    unknown
  >,
  characterWriting: vi.fn(),
  setCharacterWriting: vi.fn(async () => {}),
  getSettings: vi.fn(),
  race: "Human",
  listCharacters: vi.fn(),
  listNarrators: vi.fn(async () => [
    {
      id: "murloc_scribe",
      name: "The murloc scribe",
      icon: "fish-monster",
      optIn: false,
      custom: false,
    },
    {
      id: "easygoing_troll",
      name: "The easygoing troll",
      icon: "troll",
      optIn: false,
      custom: false,
    },
  ]),
}));
vi.mock("@/api/commands", () => commands);

beforeEach(() => {
  vi.clearAllMocks();
  commands.own = { narrator: null, meddling: null, preferences: "" };
  commands.characterWriting.mockImplementation(async () => commands.own);
  // A race with no narrator among the ones listed here offers none.
  commands.race = "Human";
  commands.listCharacters.mockImplementation(async () => [
    { character: { id: 2, race: commands.race }, level: 20 },
  ]);
  commands.getSettings.mockImplementation(async () => ({
    settings: { writing: { ...defaultWriting, preferences: "No em dashes." } },
  }));
});

const choose = async (label: string, option: string) => {
  const input = screen
    .getAllByLabelText(label)
    .find((el) => el.tagName === "INPUT");
  if (!input) throw new Error(`no ${label} input`);
  fireEvent.click(input);
  fireEvent.click(
    await screen.findByRole("option", { name: option, hidden: true }),
  );
};

describe("CharacterWritingForm", () => {
  it("starts from Settings and says what Settings would use", async () => {
    renderWithProviders(<CharacterWritingForm characterId={2} />);
    expect(
      await screen.findByDisplayValue("As in Settings (The murloc scribe)"),
    ).toBeInTheDocument();
    expect(screen.getByText(/No em dashes\./)).toBeInTheDocument();
  });

  it("saves the character's own narrator, margin notes and added preferences", async () => {
    renderWithProviders(<CharacterWritingForm characterId={2} />);
    await screen.findByDisplayValue("As in Settings (The murloc scribe)");
    await choose("Narrator", "The easygoing troll");
    await choose("Murloc margin notes", "Never");
    fireEvent.change(
      screen.getByRole("textbox", { name: /Writing preferences/ }),
      {
        target: { value: "Mention the loa now and then." },
      },
    );
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(commands.setCharacterWriting).toHaveBeenCalledWith(2, {
        narrator: "easygoing_troll",
        meddling: "off",
        preferences: "Mention the loa now and then.",
      }),
    );
    expect(await screen.findByText(/Saved/)).toBeInTheDocument();
  });

  it("goes back to Settings' choice", async () => {
    commands.own = {
      narrator: "easygoing_troll",
      meddling: "off",
      preferences: "",
    };
    renderWithProviders(<CharacterWritingForm characterId={2} />);
    await screen.findByDisplayValue("The easygoing troll");
    await choose("Narrator", "As in Settings (The murloc scribe)");
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(commands.setCharacterWriting).toHaveBeenCalledWith(2, {
        narrator: null,
        meddling: "off",
        preferences: "",
      }),
    );
  });

  it("offers the narrator made for the character's race, until it writes", async () => {
    commands.race = "Troll";
    renderWithProviders(<CharacterWritingForm characterId={2} />);
    expect(
      await screen.findByText(
        "Made for this character's race: The easygoing troll.",
      ),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Use it" }));
    expect(
      await screen.findByDisplayValue("The easygoing troll"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Made for this character's race/)).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(commands.setCharacterWriting).toHaveBeenCalledWith(
        2,
        expect.objectContaining({ narrator: "easygoing_troll" }),
      ),
    );
  });
});
