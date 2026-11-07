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
  photos: {
    "2026-11-08": [
      {
        src: "data:image/jpeg;base64,AAAA",
        moment: "levelUp",
        taken: 1794164491,
      },
    ],
  },
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
    expect(days[days.length - 1]).toBe("Wednesday, November 11, 2026");
  });

  it("becomes a page body and its own look, with nothing of the app around it", () => {
    const page = exportPage(data);
    expect(page.body).toContain('class="export"');
    expect(page.body).toContain("ink-stamp");
    expect(page.body).not.toContain("mantine-Button");
    expect(page.css).toContain(".journal-page");
    expect(page.css).toContain("@media print");
  });

  it("lets the reader switch light and dark, and the paper, starting from the exported one", () => {
    const page = exportPage(data);
    // Follows the reader's system theme unless they choose; the paper starts as exported.
    expect(page.body).toContain("prefers-color-scheme: dark");
    expect(page.body).toContain('"parchment"');
    expect(page.body).toContain('data-toggle="scheme"');
    expect(page.body).toContain('data-toggle="paper"');
    // Printing switches to light paper, and the controls never print.
    expect(page.body).toContain("beforeprint");
    expect(page.body).toContain('class="export-controls no-print"');
    // Everything is inside the file.
    expect(page.body).not.toMatch(/<script[^>]+src=/);
  });

  it("runs: starts as the reader's system and the exported paper, and switches on a click", () => {
    localStorage.clear();
    const page = exportPage(data);
    document.body.innerHTML = page.body;
    const script = page.body.match(/<script>([\s\S]*?)<\/script>/)?.[1] ?? "";
    new Function(script)();
    const root = document.documentElement;
    expect(root.getAttribute("data-mantine-color-scheme")).toBe("light");
    expect(root.getAttribute("data-paper")).toBe("parchment");
    (document.querySelector('[data-toggle="scheme"]') as HTMLElement).click();
    expect(root.getAttribute("data-mantine-color-scheme")).toBe("dark");
    (document.querySelector('[data-toggle="paper"]') as HTMLElement).click();
    expect(root.getAttribute("data-paper")).toBe("tide");
    expect(localStorage.getItem("mrglglgl.export.scheme")).toBe("dark");
    window.dispatchEvent(new Event("beforeprint"));
    expect(root.getAttribute("data-mantine-color-scheme")).toBe("light");
    window.dispatchEvent(new Event("afterprint"));
    expect(root.getAttribute("data-mantine-color-scheme")).toBe("dark");
    document.body.innerHTML = "";
    root.removeAttribute("data-mantine-color-scheme");
    root.removeAttribute("data-paper");
  });

  it("captions each screenshot with what was happening, where and when", () => {
    render(<ExportDocument {...data} />);
    const day = screen.getByRole("article", {
      name: "Sunday, November 8, 2026",
    });
    const figure = day.querySelector("figure.photo");
    expect(figure?.querySelector("figcaption")?.textContent).toMatch(
      /^Level up in Red Rocks, \d{1,2}:\d{2}/,
    );
    expect(figure?.querySelector("img")?.getAttribute("alt")).toMatch(
      /^Level up in Red Rocks/,
    );
  });
});
