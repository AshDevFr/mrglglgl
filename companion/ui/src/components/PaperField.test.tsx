import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PaperField } from "@/components/PaperField";
import { renderWithProviders } from "@/test/render";

describe("PaperField", () => {
  it("lays the app on the chosen paper and remembers it", async () => {
    localStorage.clear();
    renderWithProviders(<PaperField />);
    const field = screen
      .getAllByLabelText(/^Paper/)
      .find((el) => el.tagName === "INPUT");
    if (!field) throw new Error("no paper input");
    expect(field).toHaveValue("Tide-marked rag paper");
    fireEvent.click(field);
    fireEvent.click(
      await screen.findByRole("option", { name: "Parchment", hidden: true }),
    );
    expect(document.documentElement.dataset.paper).toBe("parchment");
    expect(localStorage.getItem("mrglglgl.paper")).toBe("parchment");
  });
});
