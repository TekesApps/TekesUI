#!/usr/bin/env python3
"""Observe Stack geometry and Surface paint through their actual public APIs."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('nav',ROOT/'scripts/audit-navigation-tokens.py');nav=importlib.util.module_from_spec(spec);spec.loader.exec_module(nav)
def inputs():
    data=nav.inputs()
    for p in [Path(__file__),ROOT/'crates/tekes-ui-controls/tests/layout_surface_tokens.rs',ROOT/'Cargo.lock']:
        data[p.relative_to(ROOT).as_posix()]=hashlib.sha256(p.read_bytes()).hexdigest()
    return data

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    if args.output.exists():parser.error('use a fresh output path')
    before=inputs();command=['cargo','test','--locked','-p','tekes-ui-controls','--features','token-audit','--test','layout_surface_tokens']
    with tempfile.TemporaryDirectory(prefix='tekesui-layout-surface-') as tmp:
        raw=Path(tmp)/'reads.json';subprocess.run(command,cwd=ROOT,env=dict(os.environ,TEKESUI_LAYOUT_SURFACE_AUDIT_OUT=str(raw)),check=True);data=json.loads(raw.read_text())
    assert inputs()==before,'source drift during actual observation'
    assert data['complete'] is False and len(data['cases'])==192
    assert sum(c['component']=='tui.stack' for c in data['cases'])==96
    assert all(c['state_asserted'] and (bool(c['lookups']) if c['component']=='tui.surface' else not c['lookups'] and c['no_dynamic_lookups'] and c['zero_lookup_reason']) for c in data['cases'])
    data.update(inputs_sha256=before,probe_command=command,public_api_test='crates/tekes-ui-controls/tests/layout_surface_tokens.rs',native_acceptance='separate/unverified')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x') as stream:json.dump(data,stream,indent=2);stream.write('\n')
    print('Wrote192 actual scoped observations; geometry and paint dependencies distinct; complete=False')
if __name__=='__main__':main()
