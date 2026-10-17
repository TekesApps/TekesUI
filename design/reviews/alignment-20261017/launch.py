from pathlib import Path
import subprocess,os,time,json,signal,hashlib
root=Path.cwd();p=root/'target/alignment-review-20261017';app=root/'target/TekesUI Gallery.app';exe=app/'Contents/MacOS/TekesUI-Gallery'
def processes():
 result=[]
 for line in subprocess.check_output(['ps','-axo','pid=,command='],text=True).splitlines():
  parts=line.strip().split(None,1)
  if len(parts)==2 and parts[1]==str(exe):result.append(int(parts[0]))
 return result
old=processes();assert len(old)<=1,old
if old:
 assert old==[89850],('Unexpected existing Gallery process; preserve it',old)
 os.kill(old[0],signal.SIGTERM)
 for _ in range(50):
  if not processes():break
  time.sleep(0.1)
 assert not processes(),'Old Gallery did not exit'
cmd=['open','--stdout',str(p/'app-stdout.log'),'--stderr',str(p/'app-stderr.log'),'--env','TEKESUI_WINDOW_PLACEMENT=external','--env','TEKESUI_GALLERY_PAGE=component-chart/playground',str(app)]
r=subprocess.run(cmd,capture_output=True,text=True);assert r.returncode==0,r.stderr
for _ in range(80):
 new=processes()
 if new:break
 time.sleep(0.1)
assert len(new)==1,new
time.sleep(0.5);assert processes()==new,'Gallery exited immediately'
v={'old_exact_app_pid':old,'new_exact_app_pid':new,'command':cmd,'app_executable':str(exe),'sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'started':True,'native_visual_or_performance_test':False,'placement_request':'external','note':'App startup only. No native test, screen capture, monitor probe or 120Hz measurement.'}
(p/'launch.json').write_text(json.dumps(v,indent=2)+'\n');print(json.dumps(v,indent=2))
