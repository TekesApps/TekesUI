#!/usr/bin/env python3
"""Observe existing Progress, StatusBadge and Skeleton public paint contracts."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('nav', ROOT/'scripts/audit-navigation-tokens.py')
nav = importlib.util.module_from_spec(spec)
spec.loader.exec_module(nav)
def inputs():
    data = nav.inputs()
    for p in [Path(__file__), ROOT/'crates/tekes-ui-controls/tests/passive_feedback_contract.rs', ROOT/'Cargo.lock']:
        data[p.relative_to(ROOT).as_posix()] = hashlib.sha256(p.read_bytes()).hexdigest()
    return data
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('use a fresh output path')
    before = inputs()
    command = ['cargo','test','--locked','-p','tekes-ui-controls','--features','token-audit','--test','passive_feedback_contract']
    with tempfile.TemporaryDirectory(prefix='tekesui-passive-feedback-') as folder:
        raw = Path(folder)/'reads.json'
        subprocess.run(command, cwd=ROOT, env=dict(os.environ,TEKESUI_PASSIVE_FEEDBACK_AUDIT_OUT=str(raw)),check=True)
        data = json.loads(raw.read_text())
    assert inputs() == before, 'source drift during observation'
    assert data['complete'] is False and len(data['cases']) == 216
    assert all(c['state_asserted'] and c['lookups'] for c in data['cases'])
    data.update(inputs_sha256=before, public_api_test='crates/tekes-ui-controls/tests/passive_feedback_contract.rs',probe_command=command,native_acceptance='separate/unverified')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x') as stream:
        json.dump(data,stream,indent=2)
        stream.write('\n')
    print('Wrote216 actual public-view observations; complete=False')
if __name__ == '__main__':
    main()
