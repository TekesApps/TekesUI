#!/usr/bin/env python3
"""Bounded original SliderView plus public SliderContent composition observations."""
import argparse,hashlib,importlib.util,json,os,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('slider',ROOT/'scripts/audit-slider-tokens.py')
slider=importlib.util.module_from_spec(spec);spec.loader.exec_module(slider)
def inputs():
 data=slider.inputs()
 for p in [*ROOT.glob('examples/composition-consumer/src/**/*.rs'),ROOT/'examples/composition-consumer/tests/slider_content.rs',Path(__file__)]:data[p.relative_to(ROOT).as_posix()]=hashlib.sha256(p.read_bytes()).hexdigest()
 return data
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
 if args.output.exists():parser.error('use a fresh evidence path')
 before=inputs();probe=['cargo','test','--manifest-path','examples/composition-consumer/Cargo.toml','--features','token-audit','--test','slider_content','--locked']
 with tempfile.TemporaryDirectory(prefix='tekesui-slider-composition-') as folder:
  folder=Path(folder);subprocess.run(['python3','scripts/audit-slider-tokens.py','--output',str(folder/'base.json')],cwd=ROOT,check=True);data=json.loads((folder/'base.json').read_text());env=os.environ.copy();env['TEKESUI_SLIDER_COMPOSITION_AUDIT_OUT']=str(folder/'composition.json');subprocess.run(probe,cwd=ROOT,env=env,check=True);extra=json.loads((folder/'composition.json').read_text())
 assert len(extra['cases'])==48 and all(c['state_asserted'] is True for c in extra['cases'])
 if before!=inputs():raise RuntimeError('source changed during probes')
 data['cases']+=extra['cases'];assert len(data['cases'])==264
 data.update(inputs_sha256=before,probe_command=probe,public_api_test='examples/composition-consumer/tests/slider_content.rs',scope='Original SliderView six states plus public SliderContent in independent ContentHost/Stack with TextFieldContent sibling; real ownership/range/actions; four themes/three brands; CoreText/no-op glyph painter; native AX separate',excluded=['native AX role/actions/VoiceOver operation','complete state combinations/dependency extraction','physicalIME/live async/full motion','performance'])
 args.output.parent.mkdir(parents=True,exist_ok=True)
 with args.output.open('x') as f:json.dump(data,f,indent=2);f.write('\n')
 print('Wrote264 bounded slider cases; complete=False')
if __name__=='__main__':main()
