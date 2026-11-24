import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
  site: "https://oxgraft.dev",
  integrations: [
    starlight({
      title: "Oxgraft",
      description: "The best of every coding agent, grafted into one.",
      social: [{ icon: "github", label: "GitHub", href: "https://github.com/code-with-rashid/oxgraft" }],
      editLink: { baseUrl: "https://github.com/code-with-rashid/oxgraft/edit/main/" },
      sidebar: [
        { label: "Start", items: [{ label: "Overview", link: "/" }] },
        { label: "Guides", items: [{ autogenerate: { directory: "guides" } }] },
        {
          label: "Learning",
          items: [
            { label: "How Oxgraft learns", link: "/learning/" },
            { label: "Capability matrix", link: "/learning/capability-matrix/" },
            { label: "Harnesses", items: [{ autogenerate: { directory: "learning/harnesses" } }] },
            { label: "Grafts", items: [{ autogenerate: { directory: "learning/grafts" } }] },
          ],
        },
        { label: "Architecture decisions", items: [{ autogenerate: { directory: "adr" } }] },
      ],
    }),
  ],
});
