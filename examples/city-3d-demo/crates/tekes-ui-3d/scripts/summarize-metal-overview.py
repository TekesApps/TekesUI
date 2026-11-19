#!/usr/bin/env python3
"""Summarize per-PID Metal Performance Overview counters, without full GPU tracing.

Inputs are separate XML exports of layer-duration-metric, layer-count-metric,
and optionally metal-current-allocated-size. GPU ms is frame-weighted;
FPS is time-weighted. These are native Metal layer counters, not headless FPS
or whole-device GPU utilization. The optional memory source may be a snapshot.
"""
import argparse
import collections
import importlib.util
import json
from pathlib import Path

spec = importlib.util.spec_from_file_location('metal_trace', Path(__file__).with_name('summarize-metal-trace.py'))
trace = importlib.util.module_from_spec(spec)
spec.loader.exec_module(trace)


def summarize(duration_path, fps_path, allocated_path, pid, seconds=None):
    durations = [r for r in trace.rows(duration_path) if r['process'][2] == pid and r['metric-variant'][0] == '0']
    layers = collections.Counter()
    for r in durations:
        if r['name'][1] == 'GPU Active Time':
            layers[r['layer-id'][0]] += int(r['number-of-frames'][0])
    if not layers:
        raise ValueError('No GPU frame counters for this PID; reject idle/occluded recordings')
    layer = layers.most_common(1)[0][0]
    durations = [r for r in durations if r['layer-id'][0] == layer]
    starts = [int(r['start'][0]) for r in durations]
    ends = [int(r['start'][0])+int(r['duration'][0]) for r in durations]
    first, last = min(starts), max(ends)
    coverage = (last-first)/1e9
    if seconds is None:
        seconds = coverage
    if seconds <= 0 or coverage < seconds:
        raise ValueError(f'Counter coverage {coverage:.3f}s is shorter than requested {seconds}s')
    first += int(((last-first)-seconds*1e9)/2)
    last = first+int(seconds*1e9)

    def overlap(row):
        a = int(row['start'][0]); b = a+int(row['duration'][0])
        return max(0, min(b, last)-max(a, first))

    metrics = collections.defaultdict(list)
    for r in durations:
        o = overlap(r)
        if o:
            frames = int(r['number-of-frames'][0])*o/int(r['duration'][0])
            metrics[r['name'][1]].append((float(r['value-in-ms'][0]), frames))
    result = dict(pid=pid, layer=layer, recorded_counter_coverage_seconds=coverage,
                  analysis_seconds=seconds, gpu_metrics={})
    for name, values in metrics.items():
        weight = sum(w for _, w in values)
        if weight:
            result['gpu_metrics'][name] = dict(frame_weighted_mean_ms=sum(v*w for v,w in values)/weight,
                counter_windows=len(values), weighted_frames=weight,
                minimum_window_mean_ms=min(v for v,_ in values), maximum_window_mean_ms=max(v for v,_ in values))
    counts = collections.defaultdict(list)
    for r in trace.rows(fps_path):
        if r['process'][2] == pid and r['layer-id'][0] == layer:
            o = overlap(r)
            if o:
                counts[r['name'][1]].append((float(r['metric-value'][0]), o))
    result['layer_counters'] = {name:dict(time_weighted_mean=sum(v*w for v,w in vs)/sum(w for _,w in vs),
        minimum_window_value=min(v for v,_ in vs), maximum_window_value=max(v for v,_ in vs), counter_windows=len(vs)) for name,vs in counts.items()}
    if 'FPS' not in result['layer_counters'] or result['layer_counters']['FPS']['time_weighted_mean'] < 1:
        raise ValueError('No sustained presentation counters; reject paused recordings')
    if allocated_path:
        sizes = [int(r['current-allocated-size'][0])/2**20 for r in trace.rows(allocated_path) if r['process'][2] == pid]
        result['metal_current_allocated_mib'] = trace.distribution(sizes)
        result['memory_source'] = str(allocated_path)
    result['scope'] = ('GPU times are frame-weighted averages of Metal per-window frame counters; FPS is the native layer presentation counter. '
        'Window means are not individual-frame percentiles; GPU time is not whole-device utilization. '
        'Memory is actual MTLDevice allocation in the provided sample, overlapping process unified memory; a short memory sample cannot establish a 60s peak.')
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--duration', required=True)
    parser.add_argument('--fps', required=True)
    parser.add_argument('--allocated')
    parser.add_argument('--pid', type=int, required=True)
    parser.add_argument('--seconds', type=float, help='Optional exact central analysis window; default uses all reported counter coverage')
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    result = summarize(args.duration, args.fps, args.allocated, args.pid, args.seconds)
    Path(args.output).write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))
