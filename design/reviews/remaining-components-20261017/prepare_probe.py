from pathlib import Path
import shutil,json,hashlib
root=Path.cwd();p=root/'target/remaining-components-20261017';dest=p/'probe-source'
assert not dest.exists()
dest.mkdir();shutil.copy2(root/'Cargo.toml',dest/'Cargo.toml');shutil.copy2(root/'Cargo.lock',dest/'Cargo.lock');shutil.copytree(root/'crates',dest/'crates')
for folder in ['design','fixtures','scripts']:(dest/folder).symlink_to(root/folder,target_is_directory=True)
g=dest/'crates/tekes-ui-gallery';w=g/'src/gallery/workbench.rs';w.write_text(w.read_text().replace('mod extended;','mod extended;\npub mod probe;',1));(g/'src/gallery/workbench/probe.rs').write_bytes((p/'probe_module.rs').read_bytes());(g/'src/bin').mkdir(exist_ok=True);(g/'src/bin/offscreen_remaining_probe.rs').write_bytes((p/'probe_main.rs').read_bytes())
with (g/'Cargo.toml').open('a') as f:f.write('\n[[bin]]\nname = "offscreen-remaining-probe"\npath = "src/bin/offscreen_remaining_probe.rs"\n')
(p/'probe-source.json').write_text(json.dumps({str(f.relative_to(dest)):hashlib.sha256(f.read_bytes()).hexdigest() for f in (dest/'crates').rglob('*') if f.is_file()},indent=2)+'\n')

e=g/"src/gallery/workbench/extended.rs";e.write_text(e.read_text().replace("    feedback_views: FeedbackViews,", "    pub(super) feedback_views: FeedbackViews,"))
f=g/"src/gallery/workbench/extended/feedback_adapter.rs";f.write_text(f.read_text().replace("pub(super) struct FeedbackViews", "pub(crate) struct FeedbackViews"))
