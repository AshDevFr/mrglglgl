import { MantineProvider } from "@mantine/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import type { Award } from "@/bindings/Award";
import type { Journal } from "@/bindings/Journal";
import awards from "./data/synthetic-awards.json";
import journal from "./data/synthetic-journal.json";

/** The shared synthetic journal, exactly as the backend serves it (written by a Rust test). */
export const syntheticJournal = journal as unknown as Journal;
export const syntheticAwards = awards as Award[];

export function renderWithProviders(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <MantineProvider>
      <QueryClientProvider client={client}>{ui}</QueryClientProvider>
    </MantineProvider>,
  );
}
