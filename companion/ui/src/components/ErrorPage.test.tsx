import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ErrorPage } from "@/components/ErrorPage";
import { renderWithProviders } from "@/test/render";

const router = vi.hoisted(() => ({
  navigate: vi.fn(async () => {}),
  invalidate: vi.fn(async () => {}),
}));
vi.mock("@tanstack/react-router", () => ({ useRouter: () => router }));

const boom = Object.assign(new Error("undefined is not an object"), {
  stack:
    "TypeError: undefined is not an object\n    at ReviewList (ReviewList.tsx:124)",
});

beforeEach(() => vi.clearAllMocks());

describe("ErrorPage", () => {
  it("says what broke and what to do, in the player's words", () => {
    renderWithProviders(<ErrorPage error={boom} reset={() => {}} />);
    expect(
      screen.getByText(
        /This page broke: undefined is not an object\. Press Try again, or go back to the journals; nothing was lost/,
      ),
    ).toBeInTheDocument();
  });

  it("tries the page again without a restart", async () => {
    const reset = vi.fn();
    renderWithProviders(<ErrorPage error={boom} reset={reset} />);
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => expect(reset).toHaveBeenCalled());
    expect(router.invalidate).toHaveBeenCalled();
  });

  it("goes back to the home page", async () => {
    const reset = vi.fn();
    renderWithProviders(<ErrorPage error={boom} reset={reset} />);
    fireEvent.click(
      screen.getByRole("button", { name: "Back to the journals" }),
    );
    await waitFor(() =>
      expect(router.navigate).toHaveBeenCalledWith({ to: "/" }),
    );
    expect(reset).toHaveBeenCalled();
  });

  it("keeps the details for a report", () => {
    renderWithProviders(<ErrorPage error={boom} reset={() => {}} />);
    expect(screen.getByText(/ReviewList\.tsx:124/)).toBeInTheDocument();
  });
});
