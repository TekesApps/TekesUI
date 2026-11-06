#!/usr/bin/env python3
"""Measure real Gallery scrolling after the shared low-background CPU gate.

Run sequentially after all other native/GPU work has stopped. This CPU-only
gate does not certify GPU, storage or user inactivity. Never stops processes.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import time


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


quiet = load('quiet', 'measure-symbols-quiet.py')
summary = load('summary', 'f1-scroll-summary.py')


QUIET_METHODS = {
    'raw': 'tekesui-scroll-quiet/raw-v1',
    'known-external': 'tekesui-scroll-quiet/known-external-v1',
}


def quiet_sample_passed(row, method):
    # CLI validates method once; usage() owns the unchanged 30/100 rules.
    return row['quiet'] if method == 'raw' else row['cpu_attribution']['known_external']['below_limits']


def quiet_during_passed(rows, method):
    return bool(rows) and all(quiet_sample_passed(row, method) for row in rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--quiet-method', choices=QUIET_METHODS, default='raw',
                        help='Explicit method identity: raw retains shared compositor; known-external uses attributed external CPU only')
    parser.add_argument('--binary', required=True)
    parser.add_argument('--out', required=True)
    parser.add_argument('--prefix', required=True)
    parser.add_argument('--driver', choices=['cad', 'metal'], default='cad')
    parser.add_argument('--shared-desktop-diagnostic', action='store_true',
                        help='Record observed load and run without quiet certification; gate_passed is always false')
    parser.add_argument('--content', nargs='+', choices=['scroll-area', 'editor', 'icons'], default=['scroll-area', 'editor', 'icons'], help='Select sequential real-page workloads; default remains all three')
    parser.add_argument('--gpu-timing', choices=['0','1'], default='0', help='Enable GPU timestamp diagnostics; records observer overhead separately')
    parser.add_argument('--frame-latency', choices=['1','2'], default='2', help='FIFO diagnostic only; production default remains 2')
    parser.add_argument('--work-done-host', action='store_true', help='Record completion notification host times as upper bounds only; does not change acceptance thresholds')
    parser.add_argument('--reference-metadata', action='store_true', help='Request opt-in actual ScrollArea layout metadata; does not change acceptance thresholds')
    parser.add_argument('--workbench', action='store_true', help='Exercise the current registered workbench route and its real chrome')
    parser.add_argument('--seconds', type=float, default=60)
    parser.add_argument('--wait-seconds', type=float, default=180)
    parser.add_argument('--stable-seconds', type=float, default=15)
    parser.add_argument('--interval', type=float, default=3)
    args = parser.parse_args()
    if min(args.seconds, args.wait_seconds, args.stable_seconds, args.interval) <= 0:
        parser.error('durations must be positive')
    binary = Path(args.binary).resolve()
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    checksum = hashlib.sha256(binary.read_bytes()).hexdigest()
    results = []
    for content in dict.fromkeys(args.content):
        label = args.prefix+'-'+content
        report = out/(label+'.json')
        audit_path = out/(label+'-ambient.json')
        if any(path.exists() for path in (report, audit_path, out/(label+'-run.json'), out/(label+'.log'))):
            raise SystemExit('Refusing to overwrite '+label)
        audit = {'format': 'tekesui-ambient-cpu/1', 'label': label,
                 'quiet_method': args.quiet_method, 'quiet_method_id': QUIET_METHODS[args.quiet_method],
                 'limits': {**vars(args), 'per_process_limit': 30, 'total_limit': 100},
                 'caution': 'CPU-only low-background gate; does not certify GPU, I/O or user inactivity. WindowServer activity during measurement can be caused by the benchmark itself; exceeding a threshold is not causal evidence of outside interference.',
                 'before': [], 'during': []}
        def save():
            audit_path.write_text(json.dumps(audit, indent=2)+'\n')
        start = time.monotonic()
        stable = raw_stable = 0
        previous = quiet.snapshot()
        while True:
            time.sleep(args.interval)
            current = quiet.snapshot()
            row = quiet.usage(previous, current, str(binary), {os.getpid()}, 30, 100)
            previous = current
            row['selected_quiet'] = quiet_sample_passed(row, args.quiet_method)
            audit['before'].append(row)
            raw_stable = raw_stable+row['interval_s'] if row['quiet'] else 0
            stable = stable+row['interval_s'] if row['selected_quiet'] else 0
            save()
            if args.shared_desktop_diagnostic or stable >= args.stable_seconds:
                break
            if time.monotonic()-start >= args.wait_seconds:
                audit.update(gate_passed=False, raw_gate_passed=False, selected_gate_passed=False)
                save()
                print('No quiet window; native measurement not launched: '+label, flush=True)
                raise SystemExit(3)
        audit['raw_gate_passed'] = not args.shared_desktop_diagnostic and raw_stable >= args.stable_seconds
        audit['gate_passed'] = audit['selected_gate_passed'] = not args.shared_desktop_diagnostic
        audit['launch_mode'] = 'shared-desktop-diagnostic' if args.shared_desktop_diagnostic else 'quiet-gated'
        save()
        if hashlib.sha256(binary.read_bytes()).hexdigest() != checksum:
            raise SystemExit('Frozen binary changed')
        env = {key: value for key,value in os.environ.items()
               if not key.startswith(('TEKES_PERF', 'TEKES_SELFTEST', 'TEKES_SYMBOL'))}
        env.update(TEKES_PERF=f'scroll:{args.seconds}', TEKES_PERF_CONTENT=content,
                   TEKES_PERF_OUT=str(report), TEKES_PERF_GPU_TIMING=args.gpu_timing,
                   TEKES_PERF_FRAME_LATENCY=args.frame_latency, TEKES_METAL_DISPLAY_LINK='1' if args.driver=='metal' else '0',
                   TEKESUI_GALLERY_LOCALE='en', TEKESUI_WINDOW_PLACEMENT='builtin',
                   TEKES_PERF_WORKBENCH='1' if args.workbench else '0')
        if args.work_done_host:
            env['TEKES_PERF_WORK_DONE_HOST'] = '1'
        if args.reference_metadata:
            env['TEKES_PERF_REFERENCE'] = '1'
        receipt = {'quiet_method': args.quiet_method, 'quiet_method_id': QUIET_METHODS[args.quiet_method],
                   'raw_gate_passed': audit['raw_gate_passed'], 'selected_gate_passed': audit['selected_gate_passed'],
                   'work_done_host_requested': args.work_done_host, 'reference_metadata_requested': args.reference_metadata, 'binary': str(binary), 'binary_sha256': checksum,
                   'driver': args.driver, 'frame_latency': int(args.frame_latency), 'gpu_timing_requested': args.gpu_timing == '1', 'content': content,
                   'seconds': args.seconds, 'workbench': args.workbench, 'started_unix': time.time()}
        receipt_path = out/(label+'-run.json')
        receipt_path.write_text(json.dumps(receipt, indent=2)+'\n')
        print('Starting '+label, flush=True)
        with (out/(label+'.log')).open('w') as log:
            process = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=subprocess.STDOUT)
            previous = quiet.snapshot()
            while process.poll() is None:
                time.sleep(args.interval)
                current = quiet.snapshot()
                row = quiet.usage(previous, current, str(binary), {os.getpid(), process.pid}, 30, 100)
                row['selected_quiet'] = quiet_sample_passed(row, args.quiet_method)
                audit['during'].append(row)
                previous = current
                save()
        receipt.update(exit_code=process.returncode, finished_unix=time.time(),
                       binary_sha256_after=hashlib.sha256(binary.read_bytes()).hexdigest())
        receipt_path.write_text(json.dumps(receipt, indent=2)+'\n')
        audit['exit_code'] = process.returncode
        # Legacy field remains raw; selected outcome is named and recorded separately.
        audit['external_process_cpu_below_threshold_during_all_samples'] = quiet_during_passed(audit['during'], 'raw')
        audit['selected_process_cpu_below_threshold_during_all_samples'] = quiet_during_passed(audit['during'], args.quiet_method)
        receipt.update(raw_ambient_cpu_below_limits=audit['external_process_cpu_below_threshold_during_all_samples'],
                       selected_ambient_cpu_below_limits=audit['selected_process_cpu_below_threshold_during_all_samples'])
        receipt_path.write_text(json.dumps(receipt, indent=2)+'\n')
        save()
        if not report.exists():
            raise SystemExit('Missing native report: '+label)
        data = json.loads(report.read_text())
        reasons = summary.strict_verdict(data)
        if receipt['binary_sha256_after'] != checksum:
            reasons.append('binary_changed_during_measurement')
        results.append({'quiet_method': args.quiet_method, 'quiet_method_id': QUIET_METHODS[args.quiet_method],
                        'case': label, 'cpu_percent': data['process']['cpu_percent_while_scrolling'],
                        'presentation': {key: value for key,value in data['presentation'].items() if key != 'samples'},
                        'strict_rejection_reasons': reasons, 'exit_code': process.returncode,
                        'raw_ambient_cpu_below_limits': audit['external_process_cpu_below_threshold_during_all_samples'],
                        'selected_ambient_cpu_below_limits': audit['selected_process_cpu_below_threshold_during_all_samples'],
                        'ambient_cpu_below_limits': audit['selected_process_cpu_below_threshold_during_all_samples'],
                        'raw_quiet_gate_passed': audit['raw_gate_passed'],
                        'selected_quiet_gate_passed': audit['selected_gate_passed'],
                        'quiet_gate_passed': audit['gate_passed'],
                        'raw_quiet_gate_and_zero_drop_pass': not reasons and audit['raw_gate_passed'] and audit['external_process_cpu_below_threshold_during_all_samples'],
                        'quiet_gate_and_zero_drop_pass': not reasons and audit['gate_passed'] and audit['selected_process_cpu_below_threshold_during_all_samples']})
        (out/(args.prefix+'-summary.json')).write_text(json.dumps(results, indent=2)+'\n')
        print(label+': '+('mechanical gate passed' if not reasons else ', '.join(reasons)), flush=True)
        if process.returncode:
            raise SystemExit(process.returncode)


if __name__ == '__main__':
    main()
