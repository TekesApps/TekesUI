#!/usr/bin/env python3
"""Observe public Tooltip lifecycle, including asserted zero-read closed renders."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('navigation', ROOT / 'scripts/audit-navigation-tokens.py')
nav = importlib.util.module_from_spec(spec)
spec.loader.exec_module(nav)


def inputs():
    data = nav.inputs()
    for path in [ROOT / 'examples/composition-consumer/tests/tooltip_tokens.rs',
                 ROOT / 'examples/composition-consumer/Cargo.toml',
                 ROOT / 'examples/composition-consumer/Cargo.lock',
                 ROOT / 'scripts/generate-component-capabilities.py', Path(__file__)]:
        data[path.relative_to(ROOT).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('use a fresh evidence path')
    before = inputs()
    command = ['cargo', 'test', '--manifest-path', 'examples/composition-consumer/Cargo.toml',
               '--features', 'token-audit', '--test', 'tooltip_tokens', '--locked']
    with tempfile.TemporaryDirectory(prefix='tekesui-tooltip-probe-') as folder:
        raw = Path(folder) / 'reads.json'
        env = dict(os.environ, TEKESUI_TOOLTIP_AUDIT_OUT=str(raw))
        subprocess.run(command, cwd=ROOT, env=env, check=True)
        data = json.loads(raw.read_text())
    assert inputs() == before, 'source drift during Tooltip observation'
    assert len(data['cases']) == 240 and data['complete'] is False
    assert all(c['state_asserted'] and bool(c['lookups']) != c['no_dynamic_lookups'] for c in data['cases'])
    data.update(inputs_sha256=before, probe_command=command,
                public_api_test='examples/composition-consumer/tests/tooltip_tokens.rs',
                native_acceptance='separate/unverified')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as handle:
        json.dump(data, handle, indent=2)
        handle.write('\n')
    print('Wrote240 bounded Tooltip observations; explicit closed zero reads, complete=False')


if __name__ == '__main__':
    main()
