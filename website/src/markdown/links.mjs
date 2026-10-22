// Docs links are site-relative (/docs/tabs/), which is how scripts/prepare-docs.mjs writes them. This mdast plugin
// puts the base in front of them, so the same source works at / and under SHIP_BASE. A Sätteri plugin, like
// alerts.mjs; Sätteri nodes are read-only, so the change goes through ctx.setProperty. `base` has no trailing
// slash ('' or '/ship').
export const rebaseLinks = (base) => {
	const at = (node, ctx) => {
		if (node.url?.startsWith('/') && !node.url.startsWith('//')) ctx.setProperty(node, 'url', base + node.url);
	};
	return { name: 'ship-base-links', link: at, definition: at, image: at };
};
