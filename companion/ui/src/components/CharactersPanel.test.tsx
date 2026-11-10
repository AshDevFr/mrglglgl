import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { defaultSite, renderWithProviders } from "@/test/render";
import { CharactersPanel } from "./CharactersPanel";

const character = (id: number, name: string, enabled: boolean) => ({
  character: {
    id,
    name,
    surname: null,
    characterFolder: name,
    account: "ACCOUNT1",
    realm: "Test Realm",
    realmFolder: "93",
    enabled,
  },
  archived: false,
  milestones: 0,
  toReview: 0,
  level: null,
});

const api = vi.hoisted(() => ({
  site: { characters: [1] as number[] },
  listCharacters: vi.fn(),
  getSettings: vi.fn(),
  saveSiteSettings: vi.fn(async () => {}),
  setCharacterEnabled: vi.fn(async () => {}),
}));
vi.mock("@/api/commands", () => api);

beforeEach(() => {
  vi.clearAllMocks();
  api.site = { characters: [1] };
  api.listCharacters.mockResolvedValue([
    character(1, "Ash", true),
    character(2, "Penny", true),
    character(3, "Brakka", false),
  ]);
  api.getSettings.mockImplementation(async () => ({
    settings: { site: { ...defaultSite, ...api.site } },
  }));
});

const box = (label: string) => screen.findByLabelText(label);

describe("CharactersPanel", () => {
  it("shows each character in the app or not, and on the site or not", async () => {
    renderWithProviders(<CharactersPanel />);
    await waitFor(async () =>
      expect(await box("Ash: On the site")).toBeChecked(),
    );
    expect(await box("Ash: In the app")).toBeChecked();
    expect(await box("Penny: On the site")).not.toBeChecked();
    // Out of the app, the site is not a choice.
    expect(await box("Brakka: In the app")).not.toBeChecked();
    expect(await box("Brakka: On the site")).toBeDisabled();
  });

  it("puts a character on the site, keeping the others there", async () => {
    renderWithProviders(<CharactersPanel />);
    fireEvent.click(await box("Penny: On the site"));
    await waitFor(() =>
      expect(api.saveSiteSettings).toHaveBeenCalledWith(
        expect.objectContaining({ characters: [1, 2] }),
      ),
    );
  });

  it("takes a character out of the app, and off the site with it", async () => {
    renderWithProviders(<CharactersPanel />);
    await waitFor(async () =>
      expect(await box("Ash: On the site")).toBeChecked(),
    );
    fireEvent.click(await box("Ash: In the app"));
    await waitFor(() =>
      expect(api.setCharacterEnabled).toHaveBeenCalledWith(1, false),
    );
    await waitFor(() =>
      expect(api.saveSiteSettings).toHaveBeenCalledWith(
        expect.objectContaining({ characters: [] }),
      ),
    );
  });

  it("brings a character back into the app", async () => {
    renderWithProviders(<CharactersPanel />);
    fireEvent.click(await box("Brakka: In the app"));
    await waitFor(() =>
      expect(api.setCharacterEnabled).toHaveBeenCalledWith(3, true),
    );
    expect(api.saveSiteSettings).not.toHaveBeenCalled();
  });
});
