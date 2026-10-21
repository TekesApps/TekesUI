#!/usr/bin/env python3
"""Bounded original Select/View and independent caller refresh observations."""
import argparse, hashlib, importlib.util, json, os, subprocess, tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('choice',ROOT/'scripts/audit-choice-selection-tokens.py')
choice=importlib.util.module_from_spec(spec);spec.loader.exec_module(choice)
def inputs():
    data=choice.inputs()
    paths=[*ROOT.glob('examples/composition-consumer/src/**/*.rs'), ROOT/'examples/composition-consumer/tests/choice_status.rs', ROOT/'examples/composition-consumer/Cargo.toml', Path(__file__)]
    data.update({p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in paths})
    return data
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    if args.output.exists():parser.error('use a fresh evidence path')
    before=inputs();command=['cargo','test','--manifest-path','examples/composition-consumer/Cargo.toml','--features','token-audit','--test','choice_status','--locked']
    with tempfile.TemporaryDirectory(prefix='tekesui-select-async-') as folder:
        folder=Path(folder)
        subprocess.run(['python3','scripts/audit-choice-selection-tokens.py','--output',str(folder/'choice.json')],cwd=ROOT,check=True)
        data=json.loads((folder/'choice.json').read_text());env=os.environ.copy();env['TEKESUI_SELECT_ASYNC_AUDIT_OUT']=str(folder/'async.json')
        subprocess.run(command,cwd=ROOT,env=env,check=True);extra=json.loads((folder/'async.json').read_text())
    assert len(extra['cases'])==24 and all(c['state_asserted'] is True for c in extra['cases'])
    assert {c['choice_state'] for c in extra['cases']}=={'loading','error'}
    if before!=inputs():raise RuntimeError('source changed during probes')
    for case in extra['cases']:case['context']='caller background refresh with cached options; not input validation'
    data['cases']+=extra['cases'];assert len(data['cases'])==228
    data.update(inputs_sha256=before,probe_command=command,public_api_test='examples/composition-consumer/tests/choice_status.rs',scope='Select9/ComboBox8 production View states plus independent Select background refresh loading/failure: ContentHost/Stack/Progress/Banner, cached choice/focus/candidate preservation and separate validation semantics; four themes/three brands; CoreText with no-op glyph painter',excluded=['ComboBox/CommandPalette async composition','complete state combinations/dependency extraction','native AX/VoiceOver/physical IME','native live async interaction and complete motion'])
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x') as f:json.dump(data,f,indent=2);f.write('\n')
    print('Wrote228 exact bounded choice/Select-refresh cases; complete=False')
if __name__=='__main__':main()
