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

  it("asks the model again once when the fact check flags an entry, unless told otherwise", async () => {
    const onChange = vi.fn();
    renderWithProviders(
      <WritingFields
        value={defaultWriting}
        onChange={onChange}
        narrators={[]}
      />,
    );
    const select = screen
      .getAllByLabelText(/^When the fact check flags an entry/)
      .find((el) => el.tagName === "INPUT");
    if (!select) throw new Error("no fact check input");
    expect(select).toHaveValue("Ask the model again once, then show it to me");
    fireEvent.click(select);
    fireEvent.click(
      await screen.findByRole("option", {
        name: "Ask the model again up to 3 times, then show it to me",
        hidden: true,
      }),
    );
    expect(onChange).toHaveBeenCalledWith({
      ...defaultWriting,
      checkRetries: 3,
    });
  });
});
