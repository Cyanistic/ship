// Shiki's TOML grammar, with the inside of a quoted key ("alt-q" = ...) given a scope of its own.
// Upstream leaves those characters unscoped, so no theme can colour them; the ansi design draws quoted
// keys green, like strings. src/ship16.theme.json colours `string.quoted.key.toml` and the key's quotes.
import toml from '@shikijs/langs/toml';

const quotedKey = new Set(['((")(.*?)("))\\s*(=)\\s*', "((')([^']*)('))\\s*(=)\\s*"]);
function patch(node) {
	if (Array.isArray(node)) return node.map(patch);
	if (!node || typeof node !== 'object') return node;
	const out = Object.fromEntries(Object.entries(node).map(([k, v]) => [k, patch(v)]));
	if (quotedKey.has(out.begin)) out.captures = { ...out.captures, 3: { ...out.captures[3], name: 'string.quoted.key.toml' } };
	return out;
}

const grammar = patch(toml[0]);
if (!JSON.stringify(grammar).includes('string.quoted.key.toml')) throw new Error('the TOML grammar changed; src/markdown/toml.mjs no longer finds its quoted keys');
export default grammar;
