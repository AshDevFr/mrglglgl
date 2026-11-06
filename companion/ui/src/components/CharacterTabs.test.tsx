import { fireEvent, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CharacterTabs } from "@/components/CharacterTabs";
import { renderWithProviders } from "@/test/render";

describe("CharacterTabs", () => {
  it("shows where the reader is and goes to the other pages", () => {
    const onSelect = vi.fn();
    renderWithProviders(
      <CharacterTabs current="review" toReview={3} onSelect={onSelect} />,
    );
    const nav = screen.getByRole("navigation", { name: "Character pages" });
    const review = within(nav).getByRole("button", { name: /To review/ });
    expect(review).toHaveAttribute("aria-current", "page");
    expect(review).toHaveTextContent("3");
    expect(
      within(nav).getByRole("button", { name: "Journal" }),
    ).not.toHaveAttribute("aria-current");
    fireEvent.click(within(nav).getByRole("button", { name: "Milestones" }));
    expect(onSelect).toHaveBeenCalledWith("milestones");
  });

  it("shows no count when nothing waits", () => {
    renderWithProviders(
      <CharacterTabs current="journal" toReview={0} onSelect={vi.fn()} />,
    );
    expect(
      screen.getByRole("button", { name: "To review" }),
    ).toBeInTheDocument();
  });
});
