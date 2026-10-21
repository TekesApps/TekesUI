#!/usr/bin/env python3
"""Observe geometry-only GridView with actual themed child consumers outside its scope."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
HOST = Path('design/reviews/grid-layout-availability-20261021/host')
OUT = ROOT / HOST.parent
spec = importlib.util.spec_from_file_location('navigation', ROOT / 'scripts/audit-navigation-tokens.py')
nav = importlib.util.module_from_spec(spec)
spec.loader.exec_module(nav)


def inputs():
    data = nav.inputs()
    paths = [p for p in (ROOT / HOST).rglob('*') if p.is_file() and p.suffix in ['.rs', '.toml', '.lock']]
    paths += [Path(__file__), ROOT / 'Cargo.lock', ROOT / 'crates/tekes-ui-controls/tests/grid_layout_availability.rs', ROOT / 'crates/tekes-ui-gallery/src/gallery/workbench/extended.rs', ROOT / 'design/reviews/collection-focus-native-20261021/host/src/text_backend.rs']
    for p in paths:
        data[p.relative_to(ROOT).as_posix()] = hashlib.sha256(p.read_bytes()).hexdigest()
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('use a fresh evidence file')
    before = inputs()
    command = ['cargo', 'run', '--manifest-path', str(HOST / 'Cargo.toml'), '--features', 'token-audit', '--locked']
    subprocess.run(command, cwd=ROOT, env=dict(os.environ, TEKESUI_GRID_REVIEW_OUT=str(OUT / 'token-gpu'), TEKESUI_GRID_REVIEW_PHASE='after'), check=True)
    data = json.loads((OUT / 'grid-token-raw.json').read_text())
    assert inputs() == before, 'source drift during actual probe'
    assert data['complete'] is False and len(data['cases']) == 96
    assert all(c['state_asserted'] and not c['lookups'] and c['no_dynamic_lookups'] and c['zero_lookup_reason'] for c in data['cases'])
    data.update(inputs_sha256=before, probe_command=command, public_api_test=str(HOST / 'src/main.rs'), native_acceptance='separate/unverified')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as stream:
        json.dump(data, stream, indent=2)
        stream.write('\n')
    print('Wrote96 asserted zero-lookup GridView observations; child paint outside capture; complete=False')


if __name__ == '__main__':
    main()
