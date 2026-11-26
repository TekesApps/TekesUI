import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {iconNames,iconAliases,getIconUrl} from './index.mjs';
const catalog=JSON.parse(await readFile(new URL('../catalog.json',import.meta.url),'utf8'));
test('all catalog public SVGs resolve with exact hashes, tintable geometry and no active content',async()=>{
  assert.equal(iconNames.length,catalog.icons.length);assert.equal(new Set(iconNames).size,catalog.icons.length);
  assert.ok(iconNames.includes("tekes-swap-horizontal"),"published canonical WIP icon must remain consumable");
  assert.ok(iconNames.includes("tekes-chat"));assert.ok(iconNames.includes("tekes-check-circle"));assert.ok(iconNames.includes("tekes-folder-edit"));
  assert.deepEqual(iconNames,catalog.icons.map(icon=>icon.name));
  for(const icon of catalog.icons) {
    const svg=await readFile(fileURLToPath(getIconUrl(icon.name)),'utf8');
    assert.equal(createHash('sha256').update(svg).digest('hex'),icon.sha256,icon.name);
    assert.ok(svg.includes('viewBox="0 0 24 24"'));assert.ok(svg.includes('currentColor'));
    assert.match(svg, /<svg\b[^>]*\bwidth="24"/, icon.name+' square width');
    assert.match(svg, /<svg\b[^>]*\bheight="24"/, icon.name+' square height');
    assert.ok(!/<(?:script|foreignObject|image|style)\b|\bon\w+\s*=|\b(?:href|src)\s*=|C2PA/i.test(svg),icon.name);
  }
});
test('legacy workflow icon aliases resolve to neutral canonical SVGs and keep matching asset files',async()=>{
  assert.deepEqual(iconAliases, catalog.aliases);
  const byName = new Map(catalog.icons.map(icon => [icon.name, icon]));
  for(const [alias, canonical] of Object.entries(catalog.aliases)) {
    assert.equal(getIconUrl(alias), getIconUrl(canonical), alias);
    const aliasSvg = await readFile(new URL(`../assets/${alias}.svg`, import.meta.url), 'utf8');
    assert.equal(createHash('sha256').update(aliasSvg).digest('hex'), byName.get(canonical).sha256, alias);
  }
});
test('retired names and unknown names fail instead of silently substituting artwork',()=>{
  for(const name of [...catalog.retired,'not-an-icon']) assert.throws(()=>getIconUrl(name),/Unknown TekesUI icon/);
});
test('published typed declarations and Rust catalog cover exactly the same names',async()=>{
  const types=await readFile(new URL('./index.d.ts',import.meta.url),'utf8');
  const rust=await readFile(new URL('../src/catalog.rs',import.meta.url),'utf8');
  const canonicalTypeLine=types.match(/^export type CanonicalIconName = .*$/m)?.[0] ?? '';
  const aliasTypeLine=types.match(/^export type LegacyIconAlias = .*$/m)?.[0] ?? '';
  const iconTypeLine=types.match(/^export type IconName = .*$/m)?.[0] ?? '';
  const typedNames = [...canonicalTypeLine.matchAll(/"(tekes-[^"]+)"/g)].map(match=>match[1]);
  const typedAliases = [...aliasTypeLine.matchAll(/"(tekes-[^"]+)"/g)].map(match=>match[1]);
  const rustNames = [...rust.matchAll(/name: "(tekes-[^"]+)"/g)].map(match=>match[1]);
  assert.deepEqual(typedNames, iconNames);
  assert.deepEqual(typedAliases, Object.keys(iconAliases).sort());
  assert.equal(iconTypeLine, 'export type IconName = CanonicalIconName | LegacyIconAlias;');
  assert.deepEqual(rustNames.sort(), [...iconNames].sort());
});
