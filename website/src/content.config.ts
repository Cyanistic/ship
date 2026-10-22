import { defineCollection } from "astro:content";
import { docsLoader } from "@astrojs/starlight/loaders";
import { docsSchema } from "@astrojs/starlight/schema";

// src/content/docs/ is written by scripts/prepare-docs.mjs; each page's frontmatter `slug` carries its
// version prefix (docs, docs/main, docs/v1.2.0), so one build serves one version at its own path.
export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
};
