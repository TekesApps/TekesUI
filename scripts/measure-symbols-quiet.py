#!/usr/bin/env python3
"""Wait for low background CPU before each real-window symbol case.

This gate cannot prove the absence of GPU, storage or user interference. It
records observed CPU deltas before/during each run, never stops other processes,
and refuses to launch after the bounded wait expires. Keep the window visible.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def cpu_seconds(value):
    days, clock = (value.split('-', 1) if '-' in value else ('0', value))
    parts = list(map(float, clock.split(':')))
    result = 0.0
    for part in parts: result = result*60+part
    return int(days)*86400+result


def snapshot():
    rows = subprocess.check_output(['ps', '-axo', 'pid=,time=,comm='], text=True)
    result = {}
    for row in rows.splitlines():
        parts = row.strip().split(None, 2)
        if len(parts) == 3:
            result[int(parts[0])] = (cpu_seconds(parts[1]), parts[2])
    return time.monotonic(), result


def usage(previous, current, binary, excluded, per_process, total_limit):
    elapsed = current[0]-previous[0]
    processes = []
    for pid, (cpu, command) in current[1].items():
        if pid in excluded or command == binary or Path(command).name == 'ps': continue
        before = previous[1].get(pid)
        # A newly observed process may have existed for less than the sample
        # period. Count all accumulated CPU, capped only by its actual total.
        percent = max(0, cpu-(before[0] if before else 0))/elapsed*100
        if percent > .1: processes.append({'pid': pid, 'cpu_percent': percent, 'command': command})
    processes.sort(key=lambda p:p['cpu_percent'], reverse=True)
    total = sum(p['cpu_percent'] for p in processes)
    heavy_names = {'rustc','cargo','du','find','Storage','StorageManagementService','ApplicationsStorageExtension'}
    blockers = [p for p in processes if p['cpu_percent'] > per_process
                or (p['cpu_percent'] > 1 and (Path(p['command']).name in heavy_names
                    or ('/target/' in p['command'] and '/deps/' in p['command'])))]
    return {'wall_unix': time.time(), 'interval_s': elapsed, 'cpu_percent_total': total,
            'quiet': not blockers and total <= total_limit,
            'blockers': blockers, 'processes': processes}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary', required=True)
    p.add_argument('--out', required=True)
    p.add_argument('--driver',choices=['cad','metal'],default='cad')
    p.add_argument('--frame-latency',choices=['1','2'],default='2')
    p.add_argument('--wait-seconds', type=float, default=180)
    p.add_argument('--stable-seconds', type=float, default=15)
    p.add_argument('--interval', type=float, default=3)
    p.add_argument('--per-process-limit', type=float, default=30)
    p.add_argument('--total-limit', type=float, default=100)
    p.add_argument('--seconds', type=float, default=60)
    p.add_argument('--prefix', default='quiet')
    p.add_argument('--gate-only', action='store_true')
    p.add_argument('--case', choices=['all','multi-24','parameters-24','retarget-24','combined-72','linear-24'], default='all')
    a = p.parse_args()
    if min(a.wait_seconds,a.stable_seconds,a.interval,a.seconds,a.per_process_limit,a.total_limit) <= 0:
        p.error('durations and limits must be positive')
    out = Path(a.out).resolve(); out.mkdir(parents=True, exist_ok=True)
    binary = str(Path(a.binary).resolve())
    cases = [('multi-24',8,'steady',False), ('parameters-24',8,'parameters',False),
             ('retarget-24',8,'retarget',False), ('combined-72',24,'combined',False),
             ('linear-24',8,'steady',True)]
    if a.case != 'all': cases = [case for case in cases if case[0] == a.case]
    if a.gate_only: cases = [('gate-only',8,'steady',False)]
    for label,sources,mode,linear in cases:
        label = a.prefix+'-'+label
        audit_path = out/(label+'-ambient.json')
        if audit_path.exists(): raise SystemExit('Refusing to overwrite '+str(audit_path))
        audit = {'format':'tekesui-ambient-cpu/1','label':label,
                 'limits':vars(a),'caution':'CPU-only low-background gate; does not certify GPU, I/O or user inactivity. WindowServer activity during measurement can be caused by the benchmark itself; exceeding a threshold is not causal evidence of outside interference.',
                 'before':[],'during':[]}
        start = time.monotonic(); stable = 0; previous = snapshot()
        while True:
            time.sleep(a.interval); current = snapshot()
            row = usage(previous,current,binary,{os.getpid()},a.per_process_limit,a.total_limit)
            previous = current; audit['before'].append(row)
            stable = stable+row['interval_s'] if row['quiet'] else 0
            audit_path.write_text(json.dumps(audit,indent=2)+'\n')
            if stable >= a.stable_seconds: break
            if time.monotonic()-start >= a.wait_seconds:
                audit['gate_passed'] = False; audit_path.write_text(json.dumps(audit,indent=2)+'\n')
                print('No low-background window; measurement not launched: '+label,flush=True)
                raise SystemExit(3)
        audit['gate_passed'] = True
        print('Low-background CPU gate passed: '+label,flush=True)
        if not a.gate_only:
            cmd=[sys.executable,'scripts/measure-symbols.py','--binary',binary,'--out',str(out),
                 '--label',label,'--sources',str(sources),'--mode',mode,'--seconds',str(a.seconds),
                 '--gpu-timing','0','--driver',a.driver,'--frame-latency',a.frame_latency]
            if linear: cmd+=['--linear']
            process=subprocess.Popen(cmd)
            previous=snapshot()
            while process.poll() is None:
                time.sleep(a.interval);current=snapshot()
                audit['during'].append(usage(previous,current,binary,{os.getpid(),process.pid},a.per_process_limit,a.total_limit));previous=current
            audit['exit_code']=process.returncode
            audit['external_process_cpu_below_threshold_during_all_samples']=all(r['quiet'] for r in audit['during'])
        audit_path.write_text(json.dumps(audit,indent=2)+'\n')
        if audit.get('exit_code',0):raise SystemExit(audit['exit_code'])


if __name__ == '__main__':
    main()
