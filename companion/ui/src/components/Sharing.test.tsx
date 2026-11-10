import { Menu } from "@mantine/core";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Story } from "@/bindings/Story";
import { renderWithProviders } from "@/test/render";
import { CopyEntryButton, DebugExportItem, ExportDialog } from "./Sharing";

const commands = vi.hoisted(() => ({
  copyEntryText: vi.fn(
    async () =>
      "Sunday, November 8, 2026\n\nThe plains were quiet.\n\nvia Mrglglgl (github.com/AshDevFr/mrglglgl)",
  ),
  defaultExportFolder: vi.fn(async () => "/Users/wren/Documents"),
  exportCharacter: vi.fn(),
  showInFolder: vi.fn(async () => {}),
  exportDebug: vi.fn(
    async () => "/Users/wren/Documents/Wren-2026-11-08-day-debug.json",
  ),
}));
vi.mock("@/api/commands", () => commands);

const dialog = vi.hoisted(() => ({
  open: vi.fn(async () => "/Users/wren/Documents"),
}));
vi.mock("@tauri-apps/plugin-dialog", () => dialog);

const pages = vi.hoisted(() => ({
  loadExportData: vi.fn(async () => ({ title: "Wren Hollowbrook" })),
  exportPage: vi.fn(() => ({ css: ".journal-page{}", body: "<main></main>" })),
}));
vi.mock("@/lib/exportPage", () => pages);

const notices = vi.hoisted(() => ({
  notifications: { show: vi.fn() },
}));
vi.mock("@mantine/notifications", () => notices);

/** The debug item where it lives: in an open More menu. */
function debugItem() {
  renderWithProviders(
    <Menu opened>
      <Menu.Target>
        <button type="button">More</button>
      </Menu.Target>
      <Menu.Dropdown>
        <DebugExportItem story={story} />
      </Menu.Dropdown>
    </Menu>,
  );
  fireEvent.click(screen.getByRole("menuitem", { name: "Save a debug file…" }));
}

// A folder remembered by one test must not open the next one's dialog.
beforeEach(() => {
  localStorage.clear();
  notices.notifications.show.mockClear();
  commands.exportDebug.mockClear();
});

const story = {
  id: 7,
  characterId: 1,
  kind: "day",
  startDay: "2026-11-08",
} as Story;

