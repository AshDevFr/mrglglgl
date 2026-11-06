import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { FolderField } from "@/components/FolderField";
import { renderWithProviders } from "@/test/render";

const open = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({ open }));

describe("FolderField", () => {
  it("picks a folder with the system chooser, starting where it is", async () => {
    open.mockResolvedValue("/Users/me/Desktop");
    const onChange = vi.fn();
    renderWithProviders(
      <FolderField
        label="Folder"
        value="/Users/me/Documents"
        onChange={onChange}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Choose…" }));
    await waitFor(() =>
      expect(onChange).toHaveBeenCalledWith("/Users/me/Desktop"),
    );
    expect(open).toHaveBeenCalledWith(
      expect.objectContaining({
        directory: true,
        defaultPath: "/Users/me/Documents",
      }),
    );
  });

  it("keeps the folder when the chooser is closed", async () => {
    open.mockResolvedValue(null);
    const onChange = vi.fn();
    renderWithProviders(
      <FolderField
        label="Folder"
        value="/Users/me/Documents"
        onChange={onChange}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Choose…" }));
    await waitFor(() => expect(open).toHaveBeenCalled());
    expect(onChange).not.toHaveBeenCalled();
  });
});
