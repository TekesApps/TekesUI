from pathlib import Path
import shutil,os,subprocess,json,hashlib
R=Path(__file__).resolve().parent
LIVE=R.parents[2];SDK=R/'sdk';PUBLIC=Path('/Users/frankqin/dev/worktrees/tekesui-icon-gold7-57')
TARGET=LIVE/'target/icon-direction-57'
def install(source):
    for p in source.glob('*.svg'):shutil.copyfile(p,SDK/'crates/tekes-ui-icons/assets'/p.name)
def run(label,args,extra=None):
    log=R/(label+'.log');assert not log.exists()
    env=dict(os.environ,**(extra or {}))
    with log.open('w')as f:r=subprocess.run(args,cwd=LIVE,env=env,stdout=f,stderr=subprocess.STDOUT)
    print(label,r.returncode,flush=True)
    if r.returncode:raise SystemExit(r.returncode)
def consumer(binary,out):return ['cargo','run','--locked','--offline','--manifest-path',str(R/'consumer/Cargo.toml'),'--bin',binary,'--target-dir',str(TARGET),'--',str(R/out)]
install(PUBLIC/'design/reviews/icon-gold7-57/before')
run('fixture-final-before',consumer('icon-direction-consumer','fixture-final-before'))
install(PUBLIC/'crates/tekes-ui-icons/assets')
shutil.copyfile(PUBLIC/'crates/tekes-ui-icons/catalog.json',SDK/'crates/tekes-ui-icons/catalog.json')
# Exact same SDK implementation, only the approved icon geometry changes.
run('catalog-final',consumer('catalog-review','catalog-final'))
run('fixture-final-after',consumer('icon-direction-consumer','fixture-final-after'))
run('gallery-final-2',['cargo','test','--locked','--offline','--manifest-path',str(SDK/'Cargo.toml'),'-p','tekes-ui-gallery','--target-dir',str(TARGET),'original_registered_icon_page_prepared_top_interior_bottom_gpu','--','--ignored','--nocapture','--test-threads=1'],{'TEKESUI_ICON_WHOLE_PAGE_OUT':str(R/'gallery-final-2')})
run('component-final',['cargo','test','--locked','--offline','--manifest-path',str(SDK/'examples/composition-consumer/Cargo.toml'),'--features','svg-icons','--test','icon_content','--test','icon_gpu','--test','icon_motion','--test','icon_status','--test','icon_status_gpu','--target-dir',str(TARGET),'--','--test-threads=1'])
run('native-final-build',['cargo','build','--locked','--offline','--manifest-path',str(SDK/'Cargo.toml'),'-p','tekes-ui-gallery','--target-dir',str(TARGET)])
(R/'final-source-sha256.json').write_text(json.dumps({str(p.relative_to(SDK)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(SDK.rglob('*')) if p.is_file()},indent=2))
