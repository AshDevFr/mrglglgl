import { MantineProvider } from "@mantine/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import type { Award } from "@/bindings/Award";
import type { Journal } from "@/bindings/Journal";
import type { SiteSettings } from "@/bindings/SiteSettings";
import type { WritingSettings } from "@/bindings/WritingSettings";
import awards from "./data/synthetic-awards.json";
import journal from "./data/synthetic-journal.json";

/** The shared synthetic journal, exactly as the backend serves it (written by a Rust test). */
export const syntheticJournal = journal as unknown as Journal;
export const syntheticAwards = awards as Award[];

/** The writing settings a fresh install has, as the backend's defaults. */
export const defaultWriting: WritingSettings = {
  narrator: "murloc_scribe",
  meddling: "sometimes",
  proseLanguage: null,
  provider: "local",
  modelId: null,
  endpointUrl: null,
  endpointModel: null,
  pseudonyms: false,
  writeWhilePlaying: false,
  writeOnBattery: false,
  paused: false,
  review: false,
  copyFooter: true,
  weekly: true,
  preferences: "",
};

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

/** The website settings a fresh install has, as the backend's defaults. */
export const defaultSite: SiteSettings = {
  enabled: false,
  folder: null,
  themeFolder: null,
  title: "",
  baseUrl: null,
  characters: [],
  standInNames: true,
  hideRealm: true,
};
