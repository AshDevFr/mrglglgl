import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { shiftWeek, weekDays, weekStart } from "@/lib/weeks";
import { renderWithProviders } from "@/test/render";
import { WeekNav } from "./WeekNav";

// The shared fixture's days: a compacted Sunday, then three written days the next week.
const written = new Set([
  "2026-11-01",
  "2026-11-08",
  "2026-11-09",
  "2026-11-10",
]);

describe("weeks", () => {
  it("runs Monday to Sunday", () => {
    expect(weekStart("2026-11-08")).toBe("2026-11-02"); // a Sunday
    expect(weekStart("2026-11-09")).toBe("2026-11-09"); // a Monday
    expect(weekDays("2026-11-05")).toEqual([
      "2026-11-02",
      "2026-11-03",
      "2026-11-04",
      "2026-11-05",
      "2026-11-06",
      "2026-11-07",
      "2026-11-08",
    ]);
  });

  it("moves to the latest written day of a week, or its Monday when nothing was written", () => {
    expect(shiftWeek("2026-11-08", -1, written)).toBe("2026-11-01");
    expect(shiftWeek("2026-11-08", 1, written)).toBe("2026-11-10");
    expect(shiftWeek("2026-11-10", 1, written)).toBe("2026-11-16");
    expect(shiftWeek("2026-11-02", -1, written)).toBe("2026-11-01");
  });
});

function renderNav(selected: string, today = "2026-11-12") {
  const onSelect = vi.fn();
  renderWithProviders(
    <WeekNav
      selected={selected}
      written={written}
      today={today}
      onSelect={onSelect}
    />,
  );
  return onSelect;
}

describe("WeekNav", () => {
  it("shows the week's range and its seven days, inking the written ones", () => {
    renderNav("2026-11-08");
    expect(
      screen.getByText(/November 2 to November 8, 2026/),
    ).toBeInTheDocument();
    const days = screen
      .getAllByRole("button", { pressed: false })
      .concat(screen.getAllByRole("button", { pressed: true }));
    expect(days.filter((b) => b.classList.contains("week-day"))).toHaveLength(
      7,
    );
    const sunday = screen.getByRole("button", {
      name: /Sunday, November 8, 2026$/,
    });
    expect(sunday).toHaveAttribute("aria-pressed", "true");
    expect(sunday).toHaveClass("written");
    const tuesday = screen.getByRole("button", {
      name: /Tuesday, November 3, 2026, No entry/,
    });
    expect(tuesday).toBeDisabled();
  });

  it("goes to the previous and next weeks", () => {
    const onSelect = renderNav("2026-11-08");
    fireEvent.click(screen.getByRole("button", { name: "Previous week" }));
    expect(onSelect).toHaveBeenLastCalledWith("2026-11-01");
    fireEvent.click(screen.getByRole("button", { name: "Next week" }));
    expect(onSelect).toHaveBeenLastCalledWith("2026-11-10");
  });

  it("goes no further than the week holding today", () => {
    renderNav("2026-11-12", "2026-11-12");
    expect(screen.getByRole("button", { name: "Next week" })).toBeDisabled();
    // A week back, the next one is there to return to.
    renderNav("2026-11-05", "2026-11-12");
    expect(
      screen.getAllByRole("button", { name: "Next week" }).at(-1),
    ).not.toBeDisabled();
  });

  it("goes no further back than the first written week, and not to today from today", () => {
    // The fixture's first day is Sunday November 1.
    renderNav("2026-11-01", "2026-11-12");
    expect(
      screen.getByRole("button", { name: "Previous week" }),
    ).toBeDisabled();
    renderNav("2026-11-12", "2026-11-12");
    expect(
      screen.getAllByRole("button", { name: "Today" }).at(-1),
    ).toBeDisabled();
  });

  it("jumps to today and marks it", () => {
    const onSelect = renderNav("2026-11-10", "2026-11-12");
    const today = screen.getByRole("button", {
      name: /Thursday, November 12, 2026/,
    });
    expect(today).toHaveAttribute("aria-current", "date");
    expect(today).not.toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Today" }));
    expect(onSelect).toHaveBeenLastCalledWith("2026-11-12");
  });

  it("selects a written day", () => {
    const onSelect = renderNav("2026-11-10");
    fireEvent.click(
      screen.getByRole("button", { name: /Monday, November 9, 2026$/ }),
    );
    expect(onSelect).toHaveBeenCalledWith("2026-11-09");
  });
});
