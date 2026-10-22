// Turns one checkout of docs/site into what a single Astro build needs.
//
//   src/content/docs/       the pages for Starlight: H1 moved into frontmatter, links rewritten to site URLs
//   .generated/site.json    this build's version, sidebar and page list (read by astro.config.mjs and src/version.ts)
//   .generated/raw/         the agent files, copied into the build output by src/integrations/ship-docs.mjs:
//                           <prefix>/<page>.md, <prefix>/llms.txt, <prefix>/agent-guide.md (and /llms.txt,
//                           /agent-guide.md when this build is the one served at /docs/)
//
// Sources stay plain GitHub markdown: each page starts with `# Title`, links between pages are relative `.md`
// links, and links that leave docs/site point at other repo files. The sidebar order comes from the overview
// (docs/site/README.md): each `## Heading` followed by a list of page links becomes a sidebar group.
//
// usage: node scripts/prepare-docs.mjs [docs-dir]
// env:   DOCS_PREFIX (/docs), DOCS_REF (main), DOCS_CHANNEL (main | latest | old), DOCS_LATEST (newest tag or ''),
//        SHIP_BASE and SHIP_SITE_URL (see site-env.mjs). Links to other pages stay site-relative (/docs/x/): the
//        Astro build adds the base. The raw copies and llms.txt use absolute URLs under SHIP_SITE_URL.
import { readFileSync, writeFileSync, mkdirSync, rmSync, readdirSync, statSync } from "node:fs";
import { join, dirname, resolve, relative, posix } from "node:path";
import { fromMarkdown } from "mdast-util-from-markdown";
import { gfmFromMarkdown } from "mdast-util-gfm";
import { gfm } from "micromark-extension-gfm";
import { toString } from "mdast-util-to-string";
import { visit } from "unist-util-visit";
import { siteEnv } from "./site-env.mjs";

const site = resolve(import.meta.dirname, "..");
const src = resolve(process.argv[2] ?? join(site, "../docs/site"));
const REPO_DOCS = "docs/site"; // where src sits in the repository, for links that leave it
const REPO = "https://github.com/shipmux/ship";
// links to the repo README that the landing page covers go there instead of to GitHub, keyed by the README anchor.
// Anything else in the README (the agent guide's "not built yet", which names the README as the source of truth) stays on GitHub.
const LANDING = { "": "/", "what-works-today": "/#deck" };
const prefix = (process.env.DOCS_PREFIX ?? "/docs").replace(/\/$/, "");
const ref = process.env.DOCS_REF ?? "main";
const channel = process.env.DOCS_CHANNEL ?? "main";
const latest = process.env.DOCS_LATEST ?? "";
const { site: SITE } = siteEnv();
// the sources name the placeholder domain (docs/site is not ours to edit), so every write swaps it for SITE
const PLACEHOLDER_SITE = "https://shipmux.dev";
const AGENT_GUIDE = "agent-guide.md";
// the build at /docs/ also owns the root copies, which is where the overview's agent prompt points
const llmsPath = prefix === "/docs" ? "/llms.txt" : `${prefix}/llms.txt`;

const content = join(site, "src/content/docs");
const gen = join(site, ".generated");
const raw = join(gen, "raw");

// ---------------------------------------------------------------- read the sources
const files = [];
(function walk(dir) {
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p);
    else if (name.endsWith(".md")) files.push(relative(src, p).split("\\").join("/"));
  }
})(src);
if (!files.includes("README.md"))
  throw new Error(`${src} has no README.md, which is the overview and the sidebar order`);

/** docs/site path -> the page's slug: README.md is its folder's index */
const slugOf = (p) => p.replace(/\.md$/, "").replace(/(^|\/)README$/, "");
const pageUrl = (slug) => `${prefix}/${slug ? slug + "/" : ""}`;
const rawUrl = (slug) => `${prefix}/${slug || "index"}.md`;

const parse = (text) => fromMarkdown(text, { extensions: [gfm()], mdastExtensions: [gfmFromMarkdown()] });

const pages = new Map(); // path -> { slug, title, text, tree }
for (const p of files) {
  const text = readFileSync(join(src, p), "utf8");
  const tree = parse(text);
  const h1 = tree.children.find((n) => n.type === "heading" && n.depth === 1);
  if (!h1) throw new Error(`${p}: a page starts with a "# Title" heading`);
  pages.set(p, { slug: slugOf(p), title: toString(h1), text, tree, h1 });
}

// ---------------------------------------------------------------- links
/** Rewrite every relative link in a page. `toPage(slug, hash)` decides where links to other pages go, `toSite(path)` where site paths go. */
function rewrite(p, { text, tree }, toPage, toSite = (path) => path) {
  const edits = [];
  visit(tree, ["link", "definition", "image"], (node) => {
    const url = node.url;
    if (!url || /^[a-z][a-z0-9+.-]*:/i.test(url) || url.startsWith("#") || url.startsWith("/")) return;
    const [path, hash = ""] = url.split("#");
    const target = posix.normalize(posix.join(posix.dirname(p), path));
    let to;
    if (pages.has(target)) to = toPage(pages.get(target).slug, hash ? "#" + hash : "");
    else if (target === "../../README.md" && hash in LANDING) to = toSite(LANDING[hash]);
    else if (target.startsWith("../")) {
      // a repo file outside docs/site: link to it on GitHub at this build's ref
      const repoPath = posix.normalize(posix.join(REPO_DOCS, target));
      to = `${REPO}/blob/${ref}/${repoPath}${hash ? "#" + hash : ""}`;
    } else throw new Error(`${p}: link to ${url} points at nothing in ${REPO_DOCS}`);
    const { start, end } = node.position;
    const slice = text.slice(start.offset, end.offset);
    const at = node.type === "definition" ? slice.indexOf(url) : slice.lastIndexOf("](" + url) + 2;
    if (at < (node.type === "definition" ? 0 : 2))
      throw new Error(`${p}: could not find the link ${url} in its source`);
    edits.push([start.offset + at, url.length, to]);
  });
  let out = text;
  for (const [at, len, to] of edits.sort((a, b) => b[0] - a[0])) out = out.slice(0, at) + to + out.slice(at + len);
  return out;
}

