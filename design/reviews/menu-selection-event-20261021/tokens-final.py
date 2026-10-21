import hashlib,json,subprocess,sys,re,os,tempfile,importlib.util
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];OUT=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'scripts'));import tekesui_build as b
spec=importlib.util.spec_from_file_location('nav',ROOT/'scripts/audit-navigation-tokens.py');nav=importlib.util.module_from_spec(spec);spec.loader.exec_module(nav)
def direct(name,old,target,env):
 fields={'field':('TEKESUI_FIELD_AUDIT_OUT','field_contract',48),'number':('TEKESUI_NUMBER_STATE_AUDIT_OUT','number_field_contract',60)}
 key,test,count=fields[name];paths=set(old['inputs_sha256'])|set(nav.inputs())|{Path(__file__).relative_to(ROOT).as_posix()}
 def hashes():return {p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in paths}
 before=hashes();cmd=['cargo','test','-p','tekes-ui-controls','--features','token-audit','--test',test,'--locked']
 with tempfile.TemporaryDirectory(prefix='tekesui-direct-')as folder:
  raw=Path(folder)/'raw.json';subprocess.run(cmd,cwd=ROOT,env=dict(env,**{key:str(raw)}),check=True);data=json.loads(raw.read_text())
 assert len(data['cases'])==count and data['complete']is False and hashes()==before
 data.update(format=old['format'],inputs_sha256=before,public_api_test=old['public_api_test'],probe_command=cmd)
 data.setdefault('scope',old['scope']);target.parent.mkdir(parents=True,exist_ok=True)
 with target.open('x')as stream:json.dump(data,stream,indent=2);stream.write('\n')
coverage=json.loads((ROOT/'design/gallery/component-coverage.json').read_text());old=sorted({c['runtime_token_observation'] for c in coverage['components'] if c.get('runtime_token_observation')});bindings={};records=[];checks=[]
with b.build_lock():
 b.prepare_cache();env=b.environment();env['CARGO_NET_OFFLINE']='true'
 env['TEKESUI_ITEM_REVEAL_GPU_OUT']=str(OUT/'gpu-final-v2')
 for path in old:
  data=json.loads((ROOT/path).read_text());changed=[p for p,h in data['inputs_sha256'].items() if hashlib.sha256((ROOT/p).read_bytes()).hexdigest()!=h];name=Path(path).stem
  if name=='canonical-observation':name='passive-feedback'
  current_target=OUT/'token-probes-final2'/f'{name}.json'
  already=False
  if current_target.exists():
   current=json.loads(current_target.read_text());already=all(hashlib.sha256((ROOT/p).read_bytes()).hexdigest()==h for p,h in current['inputs_sha256'].items())
   assert already,'existing intermediate receipt is stale; preserve it and use another path'
   bindings[path]=current_target.relative_to(ROOT).as_posix()
  if changed and not already:
   script=ROOT/f'scripts/audit-{name}-tokens.py';assert script.is_file()or name in ['field','number'],name
   target=OUT/'token-probes-final2'/f'{name}.json';log=OUT/f'final2-probe-{name}.log'
   if name in ['field','number']:
    with log.open('w')as stream:
     import contextlib
     with contextlib.redirect_stdout(stream),contextlib.redirect_stderr(stream):direct(name,data,target,env)
    class Done:returncode=0
    r=Done()
   else:
    with log.open('w') as stream:r=subprocess.run(['python3',str(script),'--output',str(target)],cwd=ROOT,env=env,stdout=stream,stderr=subprocess.STDOUT)
   checks.append({'name':name,'exit':r.returncode});(OUT/'token-checks-final2.json').write_text(json.dumps(checks,indent=2)+'\n');print(name,r.returncode,flush=True)
   if r.returncode:sys.exit(r.returncode)
   bindings[path]=target.relative_to(ROOT).as_posix()
  elif not already:bindings[path]=path
  records.append({'old':path,'current':bindings[path],'changed_inputs':changed,'action':'canonical actual rerun' if changed else 'exact input hashes revalidated'})
 (OUT/'observation-bindings-final2.json').write_text(json.dumps(records,indent=2)+'\n')
 for c in coverage['components']:
  if c.get('runtime_token_observation'):c['runtime_token_observation']=bindings[c['runtime_token_observation']]
 p=ROOT/'design/gallery/component-coverage.json';q=OUT/'before/design/gallery/component-coverage.json';q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(p.read_bytes());p.write_text(json.dumps(coverage,ensure_ascii=False,indent=2)+'\n')
