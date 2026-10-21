#!/usr/bin/env python3
"""Observe bounded MenuContent production lookups through a real public consumer."""
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
    for path in [*ROOT.glob('examples/composition-consumer/src/**/*.rs'), ROOT / 'examples/composition-consumer/tests/menu_content_tokens.rs', ROOT / 'examples/composition-consumer/Cargo.toml', ROOT / 'examples/composition-consumer/Cargo.lock', Path(__file__)]:
        data[path.relative_to(ROOT).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
    return data

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('use a fresh evidence path')
    before = inputs()
    command = ['cargo', 'test', '--manifest-path', 'examples/composition-consumer/Cargo.toml', '--features', 'token-audit', '--test', 'menu_content_tokens', '--locked']
    with tempfile.TemporaryDirectory(prefix='tekesui-menu-content-') as folder:
        raw = Path(folder) / 'reads.json'
        env = os.environ.copy()
        env['TEKESUI_MENU_CONTENT_AUDIT_OUT'] = str(raw)
        subprocess.run(command, cwd=ROOT, env=env, check=True)
        data = json.loads(raw.read_text())
    if inputs() != before:
        raise RuntimeError('source changed during probe')
    assert len(data['cases']) == 288 and data['complete'] is False
    assert all(case['state_asserted'] and case['lookups'] for case in data['cases'])
    data.update(inputs_sha256=before, probe_command=command, public_api_test='examples/composition-consumer/tests/menu_content_tokens.rs', native_acceptance='separate')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as handle:
        json.dump(data, handle, indent=2)
        handle.write('\n')
    print('Wrote288 bounded actual MenuContent observations; complete=False')
if __name__ == '__main__':
    main()
