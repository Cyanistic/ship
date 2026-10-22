import GithubSlugger from 'github-slugger';

// Starlight gives every page title the id `_top`. The ansi docs anchor the h1 by its own name
// (#configuration), so the title gets its slug, or `<slug>-page` when a heading already uses it.
export function titleId(route: { entry: { data: { title: string } }; headings: { slug: string }[] }): string {
	const id = new GithubSlugger().slug(route.entry.data.title);
	return route.headings.some((h) => h.slug === id) ? `${id}-page` : id;
}
