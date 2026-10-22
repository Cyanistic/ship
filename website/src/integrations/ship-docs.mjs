// After the build: put this version's search index under its own docs path, and copy in the agent files
// (raw markdown pages, llms.txt, agent-guide.md) that scripts/prepare-docs.mjs wrote to .generated/raw/.
import { cpSync, existsSync, readFileSync, renameSync, rmSync, writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

export default function shipDocs() {
	return {
		name: 'ship-docs',
		hooks: {
			'astro:build:done': ({ dir, logger }) => {
				const out = fileURLToPath(dir);
				const gen = fileURLToPath(new URL('../../.generated/', import.meta.url));
				const meta = JSON.parse(readFileSync(join(gen, 'site.json'), 'utf8'));
				// Starlight runs Pagefind first and writes <out>/pagefind/. Each version keeps its own index.
				const index = join(out, meta.prefix, 'pagefind');
				if (existsSync(join(out, 'pagefind'))) {
					rmSync(index, { recursive: true, force: true });
					renameSync(join(out, 'pagefind'), index);
				}
				cpSync(join(gen, 'raw'), out, { recursive: true });
				// a single build lists only itself; scripts/build-versions.mjs writes the full list
				const versions = join(out, meta.root, 'versions.json');
				if (!existsSync(versions)) {
					mkdirSync(join(out, meta.root), { recursive: true });
					writeFileSync(versions, JSON.stringify([{ ref: meta.ref, label: meta.channel === 'main' ? `${meta.ref} (unreleased)` : meta.ref, path: meta.prefix }], null, 1));
				}
				logger.info(`search index at ${meta.prefix}/pagefind/, agent files copied`);
			},
		},
	};
}
