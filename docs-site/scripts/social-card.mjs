// Draws the social card, the 1200×630 image link previews show when the docs are shared, into
// public/social-card.png: the scribe murloc, the name and what it does, in the docs' colours and
// the app's reading font. Run by hand when the art or the words change (`npm run social-card`);
// the image is committed, so builds do not need a browser for it.
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "@playwright/test";

const here = dirname(fileURLToPath(import.meta.url));
const site = resolve(here, "..");
const dataUrl = (path, type) =>
  `data:${type};base64,${readFileSync(path).toString("base64")}`;

const murloc = dataUrl(resolve(site, "src/assets/murloc/scribe.webp"), "image/webp");
const font = dataUrl(
  resolve(
    site,
    "../companion/ui/node_modules/@fontsource-variable/literata/files/literata-latin-wght-normal.woff2",
  ),
  "font/woff2",
);

const html = `<!doctype html>
<html><head><meta charset="utf-8"><style>
  @font-face { font-family: Literata; src: url(${font}) format("woff2"); font-weight: 200 900; }
  html, body { margin: 0; }
  body {
    width: 1200px; height: 630px; overflow: hidden; box-sizing: border-box;
    padding: 72px 80px; display: flex; align-items: center; gap: 40px;
    background: radial-gradient(circle at 78% 45%, #12403a 0%, #0b1f1d 55%, #071311 100%);
    color: #e9f5f2; font-family: Literata, Georgia, serif;
  }
  .words { flex: 1; }
  h1 { font-size: 112px; font-weight: 700; letter-spacing: -0.02em; margin: 0 0 20px; }
  p { font-size: 40px; line-height: 1.3; margin: 0; color: #c3fae8; max-width: 640px; }
  .url { margin-top: 40px; font-size: 28px; color: #12b886; font-family: system-ui, sans-serif; }
  img { width: 420px; height: 420px; object-fit: contain; filter: drop-shadow(0 18px 30px #0008); }
</style></head><body>
  <div class="words">
    <h1>Mrglglgl</h1>
    <p>A journal of your World of Warcraft: Forever character's life, kept by a murloc.</p>
    <div class="url">mrglglgl.4sh.dev</div>
  </div>
  <img src="${murloc}" alt="">
</body></html>`;

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1200, height: 630 } });
await page.setContent(html, { waitUntil: "load" });
await page.evaluate(() => document.fonts.ready);
const out = resolve(site, "public/social-card.png");
await page.screenshot({ path: out });
await browser.close();
console.log(`wrote ${out}`);
