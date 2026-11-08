import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { WebsitePanel } from "@/components/WebsitePanel";
import { defaultSite, renderWithProviders } from "@/test/render";

const api = vi.hoisted(() => ({
  saveSiteSettings: vi.fn(async () => {}),
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
vi.mock("@/api/commands", () => api);
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
  it("saves the site's folder, title, characters and privacy", async () => {
    renderWithProviders(<WebsitePanel site={defaultSite} />);
    fireEvent.click(screen.getByLabelText(/Keep a website up to date/));
    fireEvent.change(screen.getByLabelText("Site folder"), {
      target: { value: "/sites/wren" },
    });
    fireEvent.change(screen.getByLabelText("Site title"), {
      target: { value: "Wren's travels" },
    });
    fireEvent.click(await screen.findByLabelText("Wren Hollowbrook"));
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
