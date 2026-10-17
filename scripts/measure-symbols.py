#!/usr/bin/env python3
"""Run one real-window symbol workload; preserve raw perf JSON and process logs."""
import argparse, hashlib, json, os, pathlib, subprocess, time
p=argparse.ArgumentParser()
p.add_argument('--binary',required=True);p.add_argument('--sources',type=int,choices=[1,8,14,24],required=True)
p.add_argument('--mode',choices=['steady','parameters','retarget','combined'],required=True)
p.add_argument('--seconds',type=float,default=60);p.add_argument('--label',required=True)
p.add_argument('--driver',choices=['cad','metal'],default='cad');p.add_argument('--frame-latency',choices=['1','2'],default='2')
p.add_argument('--linear',action='store_true');p.add_argument('--freeze-raster',action='store_true');p.add_argument('--gpu-timing',choices=['0','1'],default='1');p.add_argument('--out',default='target/symbol-perf')
a=p.parse_args();out=pathlib.Path(a.out).resolve();out.mkdir(parents=True,exist_ok=True)
report=out/(a.label+'.json');log=out/(a.label+'.log');receipt=out/(a.label+'-run.json')
if report.exists() or log.exists():raise SystemExit('Refusing to replace existing measurement; use a fresh label')
binary=pathlib.Path(a.binary).resolve();env=os.environ.copy()
for k in ['TEKES_SELFTEST','TEKES_SELFTEST_OUT']:env.pop(k,None)
env.update(TEKES_METAL_DISPLAY_LINK='1' if a.driver=='metal' else '0',TEKES_PERF_FRAME_LATENCY=a.frame_latency,TEKES_PERF=f'scroll:{a.seconds}',TEKES_PERF_CONTENT='symbols',TEKES_PERF_OUT=str(report),TEKES_SYMBOL_SOURCES=str(a.sources),TEKES_SYMBOL_MODE=a.mode,TEKES_PERF_GPU_TIMING=a.gpu_timing,TEKES_SYMBOL_FREEZE_RASTER='1' if a.freeze_raster else '0',TEKES_SYMBOL_LINEAR='1' if a.linear else '0',TEKESUI_GALLERY_PAGE='motion-foundation',TEKESUI_WINDOW_PLACEMENT='builtin')
start=time.time();meta={'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'sources':a.sources,'animated_views':a.sources*3,'mode':a.mode,'display_driver':a.driver,'frame_latency':a.frame_latency,'seconds':a.seconds,'gpu_timing':a.gpu_timing,'linear':a.linear,'freeze_raster':a.freeze_raster,'started_unix':start,'ambient_processes_before':subprocess.check_output(['ps','-axo','pid,pcpu,comm'],text=True)}
receipt.write_text(json.dumps(meta,indent=2)+'\n')
with log.open('w') as f:
 try:r=subprocess.run([str(binary)],env=env,stdout=f,stderr=subprocess.STDOUT,timeout=a.seconds+60);meta['exit_code']=r.returncode
 except subprocess.TimeoutExpired:meta['timeout']=True
meta['wall_s']=time.time()-start;receipt.write_text(json.dumps(meta,indent=2)+'\n')
if not report.exists():raise SystemExit('Missing performance report; inspect '+str(log))
r=json.loads(report.read_text());s=r['stats'];w=r.get('symbols_workload',{});presentation=r['presentation']
summary={'label':a.label,'completed':r['completed'],'driver':r.get('display_driver'),'visibility':r.get('visibility'),'skipped_render_callbacks':presentation.get('skipped_render_callbacks'),'duplicate_drawable_serials':presentation.get('duplicate_drawable_serials'),'views':w.get('animated_views'),'errors':w.get('errors'),'linear':w.get('linear_rotation'),'asset_work':w.get('measured_asset_work'),'frames':s['frames'],'display':r['display'],'refresh_ms':s['refresh_interval_ms'],'cpu_percent':r['process']['cpu_percent_while_scrolling'],'cpu_peak_250ms':r['process']['cpu_rate_peak_250ms_percent'],'build_ms':s['build_ms'],'raster_ms':w.get('raster_and_upload_ms'),'gpu_ms':s['gpu_ms'],'callback_drops':s['drops'],'presentation_interval':presentation['interval_ms'],'presentation_gaps':presentation['gaps_over_1_5_refresh'],'missing_receipts':presentation['missing_receipts'],'not_presented':presentation['not_presented'],'idle':r['idle_after']}
print(json.dumps(summary,ensure_ascii=False),flush=True)
if meta.get('exit_code')!=0 or not r['completed'] or w.get('errors'):raise SystemExit(2)
