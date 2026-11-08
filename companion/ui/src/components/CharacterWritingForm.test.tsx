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
  listNarrators: vi.fn(async () => [
    {
      id: "murloc_scribe",
      name: "The murloc scribe",
      optIn: false,
      custom: false,
    },
    {
      id: "easygoing_troll",
      name: "The easygoing troll",
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
});
