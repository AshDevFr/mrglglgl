// Takes the documentation's screenshots from the app's interface in demo mode: the real
// interface in a browser, with made-up answers in place of the desktop backend.
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
    colorScheme: "light",
    locale: "en-US",
    timezoneId: "UTC",
  },
  webServer: {
    command: `npm --prefix ../companion/ui run demo -- --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
