#!/usr/bin/env python3
"""Execute bounded independent SliderView state/geometry/event observations."""
import argparse,hashlib,importlib.util,json,os,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('selection',ROOT/'scripts/audit-selection-tokens.py')
selection=importlib.util.module_from_spec(spec);spec.loader.exec_module(selection)
def inputs():
 data=selection.inputs()
 for f in ['examples/composition-consumer/tests/slider_states.rs','examples/composition-consumer/Cargo.toml','crates/tekes-ui-controls/tests/selection_availability.rs','scripts/audit-slider-tokens.py']:
  data[f]=hashlib.sha256((ROOT/f).read_bytes()).hexdigest()
 return data
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
 if args.output.exists():parser.error('use a fresh evidence path')
 before=inputs();probe=['cargo','test','--manifest-path','examples/composition-consumer/Cargo.toml','--features','token-audit','--test','slider_states','--locked']
 with tempfile.TemporaryDirectory(prefix='tekesui-slider-') as folder:
  path=Path(folder)/'raw.json';env=os.environ.copy();env['TEKESUI_SLIDER_AUDIT_OUT']=str(path);subprocess.run(probe,cwd=ROOT,env=env,check=True);data=json.loads(path.read_text())
 assert len(data['cases'])==216 and all(c['state_asserted'] is True for c in data['cases'])
 if before!=inputs():raise RuntimeError('source changed during probes')
 data.update(inputs_sha256=before,probe_command=probe,public_api_test='examples/composition-consumer/tests/slider_states.rs',scope='Original public SliderView six states; real pointer/key/drag/release, values0/50/100, four themes/three brands, Scene primitive geometry/color; no native inference',excluded=['complete state combinations/dependency extraction','native AX/VoiceOver','performance','complete native interaction acceptance'])
 args.output.parent.mkdir(parents=True,exist_ok=True)
 with args.output.open('x') as f:json.dump(data,f,indent=2);f.write('\n')
 print('Wrote216 bounded slider cases; complete=False')
if __name__=='__main__':main()
