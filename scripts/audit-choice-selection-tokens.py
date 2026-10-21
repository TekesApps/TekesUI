#!/usr/bin/env python3
"""Freeze exact bounded Select and caller TextSurface/ComboBox observations."""
import argparse,hashlib,importlib.util,json,os,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('navigation',ROOT/'scripts/audit-navigation-tokens.py');nav=importlib.util.module_from_spec(spec);spec.loader.exec_module(nav)
def inputs():
    data=nav.inputs()
    for relative in ['crates/tekes-ui-controls/tests/choice_selection_states.rs','scripts/audit-choice-selection-tokens.py']:
        data[relative]=hashlib.sha256((ROOT/relative).read_bytes()).hexdigest()
    return data
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    if args.output.exists():parser.error('use a fresh evidence path')
    before=inputs();command=['cargo','test','-p','tekes-ui-controls','--features','token-audit','--test','choice_selection_states','--locked']
    with tempfile.TemporaryDirectory(prefix='tekesui-choice-selection-') as folder:
        raw=Path(folder)/'reads.json';env=os.environ.copy();env['TEKESUI_CHOICE_SELECTION_AUDIT_OUT']=str(raw);subprocess.run(command,cwd=ROOT,env=env,check=True);data=json.loads(raw.read_text())
    if before!=inputs():raise RuntimeError('source changed during probe')
    assert len(data['cases'])==204 and all(c['state_asserted'] is True for c in data['cases'])
    assert len([c for c in data['cases'] if c['component']=='tui.select'])==108
    assert len([c for c in data['cases'] if c['component']=='tui.combo-box'])==96
    data.update(inputs_sha256=before,probe_command=command,public_api_test='crates/tekes-ui-controls/tests/choice_selection_states.rs',scope='Select9 exact production View states; ComboBox8 caller-owned TextSurface and original ComboBoxView states; four themes/three brands; CoreText layout and no-op glyph painter',excluded=['Select loading/caller async failure; ComboBox loading/error composition','complete state combinations/dependency extraction','native AX/VoiceOver/physical IME'])
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x') as f:json.dump(data,f,indent=2);f.write('\n')
    print('Wrote204 exact public choice cases; complete=False; native/experience separate')
if __name__=='__main__':main()
