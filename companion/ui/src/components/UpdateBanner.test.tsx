import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithProviders } from "@/test/render";
import { UpdateBanner } from "./UpdateBanner";
import { UpdateCheck } from "./UpdateCheck";

const install = vi.fn(async (_update: unknown) => {});
let offered: { version: string } | null = null;
vi.mock("@/lib/updates", () => ({
  availableUpdate: async () => offered,
  checkForUpdate: async () => ({ update: offered, failed: false }),
  installAndRestart: (u: unknown) => install(u),
}));
vi.mock("@/api/commands", () => ({
  appInfo: async () => ({ version: "0.1.9" }),
}));

describe("UpdateBanner", () => {
  it("offers a newer version and installs it on request", async () => {
    offered = { version: "0.2.0" };
    renderWithProviders(<UpdateBanner />);
    expect(
      await screen.findByText("Mrglglgl 0.2.0 is out"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Update and restart" }));
    await waitFor(() => expect(install).toHaveBeenCalledWith(offered));
  });

  it("says nothing when there is no update, and hides on Later", async () => {
    offered = null;
    const { unmount } = renderWithProviders(<UpdateBanner />);
    await new Promise((r) => setTimeout(r, 0));
    expect(screen.queryByRole("button", { name: "Later" })).toBeNull();
    unmount();
    offered = { version: "0.2.0" };
    renderWithProviders(<UpdateBanner />);
    fireEvent.click(await screen.findByRole("button", { name: "Later" }));
    expect(screen.queryByText("Mrglglgl 0.2.0 is out")).toBeNull();
  });

  it("offers the update again when the player checks after Later", async () => {
    offered = { version: "0.2.0" };
    renderWithProviders(
      <>
        <UpdateBanner />
        <UpdateCheck />
      </>,
    );
    fireEvent.click(await screen.findByRole("button", { name: "Later" }));
    expect(screen.queryByText("Mrglglgl 0.2.0 is out")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));
    expect(
      await screen.findByText("Mrglglgl 0.2.0 is out"),
    ).toBeInTheDocument();
  });
});
