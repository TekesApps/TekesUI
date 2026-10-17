from pathlib import Path
import hashlib,json,shutil
p=Path('target/remaining-components-20261017');d=p/'probe-source'
for src in Path('crates').rglob('*.rs'):
 target=d/src;target.parent.mkdir(parents=True,exist_ok=True);s=src.read_text()
 if str(src).endswith('/gallery/workbench.rs'):s=s.replace('mod extended;','mod extended;\npub mod probe;',1)
 if str(src).endswith('/workbench/extended.rs'):s=s.replace('    feedback_views: FeedbackViews,','    pub(super) feedback_views: FeedbackViews,')
 if str(src).endswith('/extended/feedback_adapter.rs'):s=s.replace('pub(super) struct FeedbackViews','pub(crate) struct FeedbackViews')
 target.write_text(s)
(p/'probe-source-final.json').write_text(json.dumps({str(f.relative_to(d)):hashlib.sha256(f.read_bytes()).hexdigest() for f in (d/'crates').rglob('*') if f.is_file()},indent=2)+'\n')
