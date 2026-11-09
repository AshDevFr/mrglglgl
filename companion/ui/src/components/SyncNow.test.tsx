import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SyncNow } from "@/components/SyncNow";
import { renderWithProviders } from "@/test/render";

const commands = vi.hoisted(() => ({ syncNow: vi.fn() }));
vi.mock("@/api/commands", () => commands);
const shown = vi.hoisted(() => vi.fn());
vi.mock("@mantine/notifications", () => ({ notifications: { show: shown } }));

const root = { path: "/wow", flavors: [] };

beforeEach(() => vi.clearAllMocks());

describe("SyncNow", () => {
  it("says how many journals it read", async () => {
    commands.syncNow.mockResolvedValue({
      roots: [root],
      characters: 3,
      imported: 3,
      unreadable: 0,
    });
    renderWithProviders(<SyncNow />);
    fireEvent.click(
      screen.getByRole("button", { name: "Check for new journal entries" }),
    );
    await vi.waitFor(() =>
      expect(shown).toHaveBeenCalledWith(
        expect.objectContaining({ message: "Read 3 character journals." }),
      ),
    );
  });

  it("says when a journal could not be read, or no game was found", async () => {
    commands.syncNow.mockResolvedValue({
      roots: [root],
      characters: 3,
      imported: 2,
      unreadable: 1,
    });
    renderWithProviders(<SyncNow />);
    fireEvent.click(
      screen.getByRole("button", { name: "Check for new journal entries" }),
    );
    await vi.waitFor(() =>
      expect(shown).toHaveBeenCalledWith(
        expect.objectContaining({
          message:
            "Read 2 character journals. 1 could not be read: its character page says why.",
          color: "yellow",
        }),
      ),
    );
    commands.syncNow.mockResolvedValue({
      roots: [],
      characters: 0,
      imported: 0,
      unreadable: 0,
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Check for new journal entries" }),
    );
    await vi.waitFor(() =>
      expect(shown).toHaveBeenLastCalledWith(
        expect.objectContaining({
          message: expect.stringMatching(/No World of Warcraft folder/),
        }),
      ),
    );
  });
});
