#!/usr/bin/env python3
"""Measure process CPU time / monotonic wall time, using ps on macOS or Linux.

100% means one logical core. This measures CPU consumption, not FPS or GPU usage.
Keep the target visible/active, verify animation, and keep build/scene/size/scale
and refresh policy identical when comparing results.
"""
import argparse
import datetime
import json
import math
import os
from pathlib import Path
import subprocess
import time


def cpu_seconds(stamp):
    """ps TIME can be M:SS.ss, H:MM:SS, or D-HH:MM:SS."""
    days, clock = stamp.split('-') if '-' in stamp else ('0', stamp)
    return int(days) * 86400 + sum(
        float(value) * 60 ** index
        for index, value in enumerate(reversed(clock.split(':')))
    )


def read_process(pid):
    output = subprocess.check_output(
        ['ps', '-p', str(pid), '-o', 'time=,pcpu=,lstart='], text=True
    ).split()
    if len(output) < 3:
        raise RuntimeError('target process is missing')
    return cpu_seconds(output[0]), float(output[1]), ' '.join(output[2:])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('pid', type=int)
    parser.add_argument('--seconds', type=float, default=12)
    parser.add_argument('--interval', type=float, default=3)
    parser.add_argument('--label', default='')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.pid <= 0 or not all(math.isfinite(v) and v > 0 for v in [args.seconds, args.interval]):
        parser.error('PID, duration and interval must be positive and finite')
    if args.seconds < 2:
        parser.error('use at least 2 seconds to reduce ps TIME quantization error')
    initial, _, identity = read_process(args.pid)
    current = initial
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    begin = time.monotonic()
    samples = []
    while True:
        elapsed = time.monotonic() - begin
        if elapsed >= args.seconds:
            break
        time.sleep(min(args.interval, args.seconds - elapsed))
        current, instantaneous, latest_identity = read_process(args.pid)
        if latest_identity != identity:
            raise RuntimeError('PID was reused: discard this measurement')
        samples.append({'elapsed_seconds': round(time.monotonic() - begin, 4),
                        'ps_cpu_percent': instantaneous})
    elapsed = time.monotonic() - begin
    used = current - initial
    if used < 0:
        raise RuntimeError('process CPU counter moved backwards')
    result = dict(label=args.label, pid=args.pid, started_utc=started,
                  process_start=identity, logical_cores=os.cpu_count(),
                  wall_seconds=round(elapsed, 4), process_cpu_seconds=round(used, 4),
                  average_cpu_percent_one_core=round(100 * used / elapsed, 3),
                  samples=samples)
    data = json.dumps(result, indent=2) + '\n'
    if args.output:
        args.output.write_text(data)
    print(data, end='')


if __name__ == '__main__':
    main()
