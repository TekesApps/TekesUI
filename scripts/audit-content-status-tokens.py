#!/usr/bin/env python3
"""Bounded header + caller-owned content evidence through existing public APIs."""
import argparse, importlib.util, json, os, subprocess, tempfile, hashlib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('nav',ROOT/'scripts/audit-navigation-tokens.py')
nav=importlib.util.module_from_spec(spec);spec.loader.exec_module(nav)
def inputs():
    data=nav.inputs()
    paths=[*ROOT.glob('examples/composition-consumer/src/**/*.rs'),ROOT/'examples/composition-consumer/Cargo.toml',ROOT/'examples/composition-consumer/tests/scenarios.rs',Path(__file__)]
    data.update({p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in paths})
    return data
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    if args.output.exists():parser.error('use a fresh evidence path')
    before=inputs()
    with tempfile.TemporaryDirectory(prefix='tekesui-content-status-') as folder:
        folder=Path(folder);header=folder/'headers.json';body=folder/'body.json'
        subprocess.run(['python3',str(ROOT/'scripts/audit-navigation-tokens.py'),'--output',str(header)],cwd=ROOT,check=True)
        env=os.environ.copy();env['TEKESUI_CONTENT_STATUS_AUDIT_OUT']=str(body)
        command=['cargo','test','--manifest-path','examples/composition-consumer/Cargo.toml','--features','token-audit','--locked','--test','scenarios','content_status']
        subprocess.run(command,cwd=ROOT,env=env,check=True)
        first=json.loads(header.read_text());second=json.loads(body.read_text())
    if inputs()!=before:raise RuntimeError('source changed while executing probes')
    assert len(first['cases'])==288 and len(second['cases'])==120
    data={'format':first['format'],'complete':False,'scope':first['scope'].replace('10 content-status requirements/combinations remain deferred, not reinterpreted as header color flags.','')+' '+second['scope'],
          'inputs_sha256':before,'public_api_test':'examples/composition-consumer/tests/scenarios.rs',
          'cases':first['cases']+second['cases'],'excluded':list(dict.fromkeys([x for x in first['excluded'] if x!='content loading/error/empty contracts']+second['excluded'])),
          'header_only_deferred':first['deferred'],'content_status_coverage':'caller-owned public composition verified; no new builtin adapter or broad migration',
          'probe_command':command,'native_acceptance':'not inferred from API probes'}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x') as f:json.dump(data,f,indent=2);f.write('\n')
    print(f'Wrote {args.output}:408 bounded cases; complete=False; native acceptance separate')
if __name__=='__main__':main()
