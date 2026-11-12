import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { FactList } from "@/components/FactList";
import type { FactRow } from "@/lib/factList";
import { renderWithProviders } from "@/test/render";

const row = (key: string, n: number): FactRow => ({
  key,
  label: key,
  items: Array.from({ length: n }, (_, i) => ({ text: `${key} ${i + 1}` })),
});

describe("FactList", () => {
  it("folds a long row to its first items and opens it on demand", () => {
    renderWithProviders(
      <FactList rows={[row("Places", 31)]} empty="nothing" />,
    );
    expect(screen.getByText("Places 5")).toBeInTheDocument();
    expect(screen.queryByText("Places 6")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "26 more" }));
    expect(screen.getByText("Places 31")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Show fewer" }));
    expect(screen.queryByText("Places 31")).toBeNull();
  });

  it("shows a short row whole, with nothing to open", () => {
    renderWithProviders(<FactList rows={[row("Zones", 6)]} empty="nothing" />);
    expect(screen.getByText("Zones 6")).toBeInTheDocument();
    expect(screen.queryByRole("button")).toBeNull();
  });
});
