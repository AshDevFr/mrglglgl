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
    commands.exportCharacter.mockResolvedValueOnce({
      markdown: "/Users/wren/Documents/Wren-Hollowbrook-journal.md",
      html: "/Users/wren/Documents/Wren-Hollowbrook-journal.html",
    });
    renderWithProviders(
      <ExportDialog characterId={1} opened onClose={vi.fn()} />,
    );
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
        /Saved \/Users\/wren\/Documents\/Wren-Hollowbrook-journal\.md and/,
      ),
    ).toBeInTheDocument();
    expect(commands.exportCharacter).toHaveBeenLastCalledWith(
      1,
      "/Users/wren/Documents",
    );
  });
});
