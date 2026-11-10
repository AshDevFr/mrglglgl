import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithProviders } from "@/test/render";
import { CHECK_FOR_UPDATES, SidebarUpdateCheck } from "./SidebarUpdateCheck";

const commands = vi.hoisted(() => ({
  appInfo: vi.fn(async () => ({ version: "0.1.12" })),
}));
vi.mock("@/api/commands", () => commands);
const updates = vi.hoisted(() => ({ checkForUpdate: vi.fn() }));
vi.mock("@/lib/updates", () => updates);
const shown = vi.hoisted(() => vi.fn());
vi.mock("@mantine/notifications", () => ({ notifications: { show: shown } }));
const menu = vi.hoisted(() => ({ handlers: new Map<string, () => void>() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (event: string, handler: () => void) => {
    menu.handlers.set(event, handler);
    return () => menu.handlers.delete(event);
  }),
}));

beforeEach(() => vi.clearAllMocks());

describe("SidebarUpdateCheck", () => {
  it("says the app is up to date", async () => {
    updates.checkForUpdate.mockResolvedValue({ update: null, failed: false });
    renderWithProviders(<SidebarUpdateCheck />);
    fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));
    await vi.waitFor(() =>
      expect(shown).toHaveBeenCalledWith({
        message: "Up to date: version 0.1.12.",
      }),
    );
  });

  it("says plainly when it could not look", async () => {
    updates.checkForUpdate.mockResolvedValue({ update: null, failed: true });
    renderWithProviders(<SidebarUpdateCheck />);
    fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));
    await vi.waitFor(() =>
      expect(shown).toHaveBeenCalledWith(
        expect.objectContaining({
          color: "red",
          message: expect.stringContaining("Could not look for updates"),
        }),
      ),
    );
  });

  it("checks when the macOS app menu asks", async () => {
    updates.checkForUpdate.mockResolvedValue({ update: null, failed: false });
    renderWithProviders(<SidebarUpdateCheck />);
    await vi.waitFor(() =>
      expect(menu.handlers.has(CHECK_FOR_UPDATES)).toBe(true),
    );
    menu.handlers.get(CHECK_FOR_UPDATES)?.();
    await vi.waitFor(() => expect(updates.checkForUpdate).toHaveBeenCalled());
  });
});
