import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithProviders } from "@/test/render";
import { DataUseDialog } from "./DataUse";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("DataUseDialog", () => {
  it("has a switch for sharing and one for the app per kind, and saves both", async () => {
    const calls = vi.mocked(invoke);
    calls.mockImplementation(async (command) =>
      command === "character_data_use" ? { shared: ["chat"], view: [] } : null,
    );
    renderWithProviders(
      <DataUseDialog characterId={3} opened onClose={() => {}} />,
    );
    const box = (label: string) => screen.getByLabelText(label);
    await waitFor(() =>
      expect(box("Chat: Narrators, website and exports")).not.toBeChecked(),
    );
    expect(box("Chat: What happened")).toBeChecked();
    fireEvent.click(box("Money and the auction house: What happened"));
    fireEvent.click(box("Chat: Narrators, website and exports"));
    fireEvent.click(box("Fishing: Narrators, website and exports"));
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(calls).toHaveBeenCalledWith("set_character_data_use", {
        characterId: 3,
        choice: { shared: ["fishing"], view: ["money"] },
      }),
    );
  });

  it("turns a whole column off or on from its heading", async () => {
    const calls = vi.mocked(invoke);
    calls.mockImplementation(async (command) =>
      command === "character_data_use" ? { shared: ["chat"], view: [] } : null,
    );
    renderWithProviders(
      <DataUseDialog characterId={3} opened onClose={() => {}} />,
    );
    const all = (column: string) =>
      screen.getByLabelText(`Turn every kind on or off: ${column}`);
    // One kind off: the column's box is mixed, neither all on nor all off.
    await waitFor(() =>
      expect(all("Narrators, website and exports")).toHaveAttribute(
        "data-indeterminate",
        "true",
      ),
    );
    fireEvent.click(all("What happened"));
    expect(screen.getByLabelText("Fishing: What happened")).not.toBeChecked();
    fireEvent.click(all("Narrators, website and exports"));
    expect(
      screen.getByLabelText("Chat: Narrators, website and exports"),
    ).toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(calls).toHaveBeenCalledWith("set_character_data_use", {
        characterId: 3,
        choice: {
          shared: [],
          view: expect.arrayContaining(["chat", "fishing", "money"]),
        },
      }),
    );
    const [, args] =
      calls.mock.calls.find(([c]) => c === "set_character_data_use") ?? [];
    expect((args as { choice: { view: string[] } }).choice.view).toHaveLength(
      20,
    );
  });
});
