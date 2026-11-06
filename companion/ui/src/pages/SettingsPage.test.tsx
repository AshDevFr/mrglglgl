import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Settings } from "@/bindings/Settings";
import {
  defaultSite,
  defaultWriting,
  renderWithProviders,
} from "@/test/render";
import { SettingsPage } from "./SettingsPage";

vi.mock("@tanstack/react-router", () => ({ useNavigate: () => vi.fn() }));

const api = vi.hoisted(() => ({
  unreadable: [] as string[],
  saveSettings: vi.fn(async (_: Settings) => ({})),
}));

vi.mock("@/api/commands", () => ({
  getSettings: async () => ({
    settings: {
      wowRootOverride: "/Games/World of Warcraft",
      backupRetention: "days90",
      uiLanguage: null,
      site: defaultSite,
      writing: defaultWriting,
    },
    roots: [],
    unreadable: api.unreadable,
  }),
  saveSettings: api.saveSettings,
  listNarrators: async () => [],
  models: async () => ({
    chosenId: null,
    advice: { kind: "recommend", id: "x" },
    models: [],
  }),
  addonViews: async () => [],
  addonRelease: async () => ({
    release: null,
    prereleases: false,
    updates: [],
    checked: 0,
  }),
  showLog: async () => {},
  listCharacters: async () => [],
  hasApiKey: async () => false,
  onModelDownload: async () => () => {},
}));

beforeEach(() => {
  vi.clearAllMocks();
  api.unreadable = [];
  try {
    sessionStorage.clear();
  } catch {
    // jsdom always has it
  }
});

describe("SettingsPage", () => {
  it("opens on General and shows one tab's sections at a time", async () => {
    renderWithProviders(<SettingsPage />);
    expect(
      await screen.findByLabelText(/World of Warcraft folder/),
    ).toBeInTheDocument();
    expect(screen.getByText("Show the log file")).toBeInTheDocument();
    expect(screen.queryByText("Offer test versions")).toBeNull();

    fireEvent.click(screen.getByRole("tab", { name: "Addon" }));
    expect(await screen.findByText("Offer test versions")).toBeInTheDocument();
    expect(screen.queryByLabelText(/World of Warcraft folder/)).toBeNull();
  });

  it("remembers the tab when the page is opened again", async () => {
    const { unmount } = renderWithProviders(<SettingsPage />);
    fireEvent.click(await screen.findByRole("tab", { name: "Website" }));
    unmount();
    renderWithProviders(<SettingsPage />);
    expect(
      await screen.findByRole("tab", { name: "Website", selected: true }),
    ).toBeInTheDocument();
  });

  it("saves only the open tab's settings, keeping the rest as stored", async () => {
    renderWithProviders(<SettingsPage />);
    fireEvent.change(await screen.findByLabelText(/World of Warcraft folder/), {
      target: { value: "/Elsewhere" },
    });
    fireEvent.click(screen.getByRole("tab", { name: "Writing" }));
    fireEvent.click(await screen.findByText("Save"));
    await waitFor(() => expect(api.saveSettings).toHaveBeenCalled());
    const saved = api.saveSettings.mock.calls[0][0];
    expect(saved.wowRootOverride).toBe("/Games/World of Warcraft");
    expect(saved.writing).toEqual(defaultWriting);
  });

  it("names the settings that could not be read", async () => {
    api.unreadable = ["writing", "site"];
    renderWithProviders(<SettingsPage />);
    expect(
      await screen.findByText(
        /could not be read and are back to their defaults: writing, website\. Check them below/,
      ),
    ).toBeInTheDocument();
  });
});
