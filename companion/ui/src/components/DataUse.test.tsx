import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithProviders } from "@/test/render";
import { DataUseDialog } from "./DataUse";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("DataUseDialog", () => {
  it("has a switch for sharing and one for the window per kind, and saves both", async () => {
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
});
