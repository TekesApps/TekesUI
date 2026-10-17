from pathlib import Path
import json,hashlib,tarfile,shutil,re,subprocess
p=Path('target/gold7-alignment-pass2'); out=Path('design/reviews/gold7-20261017/alignment-pass2')
out.mkdir(parents=True,exist_ok=True)
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
def save(f,data):f.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n')
checks=json.loads((p/'checks-final.json').read_text()); assert all(x['exit']==0 for x in checks)
counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',(p/'tests-final.log').read_text())
totals=[sum(int(v[i]) for v in counts) for i in range(3)];assert totals[1]==0 and totals[0]>650
comparison=json.loads((p/'package-binary-comparison.json').read_text());assert comparison['passed']
native=json.loads((p/'native.json').read_text());assert native['packaged_build_restarted']
ids=['focus-paint-space-and-reveal','semantic-gaussian-elevation','production-control-state-transitions','post-build-frame-scheduling','inactive-control-motion-cancellation']
worklist=Path('design/behavior/gold7-alignment.json');d=json.loads(worklist.read_text())
labels=['Real ancestor clips, token focus gutter, full owned control frames, keyboard scroll reveal and Menu/Dialog spacing','JSON Menu/Popover/Dialog elevation reaches actual Gaussian GPU pixels; panels and increased-contrast themes stay shadow-free','Production Button hover/press, Switch thumb and Disclosure body use JSON bindings, interruption and Reduced Motion','Host starts animation requests discovered by the first scene build and stops when completed','Deactivation and late inactive MouseUp clear stale hover/capture/presentation channels']
for key,label in zip(ids,labels):
 if not any(r['id']==key for r in d['repairs']):d['repairs'].append({'id':key,'state':'verified-code','description':label,'evidence':{'$ref':'../reviews/gold7-20261017/alignment-pass2/receipt.json'}})
for row in d['requirements']:
 selected=[]
 if row['id'].startswith('G2/same-color'):selected=[ids[1]]
 if row['id'].startswith('G3/real-scale') or row['id'].startswith('G3/hairline'):selected=[ids[0]]
 if row['id'].startswith('G4/'):selected=[ids[2],ids[3],ids[4]]
 row['repairs']=list(dict.fromkeys(row['repairs']+selected))
save(worklist,d)
# Preserve the previous source snapshot and delta; only this pass is new.
previous=json.loads(Path('design/reviews/gold7-20261017/alignment-pass1/source-manifest.json').read_text())
files=set(previous)
for root in ['crates','design/source','design/brand','design/gallery','design/behavior','docs/design','scripts']:
 for f in Path(root).rglob('*'):
  if f.is_file() and f.suffix in ['.rs','.wgsl','.toml','.json','.md','.py'] and 'target' not in f.parts:files.add(str(f))
files.update(['Cargo.toml','Cargo.lock','design/tokens.json','design/system.json','design/integration.json','design/proposal-mapping.json'])
manifest={f:sha(Path(f)) for f in sorted(files) if Path(f).is_file()}
save(out/'source-manifest.json',manifest)
changed=[f for f in manifest if previous.get(f)!=manifest[f]]
with tarfile.open(out/'sources-delta.tar.gz','w:gz') as archive:
 for f in changed:archive.add(f,arcname=f,recursive=False)
for f in p.iterdir():
 if f.is_file() and f.suffix in ['.log','.json','.py']:shutil.copy2(f,out/f.name)
for folder in ['effects','focus']:
 if (p/folder).exists():shutil.copytree(p/folder,out/folder,dirs_exist_ok=True)
receipt={'format':'tekesui-gold7-alignment-pass/1','date':'2026-10-17','source_state':'uncommitted worktree; no commit/push/publication','base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'source_manifest':'source-manifest.json','source_recovery':{'archives_in_order':['../source-snapshot.tar.gz','../alignment-pass1/sources-delta.tar.gz','sources-delta.tar.gz'],'verify':'source-manifest.json'},'requirements_all_complete':False,'verified_code_repairs':ids,'checks':checks,'rust_test_totals':dict(zip(['passed','failed','ignored'],totals)),'gpu':{'production_overlay_images':24,'actual_control_motion_frames':4,'focus_images':8,'themes':4,'scales':[1,2],'path':'effects','no_adapter_skip':False,'scope':'Actual production Menu/Popover/Dialog views and Button/Switch/Disclosure use CoreText and the real renderer. No performance measurements.'},'native_observation':native,'package_identity':'package-binary-comparison.json','limitations':['Remaining offscreen roving-focus traversal is not a blanket all-components acceptance','Overlay enter/exit transitions and full state coverage remain on the Gold7 list','Physical display matrix, VoiceOver and physical IME not accepted by these GPU/native samples','CPU, dropped frames and sustained 120Hz/8.33ms deliberately not measured']}
save(out/'receipt.json',receipt)
save(out/'artifact-manifest.json',{str(f.relative_to(out)):sha(f) for f in sorted(out.rglob('*')) if f.is_file() and f.name!='artifact-manifest.json'})
print(json.dumps({'source_files':len(manifest),'delta_files':len(changed),'rust_tests':totals,'receipt':str(out/'receipt.json')},ensure_ascii=False))
