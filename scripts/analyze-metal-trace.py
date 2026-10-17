#!/usr/bin/env python3
"""Analyze exported xctrace tables, keeping GPU work and drawable waits separate.

Only process-tagged target rows and exact command-buffer ID joins are used.
Display-wide surfaces are not attributed to Gallery without a surface mapping.
All times here are trace-relative nanoseconds, not the harness's media clock.
"""
import argparse
from collections import Counter, defaultdict
import gzip
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET


def table(path):
    root = ET.parse(path).getroot()
    ids = {e.get('id'): e for e in root.iter() if e.get('id')}

    def resolve(e):
        while e.get('ref'):
            e = ids[e.get('ref')]
        return e

    def value(e):
        e = resolve(e)
        if e.tag == 'sentinel':
            return None
        if e.tag in ('process', 'thread'):
            for child in e.iter():
                child = resolve(child)
                if child.tag == 'pid':
                    return int(child.text)
                if child.tag == 'process' and child is not e:
                    return value(child)
            return None
        if len(e):
            return e.get('fmt')
        if e.tag in ('start-time', 'duration', 'metal-command-buffer-id',
                     'uint32', 'uint64', 'boolean'):
            return int(e.text)
        return e.text or e.get('fmt', '')

    columns = [c.findtext('mnemonic') for c in root.findall('.//schema/col')]
    return [dict(zip(columns, (value(e) for e in row))) for row in root.findall('.//row')]


def distribution_ns(values):
    values = sorted(v / 1e6 for v in values)
    if not values:
        return None
    return {'count': len(values), 'mean_ms': sum(values) / len(values),
            'p50_ms': values[int((len(values) - 1) * .50)],
            'p99_ms': values[int((len(values) - 1) * .99)], 'max_ms': values[-1]}


def analyze(directory):
    receipt_path = directory / 'receipt.json'
    if not receipt_path.exists():
        receipt_path = directory / 'capture-receipt.json'
    receipt = json.loads(receipt_path.read_text())
    if not receipt.get('complete'):
        raise ValueError('Capture/export did not complete; refusing to analyze as valid')
    pid = receipt['pid']
    projection_path = directory / 'target-tables.json.gz'
    projection = None
    if projection_path.exists():
        manifest = json.loads((directory / 'manifest.json').read_text())
        raw = projection_path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != manifest[projection_path.name]['sha256']:
            raise ValueError('Target table projection checksum mismatch')
        projection = json.loads(gzip.decompress(raw))
        if projection['format'] != 'tekesui-target-metal-tables/1' or projection['pid'] != pid:
            raise ValueError('Target table projection identity mismatch')

    def read(schema):
        if projection is not None:
            return projection['tables'][schema]
        path = directory / (schema + '.xml')
        if hashlib.sha256(path.read_bytes()).hexdigest() != receipt['tables'][schema]['sha256']:
            raise ValueError('Source XML checksum mismatch: ' + schema)
        return table(path)

    submitted = [r for r in read('metal-application-command-buffer-submissions')
                 if r['process'] == pid]
    target_gpu = [r for r in read('metal-gpu-intervals') if r['process'] == pid]
    gpu = [r for r in target_gpu if r['state'] == 'Active']
    waits = [r for r in read('ca-client-buffer-wait-interval') if r['thread'] == pid]
    completed = {r['cmdbuffer-id']: r['timestamp'] for r in read('metal-command-buffer-completed')}
    by_buffer = defaultdict(list)
    by_channel = defaultdict(list)
    for row in gpu:
        by_buffer[row['cmdbuffer-id']].append(row)
        by_channel[row['channel-name']].append(row['duration'])
    frames = []
    for row in submitted:
        if 'tekes frame pass' not in (row['event-label'] or ''):
            continue
        intervals = by_buffer[row['cmdbuffer-id']]
        if not intervals:
            continue
        begin = min(r['start'] for r in intervals)
        end = max(r['start'] + r['duration'] for r in intervals)
        commit = row['start'] + row['duration']
        completion = completed.get(row['cmdbuffer-id'])
        frames.append({'buffer': row['cmdbuffer-id'], 'cpu_commit_end_ns': commit,
                       'gpu_start_ns': begin, 'gpu_end_ns': end,
                       'gpu_span_ns': end - begin,
                       'commit_to_gpu_start_ns': begin - commit,
                       'commit_to_completion_ns': None if completion is None else completion - commit})
    frames.sort(key=lambda r: r['cpu_commit_end_ns'])
    long_waits = sorted(waits, key=lambda r: r['duration'], reverse=True)[:12]
    context = []
    for wait in long_waits:
        start, end = wait['start'], wait['start'] + wait['duration']
        overlapping = [f for f in frames if f['gpu_start_ns'] < end and f['gpu_end_ns'] > start]
        context.append({'start_s': start / 1e9, 'duration_ms': wait['duration'] / 1e6,
                        'overlapping_target_render_gpu_spans': len(overlapping),
                        'overlapping_render_span_ms': distribution_ns([f['gpu_span_ns'] for f in overlapping])})
    return {'format': 'tekesui-metal-trace-analysis/1', 'pid': pid,
            'diagnostic_only': True, 'quiet_certified': False,
            'scope': 'Instrumented run only. Overlap is not causality; GPU spans exclude display scanout and compositor. Unmatched IDs are omitted, never assigned zero cost. Display-wide surfaces are not attributed to Gallery.',
            'target_submissions': len(submitted), 'target_gpu_intervals': len(gpu),
            'excluded_non_active_gpu_intervals': len(target_gpu) - len(gpu),
            'submission_labels': dict(Counter(r['event-label'] for r in submitted)),
            'joined_render_buffers': len(frames),
            'render_submissions_without_gpu_match': sum('tekes frame pass' in (r['event-label'] or '') for r in submitted) - len(frames),
            'render_buffers_without_completion': sum(f['commit_to_completion_ns'] is None for f in frames),
            'gpu_channel_execution': {k: distribution_ns(v) for k, v in by_channel.items()},
            'render_gpu_span': distribution_ns([f['gpu_span_ns'] for f in frames]),
            'render_commit_to_gpu_start': distribution_ns([f['commit_to_gpu_start_ns'] for f in frames]),
            'render_commit_to_completion': distribution_ns([f['commit_to_completion_ns'] for f in frames if f['commit_to_completion_ns'] is not None]),
            'drawable_wait': distribution_ns([r['duration'] for r in waits]),
            'drawable_waits_over_8_33ms': sum(r['duration'] > 1e9 / 120 for r in waits),
            'longest_drawable_wait_context': context}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    args.out.write_text(json.dumps(analyze(args.directory), indent=2) + '\n')


if __name__ == '__main__':
    main()
