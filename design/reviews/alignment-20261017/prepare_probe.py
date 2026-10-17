from pathlib import Path
import shutil,hashlib,json
root=Path.cwd();assets=Path(__file__).resolve().parent;p=root/'target/alignment-review-20261017';p.mkdir(parents=True,exist_ok=True);d=p/'probe-source';d.mkdir(exist_ok=False)
# Archived verification sources can bootstrap a fresh target tree without relying on old build output.
if assets != p:
 for name in ['probe_state.rs','probe_main.rs','interaction_probe.rs','interaction_main.rs']:
  target=p/name
  if target.exists():assert target.read_bytes()==(assets/name).read_bytes(), 'Preserve differing existing probe helper: '+str(target)
  else:shutil.copy2(assets/name,target)

for n in ['Cargo.toml','Cargo.lock']:shutil.copy2(root/n,d/n)
shutil.copytree(root/'crates',d/'crates',ignore=shutil.ignore_patterns('target'))
for n in ['design','fixtures','scripts']:(d/n).symlink_to(root/n,target_is_directory=True)
g=d/'crates/tekes-ui-gallery';w=g/'src/gallery/workbench.rs';w.write_text(w.read_text().replace('mod extended;','mod extended;\npub mod probe;\npub mod interaction_probe;',1));(g/'src/gallery/workbench/probe.rs').write_bytes((p/'probe_state.rs').read_bytes());(g/'src/gallery/workbench/interaction_probe.rs').write_bytes((p/'interaction_probe.rs').read_bytes());(g/'src/bin').mkdir(exist_ok=True);(g/'src/bin/offscreen_alignment_review.rs').write_bytes((p/'probe_main.rs').read_bytes());(g/'src/bin/offscreen_interaction_review.rs').write_bytes((p/'interaction_main.rs').read_bytes())
e=g/'src/gallery/workbench/extended.rs';e.write_text(e.read_text().replace('    collections_views: CollectionsViews,','    pub(super) collections_views: CollectionsViews,').replace('    feedback_views: FeedbackViews,','    pub(super) feedback_views: FeedbackViews,'));a=g/'src/gallery/workbench/extended/collections_adapter.rs';a.write_text(a.read_text().replace('pub(super) struct CollectionsViews','pub(crate) struct CollectionsViews'))
a=g/'src/gallery/workbench/extended/feedback_adapter.rs';a.write_text(a.read_text().replace('pub(super) struct FeedbackViews','pub(crate) struct FeedbackViews'))
with (g/'Cargo.toml').open('a') as f:f.write('\n[[bin]]\nname = "offscreen-alignment-review"\npath = "src/bin/offscreen_alignment_review.rs"\n\n[[bin]]\nname = "offscreen-interaction-review"\npath = "src/bin/offscreen_interaction_review.rs"\n')
(p/'probe-source.json').write_text(json.dumps({str(f.relative_to(d)):hashlib.sha256(f.read_bytes()).hexdigest() for f in (d/'crates').rglob('*') if f.is_file()},indent=2)+'\n')
