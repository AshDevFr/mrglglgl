import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { NarratorsPanel } from "@/components/NarratorsPanel";
import { renderWithProviders } from "@/test/render";

const commands = vi.hoisted(() => ({
  copyNarrator: vi.fn(),
  saveNarrator: vi.fn(),
  importNarrator: vi.fn(),
  deleteNarrator: vi.fn(() => Promise.resolve()),
  exportNarrator: vi.fn(() =>
    Promise.resolve("/Users/me/Documents/custom_heron.toml"),
  ),
}));

vi.mock("@/api/commands", () => ({
  listNarrators: () =>
    Promise.resolve([
      {
        id: "grumpy_dwarf",
        name: "The grumpy dwarf",
        optIn: false,
        custom: false,
      },
      { id: "custom_heron", name: "The heron", optIn: false, custom: true },
    ]),
  defaultExportFolder: () => Promise.resolve("/Users/me/Documents"),
  ...commands,
}));

beforeEach(() => vi.clearAllMocks());

const row = (name: string) => screen.findByRole("listitem", { name });

describe("NarratorsPanel", () => {
  it("offers a copy of a built-in and saves it before opening it", async () => {
    commands.copyNarrator.mockResolvedValue({
      id: "",
      name: "The grumpy dwarf (mine)",
    });
    commands.saveNarrator.mockResolvedValue({
      kind: "saved",
      narrator: { id: "custom_the_grumpy_dwarf_mine" },
    });
    const onEdit = vi.fn();
    renderWithProviders(<NarratorsPanel onEdit={onEdit} />);
    const dwarf = await row("The grumpy dwarf");
    expect(within(dwarf).queryByRole("button", { name: "Delete" })).toBeNull();
    fireEvent.click(within(dwarf).getByRole("button", { name: "Edit a copy" }));
    await waitFor(() =>
      expect(onEdit).toHaveBeenCalledWith("custom_the_grumpy_dwarf_mine"),
    );
    expect(commands.copyNarrator).toHaveBeenCalledWith("grumpy_dwarf");
  });

  it("edits, exports and removes the player's own", async () => {
    const onEdit = vi.fn();
    renderWithProviders(<NarratorsPanel onEdit={onEdit} />);
    const heron = await row("The heron");
    fireEvent.click(within(heron).getByRole("button", { name: "Edit" }));
    expect(onEdit).toHaveBeenCalledWith("custom_heron");

    fireEvent.click(within(heron).getByRole("button", { name: "Export" }));
    const dialog = await screen.findByRole("dialog");
    await waitFor(() =>
      expect(within(dialog).getByLabelText("Folder")).toHaveValue(
        "/Users/me/Documents",
      ),
    );
    fireEvent.click(within(dialog).getByRole("button", { name: "Export" }));
    expect(
      await within(dialog).findByText(
        /Saved to \/Users\/me\/Documents\/custom_heron.toml/,
      ),
    ).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: "Close" }));

    fireEvent.click(within(heron).getByRole("button", { name: "Delete" }));
    fireEvent.click(await screen.findByRole("button", { name: "Remove" }));
    await waitFor(() =>
      expect(commands.deleteNarrator).toHaveBeenCalledWith("custom_heron"),
    );
  });

  it("imports a narrator file", async () => {
    commands.importNarrator.mockResolvedValue({
      kind: "saved",
      narrator: { id: "custom_otter", name: "The otter" },
    });
    renderWithProviders(<NarratorsPanel onEdit={vi.fn()} />);
    const input = await screen.findByLabelText(/Import a narrator file/);
    const file = new File(['id = "otter"'], "otter.toml", {
      type: "text/plain",
    });
    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() =>
      expect(commands.importNarrator).toHaveBeenCalledWith('id = "otter"'),
    );
    expect(await screen.findByText("The otter was added.")).toBeInTheDocument();
  });
});
