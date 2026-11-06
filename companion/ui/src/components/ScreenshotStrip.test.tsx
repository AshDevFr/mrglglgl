import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithProviders } from "@/test/render";
import { ScreenshotStrip } from "./ScreenshotStrip";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) =>
    `asset://localhost/${encodeURIComponent(path)}`,
  invoke: vi.fn(),
}));

describe("ScreenshotStrip", () => {
  it("shows thumbnails that open full size, served through the asset protocol", async () => {
    renderWithProviders(
      <ScreenshotStrip
        screenshots={[
          {
            path: "/wow/_forever_/Screenshots/WoWScrnShot_110526_202955.jpg",
            taken: 1793939395,
            moment: "levelUp",
          },
        ]}
      />,
    );
    const thumb = screen.getByAltText("Screenshot: level up");
    expect(thumb.getAttribute("src")).toMatch(/^asset:\/\/localhost\//);
    fireEvent.click(thumb);
    // The full-size image opens in a modal beside the thumbnail.
    await waitFor(() =>
      expect(screen.getAllByAltText("Screenshot: level up")).toHaveLength(2),
    );
  });

  it("describes an unknown moment plainly", () => {
    renderWithProviders(
      <ScreenshotStrip
        screenshots={[{ path: "/x.jpg", taken: 0, moment: null }]}
      />,
    );
    expect(
      screen.getByAltText("Screenshot: a moment of the day"),
    ).toBeInTheDocument();
  });
});
