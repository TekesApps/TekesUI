#!/usr/bin/env python3
"""Compile registered Gallery BrandConfig profiles into deterministic palettes."""
import argparse
import hashlib
import importlib
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path('design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1')


def generate():
    sys.path.insert(0, str(ROOT / UPSTREAM / 'tools'))
    brand = importlib.import_module('brand')

    def load(path):
        return json.loads((ROOT / path).read_text())

    registry_path = 'design/brand/gallery.json'
    registry = load(registry_path)
    directions = registry['directions']
    direction_ids = {direction['id'] for direction in directions}
    presets = registry['presets']
    ids = [preset['id'] for preset in presets]
    indices = [preset['runtime_index'] for preset in presets]
    if len(set(ids)) != len(ids) or len(set(indices)) != len(indices):
        raise SystemExit('Gallery brand ids and runtime indexes must be unique')
    if sorted(indices) != list(range(len(indices))):
        raise SystemExit('Gallery brand runtime indexes must be contiguous from 0')
    for preset in presets:
        if not set(preset['recommended_for_direction']).issubset(direction_ids):
            raise SystemExit(f"Unknown recommended color direction for {preset['id']}")
        if preset['color_family'] not in direction_ids | {'neutral', 'clay'}:
            raise SystemExit(f"Unknown color family for {preset['id']}")

    base = load('design/source/base.tokens.json')
    policy = load('design/brand/policy.json')
    metadata = load('design/source/token-metadata.json')
    sources = [
        'design/source/base.tokens.json',
        'design/source/token-metadata.json',
        'design/brand/policy.json',
        registry_path,
        'scripts/generate-gallery-brands.py',
        str(UPSTREAM / 'tools' / 'brand.py'),
        str(UPSTREAM / 'tools' / 'brand_color.py'),
    ]
    compiled = []
    for preset in presets:
        config_path = preset['config']
        config = load(config_path)
        sources.append(config_path)
        bundle, _report = brand.compile_config(
            config, base=base, policy=policy, metadata=metadata
        )
        compiled.append({
            'id': preset['id'],
            'name': preset['name'],
            'runtime_index': preset['runtime_index'],
            'config': config_path,
            'seed_srgb': config['seed_srgb'],
            'category': preset['category'],
            'color_family': preset['color_family'],
            'recommended_for_direction': preset['recommended_for_direction'],
            'description': preset['description'],
            'color': bundle['color'],
        })
    source_hashes = {
        path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
        for path in sorted(set(sources))
    }
    return {
        'format': 'tekesui-gallery-brands/1',
        'inputs_sha256': source_hashes,
        'directions': directions,
        'reference_brand': registry['reference_brand'],
        'presets': compiled,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    output = ROOT / 'design/gallery/brands.json'
    data = (json.dumps(generate(), indent=2, ensure_ascii=False, sort_keys=True) + '\n').encode()
    if args.check:
        if not output.is_file() or output.read_bytes() != data:
            raise SystemExit('Stale Gallery brand projection: run scripts/generate-gallery-brands.py')
        print('Gallery brand profiles match the authoritative registry and compiler inputs.')
    else:
        output.write_bytes(data)
        print('Generated design/gallery/brands.json')


if __name__ == '__main__':
    main()
