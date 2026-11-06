import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Story } from "@/bindings/Story";
import { renderWithProviders } from "@/test/render";
import { CopyEntryButton, ExportDialog } from "./Sharing";

const commands = vi.hoisted(() => ({
  copyEntryText: vi.fn(
    async () =>
      "Sunday, November 8, 2026\n\nThe plains were quiet.\n\nvia Mrglglgl (github.com/AshDevFr/mrglglgl)",
  ),
  defaultExportFolder: vi.fn(async () => "/Users/wren/Documents"),
  exportCharacter: vi.fn(),
}));
vi.mock("@/api/commands", () => commands);

const pages = vi.hoisted(() => ({
  loadExportData: vi.fn(async () => ({ title: "Wren Hollowbrook" })),
  exportPage: vi.fn(() => ({ css: ".journal-page{}", body: "<main></main>" })),
}));
vi.mock("@/lib/exportPage", () => pages);

const story = { id: 7, characterId: 1 } as Story;

describe("sharing", () => {
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
});
