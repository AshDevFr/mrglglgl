// The Mrglglgl documentation, built by .github/workflows/docs.yml (which takes its screenshots)
// and uploaded to Cloudflare Pages, which only hosts it.
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://mrglglgl.pages.dev",
  integrations: [
    starlight({
      title: "Mrglglgl",
      logo: { src: "./src/assets/murloc/head.png", alt: "" },
      favicon: "/favicon.png",
      head: [
        {
          tag: "link",
          attrs: { rel: "apple-touch-icon", href: "/apple-touch-icon.png" },
        },
      ],
      // The 404 page is src/content/docs/404.md, with the sleeping murloc.
      disable404Route: true,
      description:
        "A journal of your World of Warcraft: Forever character's life, kept by a murloc.",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/AshDevFr/mrglglgl",
        },
      ],
      editLink: {
        baseUrl: "https://github.com/AshDevFr/mrglglgl/edit/main/docs-site/",
      },
      customCss: ["./src/styles/custom.css"],
      sidebar: [
        { label: "Getting started", slug: "getting-started" },
        {
          label: "Guide",
          items: [
            { slug: "guide/reading" },
            { slug: "guide/narrators" },
            { slug: "guide/sharing" },
            { slug: "guide/website" },
            { slug: "guide/addon" },
            { slug: "guide/recorded" },
            { slug: "guide/settings" },
            { slug: "guide/your-data" },
          ],
        },
        {
          label: "Reference",
          items: [
            { slug: "reference/narrator-files" },
            // Generated from the app's milestone data (scripts/milestones.mjs).
            { slug: "reference/milestones" },
            { slug: "reference/website-themes" },
            // Copied from docs/ before each build (scripts/reference.mjs).
            { slug: "reference/journal-format" },
            { slug: "reference/client-notes" },
            { slug: "reference/releasing" },
          ],
        },
      ],
    }),
  ],
});
