from pathlib import Path
import hashlib,json,shutil,tarfile,sys
ROOT=Path.cwd(); P=ROOT/'target/selection-components-20261017';kind=sys.argv[1];D=P/'sources'/kind
if D.exists():shutil.rmtree(D)
D.mkdir(parents=True)
for n in ['Cargo.toml','Cargo.lock']:shutil.copy2(ROOT/n,D/n)
shutil.copytree(ROOT/'crates',D/'crates')
for n in ['design','fixtures','scripts']:(D/n).symlink_to(ROOT/n,target_is_directory=True)
before=json.loads((P/'before-source.json').read_text())
if kind=='baseline':
 with tarfile.open(P/'baseline-sources.tar.gz') as t:
  for name in t.getnames():
   if name.startswith('crates/'):
    content=t.extractfile(name).read();assert hashlib.sha256(content).hexdigest()==before[name];(D/name).write_bytes(content)
wb=D/'crates/tekes-ui-gallery/src/gallery/workbench.rs';wb.write_text(wb.read_text().replace('mod extended;','mod extended;\npub mod probe;',1))
shutil.copy2(P/'probe_module.rs',wb.parent/'workbench/probe.rs')
binpath=D/'crates/tekes-ui-gallery/src/bin/offscreen_selection_probe.rs';binpath.parent.mkdir(exist_ok=True);shutil.copy2(P/'probe_main.rs',binpath)
with (D/'crates/tekes-ui-gallery/Cargo.toml').open('a') as f:f.write('\n[[bin]]\nname="offscreen-selection-probe"\npath="src/bin/offscreen_selection_probe.rs"\n')
receipt={str(f.relative_to(D)):hashlib.sha256(f.read_bytes()).hexdigest() for f in D.joinpath('crates').rglob('*') if f.is_file()}
(P/(kind+'-probe-source.json')).write_text(json.dumps(receipt,indent=2)+'\n');print('Frozen',kind,len(receipt),'crate files')
