// Build the site with every docs version, all static, one Astro build per git ref:
//   /docs/          the newest v* tag (no banner); main instead when there are no tags
//   /docs/<tag>/    every v* tag (older ones get an "older release" banner)
//   /docs/main/     main, with the "unreleased" banner (only when a tag exists)
// The site itself (landing, 404, theme, art) always comes from this checkout; only docs/site comes from
// each ref. Tags without docs/site predate the site and are skipped. Every build has its own Pagefind
// index under its own path, so search stays inside the version.
//
// usage: node scripts/build-versions.mjs <repo> <out> [--main <ref> | --main worktree]
//   --main picks what is published as main: a ref (default `main`), or `worktree` for <repo>/docs/site
//   as it is on disk, uncommitted changes included.
import { execFileSync } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const args = process.argv.slice(2);
const flag = args.indexOf("--main");
const mainRef = flag === -1 ? "main" : args.splice(flag, 2)[1];
const [repoArg, outArg] = args;
if (!repoArg || !outArg)
  throw new Error("usage: node scripts/build-versions.mjs <repo> <out> [--main <ref> | --main worktree]");
const repo = resolve(repoArg);
const out = resolve(outArg);
const site = resolve(import.meta.dirname, "..");
const DOCS = "docs/site";
const git = (...a) => execFileSync("git", ["-C", repo, ...a], { encoding: "utf8" }).trim();
const hasDocs = (ref) => {
  try {
    execFileSync("git", ["-C", repo, "cat-file", "-e", `${ref}:${DOCS}`], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
};

const allTags = git("tag", "--list", "v*", "--sort=-v:refname").split("\n").filter(Boolean);
const tags = allTags.filter(hasDocs);
for (const t of allTags) if (!tags.includes(t)) console.log(`skipping ${t}: it has no ${DOCS}`);
if (mainRef !== "worktree" && !hasDocs(mainRef))
  throw new Error(`${mainRef} has no ${DOCS}; pass --main <ref> or --main worktree`);
const latest = tags[0] ?? "";
const builds = [
  { ref: "main", from: mainRef, prefix: latest ? "/docs/main" : "/docs", channel: "main" },
  ...tags
    .slice()
    .reverse()
    .map((t) => ({ ref: t, from: t, prefix: `/docs/${t}`, channel: t === latest ? "latest" : "old" })),
  // last, so its root copies of llms.txt, agent-guide.md and the sitemap are the ones that stay
  ...(latest ? [{ ref: latest, from: latest, prefix: "/docs", channel: "latest" }] : []),
];

const work = mkdtempSync(join(tmpdir(), "ship-versions-"));
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
try {
  for (const [i, b] of builds.entries()) {
    let docs = join(repo, DOCS);
    if (b.from !== "worktree") {
      // docs only, straight from the ref: `git archive <ref> docs/site | tar -x`
      docs = join(work, `src-${i}`);
      mkdirSync(docs, { recursive: true });
      execFileSync("sh", [
        "-c",
        `git -C "$0" archive "$1" "$2" | tar -x -C "$3" --strip-components=2`,
        repo,
        b.from,
        DOCS,
        docs,
      ]);
    }
    const env = {
      ...process.env,
      DOCS_PREFIX: b.prefix,
      DOCS_REF: b.ref,
      DOCS_CHANNEL: b.channel,
      DOCS_LATEST: latest,
      DOCS_OUT: join(work, `out-${i}`),
    };
    console.log(`building ${b.ref} (${b.from}) -> ${b.prefix}/`);
    execFileSync("node", ["scripts/prepare-docs.mjs", docs], { cwd: site, env, stdio: "inherit" });
    execFileSync("pnpm", ["exec", "astro", "build"], { cwd: site, env, stdio: ["ignore", "ignore", "inherit"] });
    // every build carries the same landing, 404 and assets; its docs, index and agent files are its own
    cpSync(env.DOCS_OUT, out, { recursive: true });
  }
} finally {
  rmSync(work, { recursive: true, force: true });
}

// the version picker reads this at runtime, so old builds list releases made after them
const list = [
  { ref: "main", label: "main (unreleased)", path: latest ? "/docs/main" : "/docs" },
  ...tags.map((t) => ({
    ref: t,
    label: t === latest ? `${t} (latest)` : t,
    path: t === latest ? "/docs" : `/docs/${t}`,
  })),
];
writeFileSync(join(out, "docs/versions.json"), JSON.stringify(list, null, 1));
if (!existsSync(join(out, "index.html"))) throw new Error("no landing page in the output");
console.log(JSON.stringify(list));
