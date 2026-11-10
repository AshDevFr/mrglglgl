import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { defaultWriting, renderWithProviders } from "@/test/render";
import { RunFields, WritingFields } from "./WritingFields";

const commands = vi.hoisted(() => ({
  setWritingPaused: vi.fn(async () => {}),
}));
vi.mock("@/api/commands", () => commands);

describe("WritingFields", () => {
  it("pauses background writing at once, on the AI tab's run switches", async () => {
    const onChange = vi.fn();
    renderWithProviders(
      <RunFields value={defaultWriting} onChange={onChange} />,
    );
    fireEvent.click(
      screen.getByRole("switch", { name: /^Pause background writing/ }),
    );
    await waitFor(() =>
      expect(commands.setWritingPaused).toHaveBeenCalledWith(true),
    );
    // The setting is kept too, so the app starts paused next time.
    expect(onChange).toHaveBeenCalledWith({ ...defaultWriting, paused: true });
  });

  it("keeps the style choices on Writing, and when the model runs off it", () => {
    renderWithProviders(
      <WritingFields
        value={defaultWriting}
        onChange={vi.fn()}
        narrators={[]}
      />,
    );
    expect(screen.getByLabelText(/Writing preferences/)).toBeInTheDocument();
    expect(screen.queryByText(/Write while the game is running/)).toBeNull();
    expect(screen.queryByText(/Pause background writing/)).toBeNull();
  });

  it("turns weekly chapters on, off by default and kept for Save like the other choices", () => {
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
    expect(weekly).not.toBeChecked();
    fireEvent.click(weekly);
    expect(onChange).toHaveBeenCalledWith({ ...defaultWriting, weekly: true });
  });
});
