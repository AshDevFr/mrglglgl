import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { WebsitePanel } from "@/components/WebsitePanel";
import { defaultSite, renderWithProviders } from "@/test/render";

const api = vi.hoisted(() => ({
  saveSiteSettings: vi.fn(async () => {}),
  // The characters on the site, as chosen under Settings, Characters.
  getSettings: vi.fn(async () => ({ settings: { site: { characters: [1] } } })),
  copySiteTheme: vi.fn(async () => ["base.html", "post.html"]),
  listCharacters: vi.fn(async () => [
    {
      character: {
        id: 1,
        name: "Wren",
        surname: "Hollowbrook",
        characterFolder: "Wren",
      },
      archived: false,
      milestones: 0,
      toReview: 0,
      level: null,
    },
    {
      character: {
        id: 2,
        name: "Ash",
        surname: "Earthborn",
        characterFolder: "Ash",
      },
      archived: false,
      milestones: 0,
      toReview: 0,
      level: null,
    },
  ]),
}));
vi.mock("@/api/commands", () => ({
  ...api,
  openSite: site.openSite,
  showSiteFolder: site.showSiteFolder,
}));
const site = vi.hoisted(() => ({
  openSite: vi.fn(async () => {}),
  showSiteFolder: vi.fn(async () => {}),
}));
const build = vi.hoisted(() => ({
  rebuildSite: vi.fn(async () => ({
    at: 0,
    report: { written: 3, unchanged: 2, removed: 1 },
    error: null,
  })),
}));
vi.mock("@/lib/siteBuild", async (original) => ({
  ...(await original<typeof import("@/lib/siteBuild")>()),
  rebuildSite: build.rebuildSite,
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

beforeEach(() => vi.clearAllMocks());

describe("WebsitePanel", () => {
  it("saves the site's folder, title and privacy, keeping its characters as chosen", async () => {
    renderWithProviders(<WebsitePanel site={defaultSite} />);
    fireEvent.click(screen.getByLabelText(/Keep a website up to date/));
    fireEvent.change(screen.getByLabelText("Site folder"), {
      target: { value: "/sites/wren" },
    });
    fireEvent.change(screen.getByLabelText("Site title"), {
      target: { value: "Wren's travels" },
    });
    // The characters are chosen under Settings, Characters, not here.
    expect(screen.queryByLabelText("Wren Hollowbrook")).toBeNull();
    expect(
      screen.getByText(/No character is on the site yet/),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/stand-in names/)).toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(api.saveSiteSettings).toHaveBeenCalledWith({
        ...defaultSite,
        enabled: true,
        folder: "/sites/wren",
        title: "Wren's travels",
        characters: [1],
      }),
    );
  });

  it("builds now and says what changed", async () => {
    renderWithProviders(
      <WebsitePanel
        site={{ ...defaultSite, folder: "/sites/wren", characters: [1] }}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Build now" }));
    expect(
      await screen.findByText(/3 files written, 2 unchanged, 1 removed/),
    ).toBeInTheDocument();
    expect(api.saveSiteSettings).toHaveBeenCalled();
    expect(build.rebuildSite).toHaveBeenCalledWith(
      expect.objectContaining({ folder: "/sites/wren", characters: [1] }),
    );
  });

  it("says why an enabled site is not built yet", () => {
    renderWithProviders(
      <WebsitePanel site={{ ...defaultSite, enabled: true }} />,
    );
    expect(
      screen.getByText(
        /Choose a folder, and at least one character under Settings, Characters: until then nothing is built/,
      ),
    ).toBeInTheDocument();
  });

  it("opens the built site in the browser, or shows its folder", async () => {
    renderWithProviders(
      <WebsitePanel
        site={{ ...defaultSite, folder: "/sites/wren", characters: [1] }}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Build now" }));
    fireEvent.click(
      await screen.findByRole("button", { name: "Open in the browser" }),
    );
    expect(site.openSite).toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Show the folder" }));
    expect(site.showSiteFolder).toHaveBeenCalled();
  });

  it("copies the default theme into the theme folder", async () => {
    renderWithProviders(
      <WebsitePanel site={{ ...defaultSite, themeFolder: "/themes/mine" }} />,
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Copy the default theme here" }),
    );
    await waitFor(() =>
      expect(api.copySiteTheme).toHaveBeenCalledWith("/themes/mine"),
    );
    expect(await screen.findByText(/2 theme files copied/)).toBeInTheDocument();
  });
});
