#!/usr/bin/env python3
"""Freeze bounded production popup geometry lookups, without state promotion."""
import argparse,hashlib,importlib.util,json,os,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('navigation',ROOT/'scripts/audit-navigation-tokens.py');nav=importlib.util.module_from_spec(spec);spec.loader.exec_module(nav)
def inputs():
    data=nav.inputs()
    for relative in ['crates/tekes-ui-controls/tests/popup_placement.rs','scripts/audit-popup-tokens.py']:
        data[relative]=hashlib.sha256((ROOT/relative).read_bytes()).hexdigest()
    return data
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    if args.output.exists():parser.error('use a fresh evidence path')
    before=inputs();command=['cargo','test','-p','tekes-ui-controls','--features','token-audit','--test','popup_placement','production_popup_geometry','--locked']
    with tempfile.TemporaryDirectory(prefix='tekesui-popup-') as folder:
        raw=Path(folder)/'reads.json';env=os.environ.copy();env['TEKESUI_POPUP_AUDIT_OUT']=str(raw);subprocess.run(command,cwd=ROOT,env=env,check=True);data=json.loads(raw.read_text())
    if before!=inputs():raise RuntimeError('source changed during probe')
    assert len(data['cases'])==192 and data['complete'] is False
    assert all(case['state_asserted'] is False for case in data['cases'])
    data.update(inputs_sha256=before,probe_command=command,public_api_test='crates/tekes-ui-controls/tests/popup_placement.rs',native_acceptance='separate')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x') as f:json.dump(data,f,indent=2);f.write('\n')
    print('Wrote192 geometry cases; complete=False; required states not promoted')
if __name__=='__main__':main()
