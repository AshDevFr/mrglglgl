import { fireEvent, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Award } from "@/bindings/Award";
import { renderWithProviders } from "@/test/render";
import { InkStamp } from "./InkStamp";

const boot: Award = { id: "kept_the_boot", day: "2026-11-01", rarity: 1 };

describe("InkStamp", () => {
  it("shows the whole stamp, its name and what it is for on hover", async () => {
    renderWithProviders(<InkStamp award={boot} />);
    const stamp = screen.getByRole("img", { name: /^Kept the Boot:/ });
    fireEvent.mouseEnter(stamp.parentElement as HTMLElement);
    const tip = await screen.findByRole("tooltip");
    expect(within(tip).getByText("Kept the Boot")).toBeInTheDocument();
    expect(
      within(tip).getByText("Fished up a boot. Kept it."),
    ).toBeInTheDocument();
    // The stamp itself, large and straight, to read its rim.
    expect(tip.querySelector('svg[width="160"]')).not.toBeNull();
  });

  it("can be pressed without a tooltip, for a page that is not hovered", () => {
    renderWithProviders(<InkStamp award={boot} zoom={false} />);
    const stamp = screen.getByRole("img", { name: /^Kept the Boot:/ });
    fireEvent.mouseEnter(stamp);
    fireEvent.mouseEnter(stamp.parentElement as HTMLElement);
    expect(screen.queryByRole("tooltip")).toBeNull();
  });
});
