import { createTheme, MantineProvider } from "@mantine/core";
import "@mantine/core/styles.css";
import { Notifications } from "@mantine/notifications";
import "@mantine/notifications/styles.css";
import "@fontsource-variable/literata";
import "./styles/paper.css";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getSettings } from "./api/commands";
import { applyLanguage, initI18n } from "./i18n";
import { applyZoom, savedZoom } from "./lib/zoom";
import { router } from "./router";

// Queries are local commands, not network calls: retrying one that failed only delays the error.
const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: false } },
});

// Reading text and headings use Literata (bundled, open licence); the app's
// controls keep the system font.
const theme = createTheme({
  headings: { fontFamily: "var(--reading-font)" },
  primaryColor: "teal",
});

void applyZoom(savedZoom());

initI18n()
  .then(() => getSettings())
  .then((view) => applyLanguage(view.settings.uiLanguage))
  .catch(() => {
    // The interface still works in the system language if settings cannot be read.
  })
  .finally(() => {
    const rootEl = document.getElementById("root");
    if (!rootEl) throw new Error("#root element not found");
    createRoot(rootEl).render(
      <StrictMode>
        <MantineProvider theme={theme} defaultColorScheme="auto">
          <Notifications />
          <QueryClientProvider client={queryClient}>
            <RouterProvider router={router} />
          </QueryClientProvider>
        </MantineProvider>
      </StrictMode>,
    );
  });
