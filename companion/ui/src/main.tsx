import { MantineProvider } from "@mantine/core";
import "@mantine/core/styles.css";
import { Notifications } from "@mantine/notifications";
import "@mantine/notifications/styles.css";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { initI18n } from "./i18n";
import { router } from "./router";

const queryClient = new QueryClient();

initI18n().then(() => {
  const rootEl = document.getElementById("root");
  if (!rootEl) throw new Error("#root element not found");
  createRoot(rootEl).render(
    <StrictMode>
      <MantineProvider defaultColorScheme="auto">
        <Notifications />
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
        </QueryClientProvider>
      </MantineProvider>
    </StrictMode>,
  );
});
