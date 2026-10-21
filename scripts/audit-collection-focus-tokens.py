#!/usr/bin/env python3
"""Observe public collection focus availability via the exact Gallery adapter."""
import argparse,hashlib,importlib.util,json,os,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
HOST=Path('design/reviews/collection-focus-ink-20261021/host')
spec=importlib.util.spec_from_file_location('navigation',ROOT/'scripts/audit-navigation-tokens.py');nav=importlib.util.module_from_spec(spec);spec.loader.exec_module(nav)
def inputs():
 data=nav.inputs();paths=[*pinned_host(),ROOT/'crates/tekes-ui-gallery/src/gallery/workbench/extended/collections_adapter.rs',Path(__file__),ROOT/'Cargo.lock',ROOT/'design/reviews/collection-focus-availability-20261020/host/src/fixture.rs',ROOT/'design/reviews/collection-focus-availability-20261020/host/src/text_backend.rs']
 for p in paths:data[p.relative_to(ROOT).as_posix()]=hashlib.sha256(p.read_bytes()).hexdigest()
 return data
def pinned_host():
 return [p for p in (ROOT/HOST).rglob('*') if p.is_file() and p.suffix in ['.rs','.toml','.lock']]
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
 if args.output.exists():parser.error('use a fresh output path')
 before=inputs();command=['cargo','test','--manifest-path',str(HOST/'Cargo.toml'),'--features','token-audit','--test','collection_tokens','--locked']
 with tempfile.TemporaryDirectory(prefix='tekesui-collection-focus-') as folder:
  raw=Path(folder)/'reads.json';subprocess.run(command,cwd=ROOT,env=dict(os.environ,TEKESUI_COLLECTION_FOCUS_AUDIT_OUT=str(raw)),check=True);data=json.loads(raw.read_text())
 assert inputs()==before,'source drift during observation'
 assert len(data['cases'])==504 and data['complete'] is False and all(c['state_asserted'] and c['lookups'] for c in data['cases'])
 data.update(inputs_sha256=before,probe_command=command,public_api_test=(HOST/'tests/collection_tokens.rs').as_posix(),native_acceptance='separate/unverified')
 args.output.parent.mkdir(parents=True,exist_ok=True)
 with args.output.open('x') as output:json.dump(data,output,indent=2);output.write('\n')
 print('Wrote504 asserted collection focus/availability observations; complete=False')
if __name__=='__main__':main()
