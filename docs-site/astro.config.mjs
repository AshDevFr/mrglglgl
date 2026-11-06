// The Mrglglgl documentation, published to GitHub Pages by .github/workflows/docs.yml.
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://ashdevfr.github.io",
  base: "/mrglglgl",
  integrations: [
    starlight({
      title: "Mrglglgl",
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
            { slug: "guide/settings" },
            { slug: "guide/your-data" },
          ],
        },
        {
          label: "Reference",
          items: [
            { slug: "reference/narrator-files" },
            { slug: "reference/website-themes" },
            {
              label: "Journal format",
              link: "https://github.com/AshDevFr/mrglglgl/blob/main/docs/schema.md",
            },
            {
              label: "Releasing",
              link: "https://github.com/AshDevFr/mrglglgl/blob/main/docs/releasing.md",
            },
          ],
        },
      ],
    }),
  ],
});
