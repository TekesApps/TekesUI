#!/usr/bin/env python3
"""Bounded retained ComboBox query, IME and validation composition observations."""
import argparse, hashlib, importlib.util, json, os, subprocess, tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('select',ROOT/'scripts/audit-select-async-tokens.py')
select=importlib.util.module_from_spec(spec);spec.loader.exec_module(select)
def inputs():
    data=select.inputs()
    for path in ['examples/composition-consumer/tests/combo_status.rs','scripts/audit-combo-async-tokens.py']:
        data[path]=hashlib.sha256((ROOT/path).read_bytes()).hexdigest()
    return data
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    if args.output.exists():parser.error('use a fresh evidence path')
    before=inputs();command=['cargo','test','--manifest-path','examples/composition-consumer/Cargo.toml','--features','token-audit','--test','combo_status','--locked']
    with tempfile.TemporaryDirectory(prefix='tekesui-combo-async-') as folder:
        folder=Path(folder);subprocess.run(['python3','scripts/audit-select-async-tokens.py','--output',str(folder/'base.json')],cwd=ROOT,check=True)
        data=json.loads((folder/'base.json').read_text());env=os.environ.copy();env['TEKESUI_COMBO_ASYNC_AUDIT_OUT']=str(folder/'combo.json')
        subprocess.run(command,cwd=ROOT,env=env,check=True);extra=json.loads((folder/'combo.json').read_text())
    assert len(extra['cases'])==36 and all(c['state_asserted'] is True for c in extra['cases'])
    if before!=inputs():raise RuntimeError('source changed during probes')
    data['cases']+=extra['cases'];assert len(data['cases'])==264
    data.update(inputs_sha256=before,probe_command=command,public_api_test='examples/composition-consumer/tests/combo_status.rs',scope='Existing production choice states plus independent Select refresh and ComboBox retained TextFieldContent/ComboBoxView refresh/validation; four themes/three brands; ContentHost IME-before-overlay arbitration and marked response rejection; CoreText with no-op glyph painter',excluded=['CommandPalette async composition','complete state combinations/dependency extraction','native live async transitions/physicalIME/AX/VoiceOver','complete motion'])
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x') as f:json.dump(data,f,indent=2);f.write('\n')
    print('Wrote264 bounded choice and retained editable refresh cases; complete=False')
if __name__=='__main__':main()
