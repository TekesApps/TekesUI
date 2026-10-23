from pathlib import Path
import os,subprocess,json,hashlib
R=Path(__file__).resolve().parent;ROOT=R.parents[2];SDK=R/'sdk';TARGET=ROOT/'target/icon-direction-57'
def run(label,args,env=None):
    log=R/(label+'.log');assert not log.exists()
    with log.open('w')as f:p=subprocess.run(args,cwd=ROOT,env=dict(os.environ,**(env or{})),stdout=f,stderr=subprocess.STDOUT)
    print(label,p.returncode,flush=True)
    if p.returncode:raise SystemExit(p.returncode)
def consumer(binary,out):return ['cargo','run','--locked','--offline','--manifest-path',str(R/'consumer/Cargo.toml'),'--bin',binary,'--target-dir',str(TARGET),'--',str(R/out)]
test=['cargo','test','--locked','--offline','--manifest-path',str(SDK/'Cargo.toml'),'-p','tekes-ui-gallery','--target-dir',str(TARGET)]
run('gallery-c-square',test+['original_registered_icon_page_prepared_top_interior_bottom_gpu','--','--ignored','--nocapture','--test-threads=1'],{'TEKESUI_ICON_WHOLE_PAGE_OUT':str(R/'gallery-c-square')})
run('native-c-square',test+['icon_gold7_native_all_sizes_gpu','--','--ignored','--nocapture','--test-threads=1'],{'ICON_NATIVE_OUT':str(R/'native-c-square')})
run('native-regressions-square',test+['gallery::pages::showcase::tests','--','--test-threads=1'])

run('native-square-build',['cargo','build','--locked','--offline','--manifest-path',str(SDK/'Cargo.toml'),'-p','tekes-ui-gallery','--target-dir',str(TARGET)])
(R/'validated-square-source-sha256.json').write_text(json.dumps({str(p.relative_to(SDK)):hashlib.sha256(p.read_bytes()).hexdigest()for p in sorted(SDK.rglob('*'))if p.is_file()},indent=2)+'\n')
