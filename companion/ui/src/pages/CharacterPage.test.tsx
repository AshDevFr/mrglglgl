import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  renderWithProviders,
  syntheticAwards,
  syntheticJournal,
} from "@/test/render";
import { DayView } from "./CharacterPage";

describe("DayView", () => {
  it("shows a raw day as readable facts", () => {
    renderWithProviders(
      <DayView
        journal={syntheticJournal}
        day="2026-11-08"
        awards={syntheticAwards}
      />,
    );
    expect(screen.getByText(/Defeated: Prairie Wolf ×23/)).toBeInTheDocument();
    expect(screen.getByText(/1 of them a murloc/)).toBeInTheDocument();
    expect(screen.getByText("Died to Murloc Streamrunner")).toBeInTheDocument();
    expect(
      screen.getByText("Quests turned in: The Ravaged Caravan"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Notable foes: Flatland Cougar (elite)"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Grouped with Tamsin Reedwater"),
    ).toBeInTheDocument();
  });

  it("shows a compacted day from its digest, with the stamps earned that day", () => {
    renderWithProviders(
      <DayView
        journal={syntheticJournal}
        day="2026-11-01"
        awards={syntheticAwards}
      />,
    );
    expect(screen.getByText("Drowned")).toBeInTheDocument();
    expect(
      screen.getByText("Caught 2 fish, including 1 boot"),
    ).toBeInTheDocument();
    expect(screen.getByText(/Summary only/)).toBeInTheDocument();
    expect(screen.getByText("Kept the Boot")).toBeInTheDocument();
    expect(screen.getByText("Sleeps With the Fishes")).toBeInTheDocument();
  });

  it("says plainly when nothing happened", () => {
    renderWithProviders(
      <DayView journal={syntheticJournal} day="2026-11-10" awards={[]} />,
    );
    expect(screen.getByText("Nothing to report.")).toBeInTheDocument();
  });
});
