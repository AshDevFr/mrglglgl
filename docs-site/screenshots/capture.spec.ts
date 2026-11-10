// Each test saves one screenshot used by the documentation, under src/assets/screenshots/.
import { fileURLToPath } from "node:url";
import { expect, type Page, test } from "@playwright/test";

const out = (name: string) =>
  fileURLToPath(new URL(`../src/assets/screenshots/${name}.png`, import.meta.url));

// The sample journal's days are early November 2026; on the 11th they are all finished.
test.beforeEach(async ({ page }) => {
  await page.clock.setFixedTime(new Date("2026-11-11T12:00:00Z"));
});

/** An element with room around it: the day's stamps hang past the page's edge. */
async function around(page: Page, selector: ReturnType<Page["locator"]>, name: string) {
  // The app scrolls inside its own panel, so the card is brought into a tall viewport first.
  await page.setViewportSize({ width: 1280, height: 1600 });
  await selector.scrollIntoViewIfNeeded();
  await page.waitForTimeout(200);
  const box = await selector.boundingBox();
  if (!box) throw new Error(`${name}: nothing to capture`);
  const pad = 32;
  await page.screenshot({
    path: out(name),
    clip: {
      x: Math.max(0, box.x - pad),
      y: Math.max(0, box.y - pad),
      width: box.width + pad * 2,
      height: box.height + pad * 2,
    },
  });
}

async function settled(page: Page) {
  await page.waitForLoadState("networkidle");
  // Fonts and the paper texture, so every capture looks the same.
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(300);
}

test("home", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText(/Addon: Version/)).toBeVisible();
  await settled(page);
  await page.screenshot({ path: out("home") });
});

test("first run", async ({ page }) => {
  await page.goto("/?empty");
  await expect(page.getByText(/No journals yet/)).toBeVisible();
  await settled(page);
  await page.screenshot({ path: out("first-run") });
});

test("journal", async ({ page }) => {
  await page.goto("/characters/1?day=2026-11-08");
  await expect(page.getByText("Hogger fell")).toBeVisible();
  await settled(page);
  await page.screenshot({ path: out("journal") });
});

test("entry", async ({ page }) => {
  await page.goto("/characters/1?day=2026-11-08");
  const day = page.locator(".journal-page").last();
  await expect(day.getByText("Hogger fell")).toBeVisible();
  await settled(page);
  await around(page, day, "entry");
});

test("edit", async ({ page }) => {
  await page.goto("/characters/1?day=2026-11-08");
  const day = page.locator(".journal-page").last();
  await day.getByRole("button", { name: "Edit" }).click();
  await expect(day.getByLabel("Your version of the entry")).toBeVisible();
  await settled(page);
  await around(page, day, "edit");
});

test("milestones", async ({ page }) => {
  await page.goto("/characters/1/milestones");
  await expect(page.getByRole("heading").first()).toBeVisible();
  await settled(page);
  await page.screenshot({ path: out("milestones") });
});

for (const tab of ["General", "AI", "Writing", "Characters", "Narrators", "Website", "Addon"]) {
  test(`settings ${tab}`, async ({ page }) => {
    await page.goto("/settings");
    await page.getByRole("tab", { name: tab }).click();
    await settled(page);
    await page.screenshot({ path: out(`settings-${tab.toLowerCase()}`) });
  });
}

test("narrator editor", async ({ page }) => {
  await page.goto("/narrators/custom_heron");
  await expect(page.locator("input[value=\"The patient heron\"]")).toBeVisible();
  await settled(page);
  await page.screenshot({ path: out("narrator-editor") });
});

test("nothing to review", async ({ page }) => {
  await page.goto("/characters/1/review");
  await expect(page.getByText(/Nothing to review/)).toBeVisible();
  await settled(page);
  await page.screenshot({ path: out("review-empty") });
});

test("a day with nothing written", async ({ page }) => {
  await page.goto("/characters/1?day=2026-11-04");
  const empty = page.getByText(/Nothing written on this page/);
  await expect(empty).toBeVisible();
  await settled(page);
  // The murloc and its message, the whole empty state.
  await around(page, empty.locator("xpath=ancestor::div[2]"), "empty-day");
});
