import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithProviders } from "@/test/render";
import { LogPanel, levelsOf } from "./LogPanel";

const lines = [
  "2026-11-06T19:00:00Z DEBUG mrglglgl: looking for game folders",
  "2026-11-06T19:00:00Z  INFO mrglglgl: synced 1 character",
  "2026-11-06T19:00:01Z  WARN mrglglgl: archive: day 2026-11-01 could not be read",
  "2026-11-06T19:00:02Z ERROR mrglglgl: writing: the model stopped",
  "  caused by: the endpoint answered 500",
];
vi.mock("@/api/commands", () => ({
  recentLog: async () => lines,
  showLog: async () => {},
}));

const shown = () => screen.getByLabelText("Log");

async function chooseLevel(name: string) {
  const input = screen
    .getAllByLabelText("Level")
    .find((el) => el.tagName === "INPUT");
  if (!input) throw new Error("no level input");
  fireEvent.click(input);
  fireEvent.click(
    await screen.findByText(name, {
      selector: "[role=option] *, [role=option]",
    }),
  );
}

describe("LogPanel", () => {
  it("reads each line's level; a line without one belongs to the line above", () => {
    expect(levelsOf(lines)).toEqual([
      "debug",
      "info",
      "warn",
      "error",
      "error",
    ]);
  });

  it("shows Info and above by default, and another level when chosen", async () => {
    renderWithProviders(<LogPanel />);
    await waitFor(() =>
      expect(shown()).toHaveTextContent("synced 1 character"),
    );
    expect(shown()).not.toHaveTextContent("looking for game folders");

    await chooseLevel("Debug");
    await waitFor(() =>
      expect(shown()).toHaveTextContent("looking for game folders"),
    );

    await chooseLevel("Warning");
    await waitFor(() => expect(shown()).not.toHaveTextContent("synced"));
    expect(shown()).toHaveTextContent("could not be read");
    // The detail under an error stays with it.
    expect(shown()).toHaveTextContent("caused by: the endpoint answered 500");
  });

  it("colours each line by its level", async () => {
    renderWithProviders(<LogPanel />);
    await waitFor(() => expect(shown()).toHaveTextContent("the model stopped"));
    expect(screen.getByText(/the model stopped/)).toHaveAttribute(
      "data-level",
      "error",
    );
    expect(screen.getByText(/could not be read/)).toHaveAttribute(
      "data-level",
      "warn",
    );
    expect(screen.getByText(/synced 1 character/)).toHaveAttribute(
      "data-level",
      "info",
    );
  });

  it("copies the lines shown", async () => {
    const writeText = vi.fn(async () => {});
    Object.assign(navigator, { clipboard: { writeText } });
    renderWithProviders(<LogPanel />);
    await waitFor(() => expect(shown()).toHaveTextContent("synced"));
    fireEvent.click(screen.getByRole("button", { name: "Copy the lines" }));
    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith(lines.slice(1).join("\n")),
    );
  });
});
