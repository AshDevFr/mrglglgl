import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { UpdateCheck } from "@/components/UpdateCheck";
import { renderWithProviders } from "@/test/render";

const updates = vi.hoisted(() => ({
  checkForUpdate: vi.fn(),
}));
vi.mock("@/lib/updates", () => updates);
vi.mock("@/api/commands", () => ({
  appInfo: vi.fn(async () => ({ version: "0.1.6" })),
}));

beforeEach(() => vi.clearAllMocks());

describe("UpdateCheck", () => {
  it("says the app is up to date", async () => {
    updates.checkForUpdate.mockResolvedValue({ update: null, failed: false });
    renderWithProviders(<UpdateCheck onFound={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /Check for updates/ }));
    expect(
      await screen.findByText("Up to date: version 0.1.6."),
    ).toBeInTheDocument();
  });

  it("hands a newer version to the banner", async () => {
    const update = { version: "0.1.7" };
    updates.checkForUpdate.mockResolvedValue({ update, failed: false });
    const onFound = vi.fn();
    renderWithProviders(<UpdateCheck onFound={onFound} />);
    fireEvent.click(screen.getByRole("button", { name: /Check for updates/ }));
    await vi.waitFor(() => expect(onFound).toHaveBeenCalledWith(update));
  });

  it("says when it could not check", async () => {
    updates.checkForUpdate.mockResolvedValue({ update: null, failed: true });
    renderWithProviders(<UpdateCheck onFound={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /Check for updates/ }));
    expect(
      await screen.findByText(
        /Could not look for updates\. Check the connection/,
      ),
    ).toBeInTheDocument();
  });
});
