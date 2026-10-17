from pathlib import Path
import subprocess,os,time,json,hashlib
p=Path('target/selection-components-20261017')
with (p/'package-final.log').open('w') as f:r=subprocess.run(['python3','scripts/package-gallery.py','--binary','target/presentation-repair-20261016/build/release/tekes-ui-gallery'],stdout=f,stderr=subprocess.STDOUT)
assert r.returncode==0,'package failed'
app=Path('target/TekesUI Gallery.app/Contents/MacOS/TekesUI-Gallery');before=p/'probe-build/baseline/debug/offscreen-selection-probe';current=p/'probe-build/current/debug/offscreen-selection-probe'
cases=[('snapshot-final',[str(app),'--snapshot',str(p/'snapshot-final')],{}),('interaction-before-final',[str(before),str(p/'interaction-before-final')],{}),('interaction-after',[str(current),str(p/'interaction-after')],{}),('keyboard-after',[str(current),str(p/'keyboard-after')],{'SELECTION_PROBE_KEYBOARD':'1'}),('large-text-after',[str(current),str(p/'large-text-after'),'--text-scale','2'],{})]
for name,cmd,extra in cases:
 env=os.environ.copy();env.update(extra);t=time.monotonic()
 with (p/(name+'.log')).open('w') as f:r=subprocess.run(cmd,stdout=f,stderr=subprocess.STDOUT,env=env)
 receipt=dict(command=cmd,extra_env=extra,exit=r.returncode,png_count=len(list((p/name).glob('*.png'))),binary_sha256=hashlib.sha256(Path(cmd[0]).read_bytes()).hexdigest(),seconds=round(time.monotonic()-t,2),mode='offscreen_no_native_window')
 (p/(name+'-run.json')).write_text(json.dumps(receipt,indent=2)+'\n');print(receipt,flush=True)
 if r.returncode:raise SystemExit(r.returncode)
