// Where the site is served, read from the environment. astro.config.mjs and scripts/prepare-docs.mjs both
// call this, so Astro's base and the absolute URLs written into llms.txt and the raw markdown cannot disagree.
//
//   SHIP_BASE      the path the site lives under: '/' (default, local dev and a custom domain) or like '/ship/'
//   SHIP_SITE_URL  the public URL of the site root, base path included: https://shipmux.github.io/ship
//                  (default https://shipmux.dev, a placeholder until the domain is settled)
//
// Returns base with both slashes ('/ship/') and site without a trailing slash ('https://shipmux.github.io/ship').
export function siteEnv(env = process.env) {
  let base = (env.SHIP_BASE ?? "/").trim();
  if (!base.startsWith("/")) base = "/" + base;
  if (!base.endsWith("/")) base += "/";
  const site = (env.SHIP_SITE_URL ?? "https://shipmux.dev").replace(/\/$/, "");
  const sitePath = new URL(site).pathname.replace(/\/$/, "");
  if (sitePath !== base.replace(/\/$/, "")) {
    throw new Error(`SHIP_SITE_URL (${site}) must end with the base path (${base}); set both or neither`);
  }
  return { base, site };
}
