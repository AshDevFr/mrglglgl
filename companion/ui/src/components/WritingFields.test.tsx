import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { defaultWriting, renderWithProviders } from "@/test/render";
import { WritingFields } from "./WritingFields";

const commands = vi.hoisted(() => ({
  setWritingPaused: vi.fn(async () => {}),
}));
vi.mock("@/api/commands", () => commands);

describe("WritingFields", () => {
  it("pauses background writing at once, without waiting for Save", async () => {
    const onChange = vi.fn();
    renderWithProviders(
      <WritingFields
        value={defaultWriting}
        onChange={onChange}
        narrators={[]}
      />,
    );
    fireEvent.click(
      screen.getByRole("switch", { name: /^Pause background writing/ }),
    );
    await waitFor(() =>
      expect(commands.setWritingPaused).toHaveBeenCalledWith(true),
    );
    // The form keeps the same value, so a later Save does not undo it.
    expect(onChange).toHaveBeenCalledWith({ ...defaultWriting, paused: true });
  });

  it("turns weekly chapters off, kept for Save like the other choices", () => {
    const onChange = vi.fn();
    renderWithProviders(
      <WritingFields
        value={defaultWriting}
        onChange={onChange}
        narrators={[]}
      />,
    );
    const weekly = screen.getByRole("switch", {
      name: /^Write a weekly chapter/,
    });
    expect(weekly).toBeChecked();
    fireEvent.click(weekly);
    expect(onChange).toHaveBeenCalledWith({ ...defaultWriting, weekly: false });
  });
});
