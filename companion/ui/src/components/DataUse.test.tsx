import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithProviders } from "@/test/render";
import { DataUseDialog } from "./DataUse";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("DataUseDialog", () => {
  it("shows every kind on except those set aside, and saves what is turned off", async () => {
    const calls = vi.mocked(invoke);
    calls.mockImplementation(async (command) =>
      command === "character_unused_data" ? ["chat"] : null,
    );
    renderWithProviders(
      <DataUseDialog characterId={3} opened onClose={() => {}} />,
    );
    await waitFor(() =>
      expect(screen.getByLabelText("Chat")).not.toBeChecked(),
    );
    expect(screen.getByLabelText("Fishing")).toBeChecked();
    fireEvent.click(screen.getByLabelText("Money and the auction house"));
    fireEvent.click(screen.getByLabelText("Chat"));
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(calls).toHaveBeenCalledWith("set_character_unused_data", {
        characterId: 3,
        unused: ["money"],
      }),
    );
  });
});
