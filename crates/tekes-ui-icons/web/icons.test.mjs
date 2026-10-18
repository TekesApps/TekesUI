import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {iconNames,getIconUrl} from './index.mjs';
const catalog=JSON.parse(await readFile(new URL('../catalog.json',import.meta.url),'utf8'));
test('all 176 public SVGs resolve with exact hashes, tintable geometry and no active content',async()=>{
  assert.equal(iconNames.length,176);assert.equal(new Set(iconNames).size,176);
  assert.deepEqual(iconNames,catalog.icons.map(icon=>icon.name));
  for(const icon of catalog.icons) {
    const svg=await readFile(fileURLToPath(getIconUrl(icon.name)),'utf8');
    assert.equal(createHash('sha256').update(svg).digest('hex'),icon.sha256,icon.name);
    assert.ok(svg.includes('viewBox="0 0 24 24"'));assert.ok(svg.includes('currentColor'));
    assert.ok(!/<(?:script|foreignObject|image|style)\b|\bon\w+\s*=|\b(?:href|src)\s*=|C2PA/i.test(svg),icon.name);
  }
});
test('retired names and unknown names fail instead of silently substituting artwork',()=>{
  for(const name of [...catalog.retired,'not-an-icon']) assert.throws(()=>getIconUrl(name),/Unknown TekesUI icon/);
});
test('published typed declarations and Rust catalog cover exactly the same names',async()=>{
  const types=await readFile(new URL('./index.d.ts',import.meta.url),'utf8');
  const rust=await readFile(new URL('../src/catalog.rs',import.meta.url),'utf8');
  for(const name of iconNames) {assert.ok(types.includes(JSON.stringify(name)));assert.ok(rust.includes(JSON.stringify(name)));}
});
