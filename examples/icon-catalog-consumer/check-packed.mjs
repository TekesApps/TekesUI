// Run against an extracted npm pack, not the repository's loose files.
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL, fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
const root = process.argv[2];
assert.ok(root, 'pass the extracted package directory');
const {iconNames, getIconUrl} = await import(pathToFileURL(join(root, 'crates/tekes-ui-icons/web/index.mjs')));
const catalog = JSON.parse(await readFile(join(root, 'crates/tekes-ui-icons/catalog.json'), 'utf8'));
assert.deepEqual(iconNames, catalog.icons.map(i => i.name));
for (const entry of catalog.icons) {
  const svg = await readFile(fileURLToPath(getIconUrl(entry.name)), 'utf8');
  assert.match(svg, /<svg\b[^>]*\bwidth="24"/);
  assert.match(svg, /<svg\b[^>]*\bheight="24"/);
  assert.ok(svg.includes('viewBox="0 0 24 24"'));
  assert.equal(createHash('sha256').update(svg).digest('hex'), entry.sha256);
}
assert.throws(() => getIconUrl('missing-icon'), /Unknown TekesUI icon/);
console.log(`${iconNames.length} square, hash-matched public assets resolved from the packed package`);
