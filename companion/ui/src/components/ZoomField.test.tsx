import { act, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ZoomField } from "@/components/ZoomField";
import { applyZoom } from "@/lib/zoom";
import { renderWithProviders } from "@/test/render";

vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ setZoom: () => Promise.resolve() }),
}));

describe("ZoomField", () => {
  it("shows the remembered size and follows the keyboard shortcuts", async () => {
    localStorage.clear();
    renderWithProviders(<ZoomField />);
    const field = screen
      .getAllByLabelText(/Interface size/)
      .find((el) => el.tagName === "INPUT");
    if (!field) throw new Error("no size input");
    expect(field).toHaveValue("100%");
    await act(() => applyZoom(125));
    expect(field).toHaveValue("125%");
    expect(localStorage.getItem("mrglglgl.zoom")).toBe("125");
  });
});
