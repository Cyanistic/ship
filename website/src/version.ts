// Which docs version this build is, as scripts/prepare-docs.mjs recorded it.
// channel: 'main' (unreleased, banner), 'latest' (newest tag, served at /docs/), 'old' (an older tag)
import meta from '../.generated/site.json';

export const version = {
	ref: meta.ref,
	channel: meta.channel as 'main' | 'latest' | 'old',
	latest: meta.latest,
	root: meta.root, // where versions.json and the newest docs live
	prefix: meta.prefix, // this build's docs path, like /docs or /docs/main
};

/** The GitHub source of the page at `path` (a docs URL like /docs/tabs/), at this build's ref. */
export function sourceOf(path: string): string | undefined {
	const page = (meta.pages as Record<string, { source: string; raw: string }>)[path];
	return page && `${meta.repo}/blob/${meta.ref}/${page.source}`;
}