describe("sharing", () => {
  it("writes a day's debug file into the folder chosen, and says where", async () => {
    debugItem();
    await waitFor(() => expect(notices.notifications.show).toHaveBeenCalled());
    expect(commands.exportDebug).toHaveBeenCalledWith(
      1,
      "day",
      "2026-11-08",
      "/Users/wren/Documents",
    );
    // The notification names the file, and offers to show it.
    const { message } = notices.notifications.show.mock.calls[0][0];
    renderWithProviders(message);
    expect(
      screen.getByText(/Wren-2026-11-08-day-debug\.json/),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Show in folder" }),
    ).toBeInTheDocument();
  });

  it("does nothing when no folder is chosen", async () => {
    dialog.open.mockResolvedValueOnce(null as unknown as string);
    debugItem();
    await waitFor(() => expect(dialog.open).toHaveBeenCalled());
    expect(commands.exportDebug).not.toHaveBeenCalled();
    expect(notices.notifications.show).not.toHaveBeenCalled();
  });

  it("copies the entry with its day and footer", async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
    renderWithProviders(<CopyEntryButton story={story} />);
    fireEvent.click(screen.getByRole("button", { name: "Copy entry" }));
    expect(
      await screen.findByRole("button", { name: "Copied" }),
    ).toBeInTheDocument();
    expect(commands.copyEntryText).toHaveBeenCalledWith(1, 7);
    expect(writeText).toHaveBeenCalledWith(
      expect.stringMatching(/^Sunday, November 8, 2026\n\n.*\n\nvia Mrglglgl/s),
    );
  });

  it("exports to the folder shown, and says plainly when it cannot", async () => {
    commands.exportCharacter.mockRejectedValueOnce(
      "exports cannot be saved inside the World of Warcraft folder (/Games/World of Warcraft); pick another folder",
    );
    commands.exportCharacter.mockResolvedValueOnce(
      "/Users/wren/Documents/Wren-Hollowbrook-journal.md",
    );
    renderWithProviders(
      <ExportDialog
        characterId={1}
        opened
        onClose={vi.fn()}
        onPrint={vi.fn()}
      />,
    );
    fireEvent.click(await screen.findByRole("radio", { name: /Markdown/ }));
    const folder = await screen.findByLabelText("Folder");
    await waitFor(() => expect(folder).toHaveValue("/Users/wren/Documents"));
    fireEvent.change(folder, {
      target: { value: "/Games/World of Warcraft/_forever_" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Export" }));
    expect(
      await screen.findByText(
        /The export was not saved: exports cannot be saved inside the World of Warcraft folder/,
      ),
    ).toBeInTheDocument();
    fireEvent.change(folder, { target: { value: "/Users/wren/Documents" } });
    fireEvent.click(screen.getByRole("button", { name: "Export" }));
    expect(
      await screen.findByText(
        "Saved /Users/wren/Documents/Wren-Hollowbrook-journal.md.",
      ),
    ).toBeInTheDocument();
    expect(commands.exportCharacter).toHaveBeenLastCalledWith(
      1,
      "/Users/wren/Documents",
      "markdown",
      null,
    );
    fireEvent.click(screen.getByRole("button", { name: "Show in folder" }));
    expect(commands.showInFolder).toHaveBeenCalledWith(
      "/Users/wren/Documents/Wren-Hollowbrook-journal.md",
    );
  });

  it("exports a web page laid out like the app, pictures inside", async () => {
    commands.exportCharacter.mockResolvedValueOnce(
      "/Users/wren/Documents/Wren-Hollowbrook-journal.html",
    );
    renderWithProviders(
      <ExportDialog
        characterId={1}
        opened
        onClose={vi.fn()}
        onPrint={vi.fn()}
      />,
    );
    expect(
      await screen.findByRole("radio", { name: /Web page/ }),
    ).toBeChecked();
    await waitFor(() =>
      expect(screen.getByLabelText("Folder")).toHaveValue(
        "/Users/wren/Documents",
      ),
    );
    fireEvent.click(screen.getByRole("button", { name: "Export" }));
    await waitFor(() =>
      expect(commands.exportCharacter).toHaveBeenLastCalledWith(
        1,
        "/Users/wren/Documents",
        "html",
        { css: ".journal-page{}", body: "<main></main>" },
      ),
    );
    expect(pages.loadExportData).toHaveBeenCalledWith(1, true);
  });

  it("sends a PDF through the print dialog, with no folder to pick", async () => {
    const onPrint = vi.fn();
    renderWithProviders(
      <ExportDialog
        characterId={1}
        opened
        onClose={vi.fn()}
        onPrint={onPrint}
      />,
    );
    fireEvent.click(await screen.findByRole("radio", { name: /PDF/ }));
    expect(screen.queryByLabelText("Folder")).toBeNull();
    expect(screen.getByText(/Save as PDF/)).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Open the print dialog" }),
    );
    expect(onPrint).toHaveBeenCalled();
  });

  it("opens on the folder of the last export", async () => {
    commands.exportCharacter.mockResolvedValueOnce(
      "/Users/wren/Journals/Wren-Hollowbrook-journal.html",
    );
    const first = renderWithProviders(
      <ExportDialog
        characterId={1}
        opened
        onClose={vi.fn()}
        onPrint={vi.fn()}
      />,
    );
    const folder = await screen.findByLabelText("Folder");
    await waitFor(() => expect(folder).toHaveValue("/Users/wren/Documents"));
    fireEvent.change(folder, { target: { value: "/Users/wren/Journals" } });
    fireEvent.click(screen.getByRole("button", { name: "Export" }));
    await screen.findByText(/Saved \/Users\/wren\/Journals/);
    first.unmount();

    renderWithProviders(
      <ExportDialog
        characterId={1}
        opened
        onClose={vi.fn()}
        onPrint={vi.fn()}
      />,
    );
    await waitFor(() =>
      expect(screen.getByLabelText("Folder")).toHaveValue(
        "/Users/wren/Journals",
      ),
    );
  });
});
