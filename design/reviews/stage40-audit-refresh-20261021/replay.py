"""Rerun current public probes in one shared cache; never update old receipts."""
import sys, subprocess,json,hashlib,os
from pathlib import Path
root=Path(__file__).resolve().parents[3];out=Path(__file__).resolve().parent
sys.path.insert(0,str(root/'scripts'))
from tekesui_build import build_lock,environment
probe_dir=out/(sys.argv[1] if len(sys.argv)>1 else 'probes')
if probe_dir.exists():raise SystemExit('Fresh output directory required')
scripts=['action','collection-focus','combo-async','command-async','content-status','grid-layout','input','layout-surface','menu-content','passive-feedback','popover-content','selection','separator','slider-composition','tooltip']
with build_lock():
 env=environment();env['RUSTUP_TOOLCHAIN']='1.95.0'
 for name in scripts:
  with (out/(probe_dir.name+'-'+name+'.log')).open('w') as log:
   p=subprocess.run([sys.executable,str(root/'scripts'/('audit-'+name+'-tokens.py')),'--output',str(probe_dir/(name+'.json'))],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=120)
  print(name,p.returncode,flush=True)
  if p.returncode:raise SystemExit(p.returncode)
 # These existing tests expose their actual probe output directly. Bind the
 # executed test and current source, preserving complete=False and exclusions.
 for name,test,variable in [('field','field_contract','TEKESUI_FIELD_AUDIT_OUT'),('number','number_field_contract','TEKESUI_NUMBER_STATE_AUDIT_OUT')]:
  old=json.loads((root/'design/reviews/menu-declarations-20261021/token-probes-scroll-final'/(name+'.json')).read_text())
  import importlib.util
  spec=importlib.util.spec_from_file_location('navigation',root/'scripts/audit-navigation-tokens.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
  before={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in set(old['inputs_sha256'])|set(module.inputs())}
  raw=probe_dir/(name+'-raw.json');command=['cargo','test','-p','tekes-ui-controls','--features','token-audit','--test',test,'--locked']
  with (out/(probe_dir.name+'-'+name+'.log')).open('w') as log:
   subprocess.run(command,cwd=root,env=dict(env,**{variable:str(raw)}),stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
  data=json.loads(raw.read_text());assert data['complete'] is False
  assert before=={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in before},'source drift'
  assert len(data['cases'])==len(old['cases']),'unexpected probe coverage change'
  for key in ['format','scope','excluded']:
   if key not in data and key in old:data[key]=old[key]
  data.update(inputs_sha256=before,public_api_test=f'crates/tekes-ui-controls/tests/{test}.rs',probe_command=command,native_acceptance='separate/unverified')
  (probe_dir/(name+'.json')).write_text(json.dumps(data,indent=2)+'\n');raw.unlink()
  print(name,0,flush=True)

 # New shared ToolbarControl is observed separately from the legacy View.
 with (out/(probe_dir.name+'-toolbar.log')).open('w') as log:
  import importlib.util
  spec=importlib.util.spec_from_file_location('navigation',root/'scripts/audit-navigation-tokens.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
  before=module.inputs();test='crates/tekes-ui-controls/tests/toolbar_contract.rs';before[test]=hashlib.sha256((root/test).read_bytes()).hexdigest()
  raw=probe_dir/'toolbar-raw.json';command=['cargo','test','-p','tekes-ui-controls','--features','token-audit','--test','toolbar_contract','--locked']
  subprocess.run(command,cwd=root,env=dict(env,TEKESUI_TOOLBAR_AUDIT_OUT=str(raw)),stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
  assert before=={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in before},'source drift'
  data=json.loads(raw.read_text());assert len(data['cases'])==72 and data['complete'] is False
  data.update(inputs_sha256=before,probe_command=command,public_api_test=test,native_acceptance='separate/unverified')
  (probe_dir/'toolbar.json').write_text(json.dumps(data,indent=2)+'\n');raw.unlink();print('toolbar',0,flush=True)
