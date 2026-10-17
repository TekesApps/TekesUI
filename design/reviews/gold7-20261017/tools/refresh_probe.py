from pathlib import Path
import hashlib,json
p=Path('target/gold7-review-20261017');d=p/'probe-source'
for src in Path('crates').rglob('*.rs'):
 if 'target' in src.parts:continue
 target=d/src;s=src.read_text()
 if str(src).endswith('/gallery/workbench.rs'):s=s.replace('mod extended;','mod extended;\npub mod probe;\npub mod interaction_probe;',1)
 if str(src).endswith('/workbench/extended.rs'):s=s.replace('    collections_views: CollectionsViews,','    pub(super) collections_views: CollectionsViews,').replace('    feedback_views: FeedbackViews,','    pub(super) feedback_views: FeedbackViews,')
 if str(src).endswith('/extended/collections_adapter.rs'):s=s.replace('pub(super) struct CollectionsViews','pub(crate) struct CollectionsViews')
 if str(src).endswith('/extended/feedback_adapter.rs'):s=s.replace('pub(super) struct FeedbackViews','pub(crate) struct FeedbackViews')
 if not target.exists() or target.read_text()!=s:target.parent.mkdir(parents=True,exist_ok=True);target.write_text(s)
(p/'probe-source.json').write_text(json.dumps({str(f.relative_to(d)):hashlib.sha256(f.read_bytes()).hexdigest() for f in (d/'crates').rglob('*') if f.is_file()},indent=2)+'\n')
