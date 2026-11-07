// Takes the documentation's screenshots from the app's interface in demo mode: the real
// interface in a browser, with made-up answers in place of the desktop backend.
import { fileURLToPath } from "node:url";
import { defineConfig } from "@playwright/test";

const PORT = 5179;

export default defineConfig({
  testDir: ".",
  outputDir: "results",
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  use: {
    baseURL: `http://localhost:${PORT}`,
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 2,
    // The app in dark mode: its paper reads well against the docs in either theme.
    colorScheme: "dark",
    locale: "en-US",
    timezoneId: "UTC",
  },
  webServer: {
    // Run from the app's interface folder: the command's working folder is otherwise this one.
    command: `npm run demo -- --port ${PORT} --strictPort`,
    cwd: fileURLToPath(new URL("../../companion/ui", import.meta.url)),
    url: `http://localhost:${PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
