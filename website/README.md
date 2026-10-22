# Ship website

This folder builds Ship's landing page and docs site, using Astro and Starlight. Pushes to main and each release deploy it to GitHub Pages, under `/ship/` (see [`.github/workflows/website.yml`](../.github/workflows/website.yml)).

The docs themselves don't live here. They're plain markdown in [`docs/site/`](../docs/site/), so they read fine on GitHub too, and this folder turns them into the site. To fix or add a page, edit it there. A new page also needs a link in the overview's lists, [`docs/site/README.md`](../docs/site/README.md), which is how the sidebar knows about it.

## Running it

You'll need Node and pnpm.

```sh
pnpm install
pnpm dev      # a local copy that reloads as you edit
pnpm build    # the static site, in dist/
pnpm format   # Prettier; pull requests fail if this would change anything
```

## Versions

Each release gets its own copy of the docs, built from its git tag. `/docs/` shows the latest release, `/docs/v0.1.0/` and friends show older ones, and `/docs/main/` shows what's on `main` with an "unreleased" banner. `pnpm build:versions` builds all of them at once.
