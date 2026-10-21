#!/usr/bin/env python3
"""Run the existing public action probe and freeze bounded lookup evidence.

Use a fresh output path; historical evidence is never overwritten. This is an
explicit development audit, not a benchmark or a complete dependency extractor.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def inputs():
    paths = set()
    for prefix in ['crates/tekes-ui-controls/src', 'crates/tekes-ui-tokens/src',
                   'crates/tekes-ui-design-export/src']:
        paths.update(p.relative_to(ROOT).as_posix() for p in (ROOT / prefix).rglob('*.rs'))
    for prefix in ['design/source', 'design/brand']:
        paths.update(p.relative_to(ROOT).as_posix() for p in (ROOT / prefix).rglob('*.json'))
    paths.update(['design/tokens.json', 'crates/tekes-ui-controls/tests/action_contract.rs',
                  'crates/tekes-ui-controls/Cargo.toml', 'crates/tekes-ui-tokens/Cargo.toml',
                  'scripts/audit-action-tokens.py',
                  'design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/registry/components.json'])
    return {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in sorted(paths)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='new, non-existing evidence file')
    args = parser.parse_args()
    if args.output.exists():
        parser.error('output exists; choose a fresh evidence path to protect prior results')
    before = inputs()
    command = ['cargo', 'test', '-p', 'tekes-ui-controls', '--features', 'token-audit',
               '--test', 'action_contract', '--locked']
    with tempfile.TemporaryDirectory(prefix='tekesui-action-') as folder:
        raw = Path(folder) / 'lookups.json'
        env = os.environ.copy()
        env['TEKESUI_ACTION_AUDIT_OUT'] = str(raw)
        subprocess.run(command, cwd=ROOT, env=env, check=True)
        data = json.loads(raw.read_text())
    if inputs() != before:
        raise RuntimeError('source changed during the probe; no evidence written')
    assert data['complete'] is False and len(data['cases']) == 576
    assert {case['component'] for case in data['cases']} == {'tui.button', 'tui.icon-button', 'tui.link', 'tui.checkbox', 'tui.segmented-control'}
    data.update(inputs_sha256=before, public_api_test='crates/tekes-ui-controls/tests/action_contract.rs',
                probe_command=command, assertions=['all 31 required states and 17 registered combinations via public render/event APIs',
                'stable frame, controlled value and accessible icon name',
                'disabled/loading block activation', 'Link Space remains unconsumed',
                'latest model flags before pointer/keyboard activation',
                'observed disabled interval cancels stale Space arming'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    # Exclusive creation also protects an output created while the probe ran.
    with args.output.open('x') as handle:
        json.dump(data, handle, indent=2)
        handle.write('\n')
    print(f'Wrote {args.output}: 576 bounded cases; complete=False; native acceptance unchanged')


if __name__ == '__main__':
    main()
