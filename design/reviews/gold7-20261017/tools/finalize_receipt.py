from pathlib import Path
import hashlib,json,shutil,tarfile,re,subprocess
p=Path('target/gold7-review-20261017');out=Path('design/reviews/gold7-20261017');out.mkdir(parents=True,exist_ok=True)
def digest(f):return hashlib.sha256(f.read_bytes()).hexdigest()
def save(f,v):f.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
files=set(json.loads((p/'final-source.json').read_text()))
for base in ['crates','design/source','design/brand','design/gallery','design/behavior','docs/design','scripts']:
 for f in Path(base).rglob('*'):
  if f.is_file() and f.suffix in ['.rs','.wgsl','.toml','.json','.md','.py']:files.add(str(f))
files.update(['Cargo.toml','Cargo.lock','design/tokens.json','design/system.json','design/system.schema.json','design/integration.json','design/proposal-mapping.json','docs/design/gold7.md'])
manifest={f:digest(Path(f)) for f in sorted(files) if Path(f).is_file() and 'target' not in Path(f).parts}
save(p/'final-source.json',manifest);save(out/'source-manifest.json',manifest)
with tarfile.open(out/'source-snapshot.tar.gz','w:gz') as archive:
 for f in manifest:archive.add(f,arcname=f,recursive=False)
copy_names=['checks-final.json','tests-final.log','clippy-final.log','build-final.log','fmt-final.log','pure-rust-final.log','design-system-final.log','diff-final.log','reference-colors-final.json','gold7-mechanical-final.json','gold7-checker-tests-final.log','pixel-checks.json','checkbox-pixels-final.json','package-binary-comparison.json','package-final.log','captures-final.log','interactions-final.log']
for name in copy_names:shutil.copy2(p/name,out/name)
shutil.copy2(p/'captures-final/manifest.json',out/'component-cases.json');shutil.copy2(p/'interactions-final/manifest.json',out/'interaction-cases.json')
for name in ['run_checks.py','refresh_probe.py','probe_main.rs','probe_state.rs','interaction_main.rs','interaction_probe.rs','analyze_pixels.py','check_checkbox_pixels.py','compare_package.py','finalize_receipt.py']:
 d=out/'tools'/name;d.parent.mkdir(exist_ok=True);shutil.copy2(p/name,d)
selected=['component-checkbox-large-narrow-light-2x','component-checkbox-large-narrow-light-2x-checked','component-checkbox-large-narrow-dark-2x-checked','component-status-badge-large-narrow-light-2x','component-status-badge-large-narrow-dark-2x','component-markdown-large-narrow-light-1x','component-code-block-large-narrow-light-1x','component-diff-view-large-narrow-light-1x','brand-matrix-Theme { appearance: Light, contrast: Standard }-2x-direction-0']
# Filenames use the Theme debug representation emitted by the actual harness.
actual={f.stem:f for f in (p/'captures-final').glob('*.png')}
selected=[n for n in selected if n in actual]
selected += [f.stem for f in (p/'captures-final').glob('brand-matrix-*2x-direction-0.png')][:1]
for name in selected:
 d=out/'images'/(name+'.png');d.parent.mkdir(exist_ok=True);shutil.copy2(actual[name],d)
for k in range(1,4):shutil.copy2(p/f'components-large-{k}.png',out/'images'/f'components-large-{k}.png')
artifacts={str(f.relative_to(p)):digest(f) for sub in ['captures-final','interactions-final'] for f in sorted((p/sub).iterdir()) if f.is_file()}
save(out/'all-render-artifact-hashes.json',artifacts)
checks=json.loads((p/'checks-final.json').read_text());assert all(c['exit']==0 for c in checks)
r=json.loads((p/'interactions-final/manifest.json').read_text());assert len(r)==63 and all(c['passed'] for c in r)
report=json.loads((p/'reference-colors-final.json').read_text());assert report['static_result']=='pass'
counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',(p/'tests-final.log').read_text());totals=[sum(int(v[i]) for v in counts) for i in range(3)];assert totals==[556,0,1]
receipt={'format':'tekesui-gold7-refinement-review/1','date':'2026-10-17','scope':'warm neutral roles, restrained brand candidates, existing component geometry/ink/clip fixes; no new components; performance paused','source_state':'uncommitted worktree; no automatic commit/push/publication','base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'source_manifest':'source-manifest.json','source_snapshot':'source-snapshot.tar.gz','checks':checks,'rust_tests':{'passed':556,'failed':0,'ignored':1,'ignored_reason':'full icon catalogue requires explicit local TEKESUI_ICON_DIR and TEKESUI_ICONS_REVIEW_OUT'},'color_checks':{'brand_profiles':13,'themes':4,'declared_pairs':len(report['contrast_checks']),'brand_independent_color_roles':report['brand_independent_role_count'],'result':'pass','scope':'declared pairs only, not arbitrary nesting or physical displays'},'render_checks':{'component_count':45,'main_cases':743,'png_total':859,'interaction_cases':63,'interaction_passed':63,'checkbox_pointer_toggle_pixel_cases':16,'full_artifacts_local':str(p.resolve()),'full_artifact_hashes':'all-render-artifact-hashes.json','retained_samples':'images','tools':'tools','method':'target-only clone of actual Gallery; visibility adapters for inspection, real components, TextEngine, GPU renderer and actual event dispatch; all production Rust files refreshed before rendering; no screenshots used as component implementations'},'native_observations':{'earlier_build_this_round':['Checkbox click and Space toggled without label disappearing','hue draft did not change applied brand','candidate dialog blocked background scrolling and Escape restored invoker focus'],'final_packaged_build':['StatusBadge actual window showed readable light labels on four status surfaces','Gallery relaunched, Theme & brand matrix opened in Light with reference brand and no candidate applied'],'native_screenshot_file_saved':False,'display_timing_measured':False},'not_completed':['Gold7 G4 real component StateTransition integration','focus-ring and elevation design comparison','draft recovery and explicit return-to-latest gaps','media zoom/pan interactions','native VoiceOver and physical IME acceptance','physical external-display edge appearance across scales','CPU, dropped frames and sustained built-in 120Hz/8.33ms acceptance'],'gold7_result':'partial refinement only; contract/code/experience remain separate; no blanket pass','package_comparison':'package-binary-comparison.json'}
save(out/'receipt.json',receipt)
save(out/'artifact-manifest.json',{str(f.relative_to(out)):digest(f) for f in sorted(out.rglob('*')) if f.is_file() and f.name!='artifact-manifest.json'})
print(json.dumps({'source_files':len(manifest),'retained_images':len(list((out/'images').glob('*.png'))),'full_artifact_files':len(artifacts),'receipt':str(out/'receipt.json')}))
