#!/usr/bin/env python3
"""Bounded original CommandPalette and independent caller async observations."""
import argparse, hashlib, importlib.util, json, os, subprocess, tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('command',ROOT/'scripts/audit-command-results-tokens.py')
command=importlib.util.module_from_spec(spec);spec.loader.exec_module(command)
def inputs():
    data=command.inputs()
    for p in [*ROOT.glob('examples/composition-consumer/src/**/*.rs'),ROOT/'examples/composition-consumer/tests/command_status.rs',Path(__file__)]:
        data[p.relative_to(ROOT).as_posix()]=hashlib.sha256(p.read_bytes()).hexdigest()
    return data
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    if args.output.exists():parser.error('use a fresh evidence path')
    before=inputs();probe=['cargo','test','--manifest-path','examples/composition-consumer/Cargo.toml','--features','token-audit','--test','command_status','--locked']
    with tempfile.TemporaryDirectory(prefix='tekesui-command-async-') as folder:
        folder=Path(folder);subprocess.run(['python3','scripts/audit-command-results-tokens.py','--output',str(folder/'base.json')],cwd=ROOT,check=True)
        data=json.loads((folder/'base.json').read_text());env=os.environ.copy();env['TEKESUI_COMMAND_ASYNC_AUDIT_OUT']=str(folder/'async.json')
        subprocess.run(probe,cwd=ROOT,env=env,check=True);extra=json.loads((folder/'async.json').read_text())
    assert len(extra['cases'])==48 and all(c['state_asserted'] is True for c in extra['cases'])
    if before!=inputs():raise RuntimeError('source changed during probes')
    data['cases']+=extra['cases'];assert len(data['cases'])==168
    data.update(inputs_sha256=before,probe_command=probe,public_api_test='examples/composition-consumer/tests/command_status.rs',scope='Original public CommandPaletteView trigger/results plus retained and initial-empty async caller; original TextFieldContent/Status/Feedback; four themes/three brands; existing host IME/capture routing; CoreText/no-op glyph painter',excluded=['complete state combinations/dependency extraction','native live async transitions/physicalIME/controlsAX/VoiceOver','complete motion'])
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x') as f:json.dump(data,f,indent=2);f.write('\n')
    print('Wrote168 bounded command cases; complete=False')
if __name__=='__main__':main()
