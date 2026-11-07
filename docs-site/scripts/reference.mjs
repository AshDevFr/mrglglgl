// Copies the developer references from docs/ into the site as pages. They live in docs/ because
// the app's tests read docs/schema.md against the code, so the site takes a copy at build time
// instead of keeping a second one. The copies are not committed.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const site = join(dirname(fileURLToPath(import.meta.url)), "..");
const repo = join(site, "..");
const out = join(site, "src/content/docs/reference");

const PAGES = [
  {
    file: "docs/schema.md",
    slug: "journal-format",
    description: "What the addon writes and the app reads: the journal's tables and fields.",
  },
  {
    file: "docs/forever-api.md",
    slug: "client-notes",
    description: "What the addon relies on in the WoW Forever client, and how each fact was checked.",
  },
  {
    file: "docs/releasing.md",
    slug: "releasing",
    description: "How the addon, the app and these docs are released.",
  },
];

mkdirSync(out, { recursive: true });
for (const page of PAGES) {
  const text = readFileSync(join(repo, page.file), "utf8");
  // The first heading becomes the page title; Starlight prints it itself.
  const [first, ...rest] = text.split("\n");
  const title = first.replace(/^#\s+/, "").trim();
  const front = [
    "---",
    `title: ${JSON.stringify(title)}`,
    `description: ${JSON.stringify(page.description)}`,
    `editUrl: https://github.com/AshDevFr/mrglglgl/edit/main/${page.file}`,
    "---",
    "",
  ].join("\n");
  writeFileSync(join(out, `${page.slug}.md`), front + rest.join("\n").replace(/^\n+/, ""));
  console.log(`reference: ${page.file} -> reference/${page.slug}`);
}
