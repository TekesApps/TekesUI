#!/usr/bin/env python3
"""Summarize xctrace Metal XML exports, filtering one PID and merging GPU overlaps.

Export metal-gpu-intervals and metal-current-allocated-size with xctrace first.
GPU active time is this process's interval union, not device utilization. Render
pass spans include stage gaps; pass frequency is not presented-display FPS.
"""
import argparse
import collections
import json
import statistics
import xml.etree.ElementTree as ET


def rows(path):
    # xctrace shares values through id/ref, including nested process identifiers.
    refs = {}
    columns = None
    def value(element):
        if element.get('ref'):
            return refs[element.get('ref')]
        return refs.get(element.get('id'), (element.text or '', element.get('fmt', ''), None))
    for _, element in ET.iterparse(path, events=['end']):
        if element.tag == 'schema':
            columns = [c.findtext('mnemonic') for c in element.findall('col')]
        if element.get('id'):
            pid = element.find('pid')
            refs[element.get('id')] = (element.text or '', element.get('fmt', ''), int(value(pid)[0]) if pid is not None else None)
        if element.tag == 'row':
            yield dict(zip(columns, (value(c) for c in element)))
            element.clear()


def distribution(values):
    if not values:
        return None
    ordered = sorted(values)
    return dict(count=len(values), mean=statistics.mean(values), median=statistics.median(values), p95=ordered[max(0, int(len(values)*.95)-1)], maximum=ordered[-1], minimum=ordered[0])


def summarize(gpu_path, allocated_path, pid, window_seconds=None):
    intervals = []
    passes = collections.defaultdict(list)
    for row in rows(gpu_path):
        if row['process'][2] != pid or row['state'][0] != 'Active':
            continue
        start = int(row['start'][0]); end = start + int(row['duration'][0])
        intervals.append((start, end))
        label = row['event-label'][1]
        for name, prefix in [('scene3d', 'Scene3D:'), ('ui', 'tekes frame pass:')]:
            if label.startswith(prefix):
                passes[name, row['encoder-id'][0]].append((start, end))
    if not intervals:
        raise ValueError('No active GPU intervals for this PID; do not report an idle/occluded run as a benchmark.')
    first = min(x[0] for x in intervals); last = max(x[1] for x in intervals)
    recorded_duration = (last-first)/1e9
    if window_seconds is not None:
        if window_seconds <= 0 or recorded_duration < window_seconds:
            raise ValueError(f'GPU activity covers {recorded_duration:.3f}s, less than requested {window_seconds}s')
        trim = int(((last-first)-window_seconds*1e9)/2)
        first += trim
        last = first + int(window_seconds*1e9)
        intervals = [(max(a, first), min(b, last)) for a, b in intervals if b > first and a < last]
    end = first; busy = 0
    for a, b in sorted(intervals):
        busy += max(0, b-max(a, end)); end = max(end, b)
    duration = (last-first)/1e9
    summary = dict(pid=pid, recorded_activity_window_seconds=recorded_duration, active_window_seconds=duration, gpu_active_union_ms=busy/1e6, process_gpu_active_percent=100*busy/(last-first), render_passes={})
    for name in ['scene3d', 'ui']:
        spans = [(max(b for _, b in spans)-min(a for a, _ in spans))/1e6 for (kind, _), spans in passes.items() if kind == name and min(a for a, _ in spans) >= first and max(b for _, b in spans) <= last]
        stats = distribution(spans)
        if stats:
            summary['render_passes'][name] = dict(span_ms=stats, passes_per_second=len(spans)/duration)
    sizes = [int(r['current-allocated-size'][0]) / 2**20 for r in rows(allocated_path) if r['process'][2] == pid]
    summary['metal_current_allocated_mib'] = distribution(sizes)
    summary['scope'] = 'GPU interval union for the target PID; render-pass wall spans, not shader busy time; pass count is not presented FPS. Metal allocation overlaps process unified memory. Tracing overhead is separate from CPU baseline.'
    return summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--gpu', required=True)
    parser.add_argument('--allocated', required=True)
    parser.add_argument('--pid', required=True, type=int)
    parser.add_argument('--output', required=True)
    parser.add_argument('--seconds', type=float, help='Analyze the central N seconds; fail if GPU coverage is shorter')
    args = parser.parse_args()
    result = summarize(args.gpu, args.allocated, args.pid, args.seconds)
    with open(args.output, 'w') as output:
        json.dump(result, output, indent=2)
        output.write('\n')
    print(json.dumps(result, indent=2))
