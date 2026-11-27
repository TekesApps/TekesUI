#!/usr/bin/env python3
"""Project the canonical public icon names/paths into existing Web entry points."""
import argparse
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]

def main():
    args = argparse.ArgumentParser()
    args.add_argument('--check', action='store_true')
    check = args.parse_args().check
    catalog = json.loads((root / 'crates/tekes-ui-icons/catalog.json').read_text())
    icons = catalog['icons']
    names = [icon['name'] for icon in icons]
    assert len(names) == len(set(names)), 'duplicate canonical icon'
    aliases = catalog.get('aliases', {})
    for alias, target in aliases.items():
        assert alias not in names, f'alias collides with canonical icon: {alias}'
        assert target in names, f'alias target missing from canonical catalog: {alias} -> {target}'
    javascript = '// Generated from catalog.json. Original SVG geometry is owned by TekesUI.\n'
    javascript += 'export const iconNames = Object.freeze(' + json.dumps(names) + ');\n'
    javascript += 'export const iconAliases = Object.freeze(' + json.dumps(aliases, sort_keys=True) + ');\n'
    javascript += 'const urls = Object.freeze({\n'
    for icon in icons:
        javascript += '  ' + json.dumps(icon['name']) + ': new URL(' + json.dumps('../' + icon['file']) + ', import.meta.url).href,\n'
    javascript += (
        '});\nexport function getIconUrl(name) {\n'
        '  const canonical = Object.hasOwn(iconAliases, name) ? iconAliases[name] : name;\n'
        '  if (!Object.hasOwn(urls, canonical)) throw new Error(`Unknown TekesUI icon: ${name}`);\n'
        '  return urls[canonical];\n'
        '}\n'
    )
    alias_keys = sorted(aliases)
    types = '// Generated from catalog.json.\n'
    types += 'export type CanonicalIconName = ' + ' | '.join(json.dumps(name) for name in names) + ';\n'
    if alias_keys:
        types += 'export type LegacyIconAlias = ' + ' | '.join(json.dumps(alias) for alias in alias_keys) + ';\n'
        types += 'export type IconName = CanonicalIconName | LegacyIconAlias;\n'
        types += 'export declare const iconNames: readonly IconName[];\n'
        types += 'export declare const iconAliases: Readonly<Record<LegacyIconAlias, CanonicalIconName>>;\n'
        types += 'export declare function getIconUrl(name: IconName): string;\n'
    else:
        types += 'export type IconName = CanonicalIconName;\n'
        types += 'export declare const iconNames: readonly CanonicalIconName[];\n'
        types += 'export declare const iconAliases: Readonly<Record<string, never>>;\n'
        types += 'export declare function getIconUrl(name: IconName): string;\n'
    for name, content in [('index.mjs', javascript), ('index.d.ts', types)]:
        path = root / 'crates/tekes-ui-icons/web' / name
        if check:
            if path.read_text() != content:
                raise SystemExit('Stale icon Web projection: ' + str(path))
        else:
            path.write_text(content)
    print('Canonical icon Web projection matches' if check else 'Generated canonical icon Web projection')

if __name__ == '__main__':
    main()
