import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Story } from "@/bindings/Story";
import { ExportDocument } from "@/components/ExportDocument";
import { exportPage } from "@/lib/exportPage";
import { syntheticAwards, syntheticJournal } from "@/test/render";

function story(
  id: number,
  kind: Story["kind"],
  day: string,
  text: string,
  extra: Partial<Story> = {},
): Story {
  return {
    id,
    characterId: 1,
    kind,
    startDay: day,
    endDay: day,
    narrator: "murloc_scribe",
    language: "en",
    provider: "local",
    model: "m",
    promptHash: "h",
    text,
    createdAt: 0,
    check: "passed",
    checkReason: null,
    status: "accepted",
    note: null,
    partial: false,
    ...extra,
  };
}

const data = {
  title: "Wren Hollowbrook",
  journal: syntheticJournal,
  awards: syntheticAwards,
  stories: [
    story(1, "day", "2026-11-08", "An older version."),
    story(2, "day", "2026-11-08", "The wolves of Mulgore kept me busy."),
    story(3, "day", "2026-11-09", "I sailed to Booty Bay.", {
      check: "failed",
    }),
    story(4, "week", "2026-11-02", "A week of wolves and water."),
  ],
  photos: { "2026-11-08": ["data:image/jpeg;base64,AAAA"] },
  paper: "parchment" as const,
};

describe("ExportDocument", () => {
  it("lays the journal out as the app does: pages, stamps, photos, chapters", () => {
    render(<ExportDocument {...data} />);
    expect(
      screen.getByRole("heading", { level: 1, name: "Wren Hollowbrook" }),
    ).toBeInTheDocument();
    const day = screen.getByRole("article", {
      name: "Sunday, November 8, 2026",
    });
    expect(day).toHaveTextContent("The wolves of Mulgore kept me busy.");
    expect(day).not.toHaveTextContent("An older version.");
    // Milestones are pressed on as ink stamps, as in the app.
    expect(within(day).getAllByRole("img").length).toBeGreaterThanOrEqual(2);
    expect(day.querySelector("svg.ink-stamp")).not.toBeNull();
    expect(day.querySelector("img")?.getAttribute("src")).toBe(
      "data:image/jpeg;base64,AAAA",
    );
    // An entry set aside by the fact check is not exported; the day reads as on its page.
    const setAside = screen.getByRole("article", {
      name: "Monday, November 9, 2026",
    });
    expect(setAside).not.toHaveTextContent("Booty Bay");
    expect(setAside.textContent?.length).toBeGreaterThan(10);
    expect(screen.getByText("A week of wolves and water.")).toBeInTheDocument();
  });

  it("tells the days in order, oldest first, like a book", () => {
    render(<ExportDocument {...data} />);
    const days = screen
      .getAllByRole("article")
      .map((a) => a.getAttribute("aria-label"));
    expect(days[0]).toBe("Sunday, November 1, 2026");
    expect(days[days.length - 1]).toBe("Tuesday, November 10, 2026");
  });

  it("becomes a page body and its own look, with nothing of the app around it", () => {
    const page = exportPage(data);
    expect(page.body).toContain('class="export paper-light"');
    expect(page.body).toContain('data-paper="parchment"');
    expect(page.body).toContain("ink-stamp");
    expect(page.body).not.toContain("mantine");
    expect(page.css).toContain(".journal-page");
    expect(page.css).toContain("@media print");
  });
});