// ---------------------------------------------------------------- sidebar, from the overview's lists
const overview = pages.get("README.md");
const sidebar = [{ label: overview.title, link: pageUrl("") }];
const groups = [];
{
  const kids = overview.tree.children;
  for (let i = 0; i < kids.length; i++) {
    const h = kids[i];
    if (h.type !== "heading" || h.depth !== 2) continue;
    let j = i + 1;
    while (j < kids.length && kids[j].type !== "heading" && kids[j].type !== "list") j++;
    const list = kids[j];
    if (!list || list.type !== "list") continue;
    const items = [];
    for (const li of list.children) {
      const link = li.children[0]?.children?.find((n) => n.type === "link");
      const target = link && posix.normalize(posix.join(".", link.url.split("#")[0]));
      if (!link || !pages.has(target) || link.url.includes("#")) {
        items.length = 0;
        break;
      }
      items.push(target);
    }
    if (items.length) groups.push({ label: toString(h).toLowerCase(), items });
  }
}
const listed = new Set(["README.md", AGENT_GUIDE, ...groups.flatMap((g) => g.items)]);
const unlisted = files.filter((p) => !listed.has(p));
if (unlisted.length)
  throw new Error(`not in any list in ${REPO_DOCS}/README.md, so not in the sidebar: ${unlisted.join(", ")}`);
for (const g of groups) {
  sidebar.push({
    label: g.label,
    items: g.items.map((p) => ({ label: pages.get(p).title, link: pageUrl(pages.get(p).slug) })),
  });
}

// ---------------------------------------------------------------- write
rmSync(content, { recursive: true, force: true });
rmSync(gen, { recursive: true, force: true });
const put = (path, text) => {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text.replaceAll(PLACEHOLDER_SITE, SITE));
};
const yaml = (s) => JSON.stringify(s); // JSON strings are valid YAML scalars

const header = `> These are the Ship docs (${ref}) as markdown. The index of every page is at ${SITE}${llmsPath}\n\n`;
for (const [p, page] of pages) {
  const toRaw = (slug, hash) => SITE + rawUrl(slug) + hash;
  const rawText = header + rewrite(p, page, toRaw, (path) => SITE + path);
  if (p === AGENT_GUIDE) {
    put(join(raw, prefix, AGENT_GUIDE), rawText);
    if (prefix === "/docs") put(join(raw, AGENT_GUIDE), rawText);
    continue; // the agent guide is served only as markdown
  }
  put(join(raw, rawUrl(page.slug)), rawText);

  // Starlight: the title moves into frontmatter, the H1 line goes, links point at the HTML pages
  const html = rewrite(p, page, (slug, hash) => pageUrl(slug) + hash);
  const { start } = page.h1.position; // the H1 comes first, so the link edits after it leave its offset alone
  const before = html.slice(0, start.offset);
  if (before.trim()) throw new Error(`${p}: the "# Title" heading must be the first thing in the file`);
  const endAt = html.indexOf("\n", start.offset);
  const body = html.slice(endAt === -1 ? html.length : endAt + 1).replace(/^\n+/, "");
  const id = (prefix + "/" + page.slug).replace(/^\//, "").replace(/\/$/, "");
  put(join(content, p), `---\ntitle: ${yaml(page.title)}\nslug: ${yaml(id)}\n---\n\n${body}`);
}

// llms.txt: a small index of links, nothing else
const line = (p) => `- [${pages.get(p).title}](${SITE}${rawUrl(pages.get(p).slug)})`;
const llms = [
  "# Ship",
  "",
  `> Ship is a small terminal workspace for running several coding agents at once. Every page of its docs (${ref}) as markdown.`,
  "",
  ...(pages.has(AGENT_GUIDE)
    ? [
        "## Start here",
        "",
        `- [Agent guide](${SITE}${prefix === "/docs" ? "" : prefix}/${AGENT_GUIDE}): for an agent helping a person set Ship up`,
        `- ${line("README.md").slice(2)}`,
        "",
      ]
    : []),
  ...groups.flatMap((g) => [`## ${g.label[0].toUpperCase()}${g.label.slice(1)}`, "", ...g.items.map(line), ""]),
].join("\n");
put(join(raw, prefix, "llms.txt"), llms);
if (prefix === "/docs") put(join(raw, "llms.txt"), llms);

const meta = {
  prefix,
  ref,
  channel,
  latest,
  root: "/docs",
  repo: REPO,
  repoDocs: REPO_DOCS,
  sidebar,
  pages: Object.fromEntries(
    [...pages]
      .filter(([p]) => p !== AGENT_GUIDE)
      .map(([p, pg]) => [pageUrl(pg.slug), { source: `${REPO_DOCS}/${p}`, raw: rawUrl(pg.slug) }]),
  ),
};
put(join(gen, "site.json"), JSON.stringify(meta, null, 1));
console.log(`prepared ${pages.size} docs pages from ${relative(process.cwd(), src) || "."} for ${prefix}/ (${ref})`);
