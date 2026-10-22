// The site's base path, from SHIP_BASE through astro.config.mjs: '' at the domain root, '/ship' on GitHub Pages.
// Paths in this codebase are site-relative ('/docs/', '/css/site.css'); anything that becomes a URL in the
// browser goes through withBase(), or the link breaks under a base.
export const base = import.meta.env.BASE_URL.replace(/\/$/, '');

/** A site-relative path like '/docs/' as a URL under the base. */
export const withBase = (path: string): string => base + path;

/** Rebases the root-absolute href and src attributes of a raw HTML string (the landing body is one). */
export const rebase = (html: string): string => (base ? html.replace(/(\s(?:href|src)=")\/(?!\/)/g, `$1${base}/`) : html);
