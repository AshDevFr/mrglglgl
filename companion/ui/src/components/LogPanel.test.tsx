import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithProviders } from "@/test/render";
import { isProblem, LogPanel } from "./LogPanel";

const lines = [
  "2026-11-06T19:00:00Z  INFO mrglglgl: synced 1 character",
  "2026-11-06T19:00:01Z  WARN mrglglgl: archive: day 2026-11-01 could not be read",
  "2026-11-06T19:00:02Z ERROR mrglglgl: writing: the model stopped",
];
vi.mock("@/api/commands", () => ({
  recentLog: async () => lines,
  showLog: async () => {},
}));

describe("LogPanel", () => {
  it("shows the log's last lines, and only the problems when asked", async () => {
    renderWithProviders(<LogPanel />);
    const box = await screen.findByLabelText("Log");
    expect(box).toHaveTextContent("synced 1 character");
    fireEvent.click(screen.getByLabelText("Only warnings and errors"));
    await waitFor(() =>
      expect(screen.getByLabelText("Log")).not.toHaveTextContent("synced"),
    );
    expect(screen.getByLabelText("Log")).toHaveTextContent("the model stopped");
  });

  it("copies the lines shown", async () => {
    const writeText = vi.fn(async () => {});
    Object.assign(navigator, { clipboard: { writeText } });
    renderWithProviders(<LogPanel />);
    await screen.findByLabelText("Log");
    fireEvent.click(screen.getByRole("button", { name: "Copy the lines" }));
    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith(lines.join("\n")),
    );
    expect(isProblem(lines[0])).toBe(false);
  });
});
