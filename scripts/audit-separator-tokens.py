#!/usr/bin/env python3
"""Observe existing shared decorative rules through the public SeparatorView."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
HOST = Path('design/reviews/separator-scale-20261021/host')
spec = importlib.util.spec_from_file_location('navigation', ROOT / 'scripts/audit-navigation-tokens.py')
nav = importlib.util.module_from_spec(spec)
spec.loader.exec_module(nav)


def inputs():
    data = nav.inputs()
    paths = [p for p in (ROOT / HOST).rglob('*') if p.is_file() and p.suffix in ['.rs', '.toml', '.lock']]
    paths += [Path(__file__), ROOT / 'Cargo.lock', ROOT / 'crates/tekes-ui-controls/tests/separator_scale.rs', ROOT / 'crates/tekes-ui-gallery/src/gallery/workbench/extended.rs']
    for p in paths:
        data[p.relative_to(ROOT).as_posix()] = hashlib.sha256(p.read_bytes()).hexdigest()
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('use a fresh output path')
    before = inputs()
    command = ['cargo', 'test', '--manifest-path', str(HOST / 'Cargo.toml'), '--features', 'token-audit', '--test', 'tokens', '--locked']
    with tempfile.TemporaryDirectory(prefix='tekesui-separator-probe-') as folder:
        raw = Path(folder) / 'reads.json'
        subprocess.run(command, cwd=ROOT, env=dict(os.environ, TEKESUI_SEPARATOR_TOKEN_OUT=str(raw)), check=True)
        data = json.loads(raw.read_text())
    assert inputs() == before, 'source drift during observation'
    assert data['complete'] is False and len(data['cases']) == 144
    assert all(c['state_asserted'] and c['lookups'] and c['component'] == 'tui.separator' for c in data['cases'])
    data.update(inputs_sha256=before, probe_command=command, public_api_test=str(HOST / 'tests/tokens.rs'), native_acceptance='separate/unverified')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as stream:
        json.dump(data, stream, indent=2)
        stream.write('\n')
    print('Wrote144 bounded actual Separator observations; complete=False')


if __name__ == '__main__':
    main()
