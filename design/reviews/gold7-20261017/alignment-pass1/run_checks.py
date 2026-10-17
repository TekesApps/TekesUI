from pathlib import Path
import subprocess,os,time,json
p=Path('target/gold7-alignment-pass1'); env=os.environ.copy();env.update(CARGO_TARGET_DIR='target/presentation-repair-20261016/build',CARGO_BUILD_JOBS='4',TEKESUI_GOLD7_FOCUS_OUT=str((p/'focus').resolve()))
checks=[('tests-final',['cargo','test','-p','tekes-ui-controls','-p','tekes-ui-gallery','-p','tekes-ui-motion','-p','tekes-ui-icons','-p','tekes-ui-design-export','-p','tekes-ui-tokens','--release','--locked']),('clippy-final',['cargo','clippy','-p','tekes-ui-controls','-p','tekes-ui-gallery','-p','tekes-ui-motion','-p','tekes-ui-icons','-p','tekes-ui-design-export','-p','tekes-ui-tokens','--all-targets','--release','--locked','--','-D','warnings']),('build-final',['cargo','build','-p','tekes-ui-gallery','--release','--locked']),('fmt-final',['cargo','fmt','--all','--','--check']),('pure-rust-final',['python3','scripts/check-pure-rust.py']),('design-system-final',['target/design-check-venv/bin/python','scripts/check-design-system.py']),('diff-final',['git','diff','--check'])]
r=[]
for name,cmd in checks:
 t=time.monotonic()
 with (p/(name+'.log')).open('w') as f:code=subprocess.run(cmd,env=env,stdout=f,stderr=subprocess.STDOUT).returncode
 r.append(dict(check=name,command=cmd,exit=code,seconds=round(time.monotonic()-t,2)))
 (p/'checks-final.json').write_text(json.dumps(r,indent=2)+'\n');print(r[-1],flush=True)
 if code:raise SystemExit(code)
