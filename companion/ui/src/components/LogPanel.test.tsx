import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LogPanel } from "@/components/LogPanel";
import { renderWithProviders } from "@/test/render";

const api = vi.hoisted(() => ({ showLog: vi.fn(async () => {}) }));
vi.mock("@/api/commands", () => api);

describe("LogPanel", () => {
  it("shows the log file, and says what to do when it cannot", async () => {
    renderWithProviders(<LogPanel />);
    fireEvent.click(screen.getByText("Show the log file"));
    await waitFor(() => expect(api.showLog).toHaveBeenCalled());
    api.showLog.mockRejectedValueOnce("no file manager");
    fireEvent.click(screen.getByText("Show the log file"));
    expect(
      await screen.findByText(/Could not show the log: no file manager\. /),
    ).toBeInTheDocument();
  });
});
