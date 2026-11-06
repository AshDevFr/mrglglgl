import { MantineProvider } from "@mantine/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { HomePage } from "./HomePage";

vi.mock("@/api/commands", () => ({
  appInfo: () => Promise.resolve({ version: "9.9.9" }),
}));

function renderWithProviders(ui: ReactNode) {
  const client = new QueryClient();
  return render(
    <MantineProvider>
      <QueryClientProvider client={client}>{ui}</QueryClientProvider>
    </MantineProvider>,
  );
}

describe("HomePage", () => {
  it("shows translated text and the version reported by the Rust side", async () => {
    renderWithProviders(<HomePage />);
    expect(screen.getByText("Mrglglgl")).toBeInTheDocument();
    expect(await screen.findByText("Version 9.9.9")).toBeInTheDocument();
  });
});
