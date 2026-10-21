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
    icons = json.loads((root / 'crates/tekes-ui-icons/catalog.json').read_text())['icons']
    names = [icon['name'] for icon in icons]
    assert len(names) == len(set(names)), 'duplicate canonical icon'
    javascript = '// Generated from catalog.json. Original SVG geometry is owned by TekesUI.\n'
    javascript += 'export const iconNames = Object.freeze(' + json.dumps(names) + ');\n'
    javascript += 'const urls = Object.freeze({\n'
    for icon in icons:
        javascript += '  ' + json.dumps(icon['name']) + ': new URL(' + json.dumps('../' + icon['file']) + ', import.meta.url).href,\n'
    javascript += '});\nexport function getIconUrl(name) {\n  if (!Object.hasOwn(urls, name)) throw new Error(`Unknown TekesUI icon: ${name}`);\n  return urls[name];\n}\n'
    types = '// Generated from catalog.json.\nexport type IconName = ' + ' | '.join(json.dumps(name) for name in names) + ';\n'
    types += 'export declare const iconNames: readonly IconName[];\nexport declare function getIconUrl(name: IconName): string;\n'
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
