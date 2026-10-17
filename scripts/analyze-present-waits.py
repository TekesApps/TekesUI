#!/usr/bin/env python3
"""Keep callback/acquire/presentation timelines separate; queueing shifts gaps.

This is a diagnostic, not a causal classifier or a replacement for the strict
presentation gate. An application-cost spike and a display gap can overlap
without one proving the other caused it.
"""
import argparse
import gzip
import hashlib
import json
import math
from pathlib import Path


def distribution(values):
    values = sorted(values)
    if not values:
        return None
    return {"count": len(values), "mean": sum(values) / len(values),
            "p50": values[int((len(values) - 1) * .50)],
            "p99": values[int((len(values) - 1) * .99)], "max": values[-1]}


def budget_comparison(rows):
    margins = []
    for row in rows:
        budget, work = row.get('budget_ms'), row.get('cpu_ms')
        if (isinstance(budget, (int, float)) and isinstance(work, (int, float))
                and math.isfinite(budget) and math.isfinite(work)):
            margins.append(budget - work)
    return {'samples': len(rows), 'known_samples': len(margins),
            'unknown_samples': len(rows) - len(margins),
            'reported_budget_minus_handler_work_ms': distribution(margins),
            'handler_work_over_reported_budget': sum(v < 0 for v in margins),
            'scope': 'Reported dispatch-start budget minus measured handler wall time. Not native GPU scheduling, exact submit timestamp, or proof of meeting a display deadline.'}


def analyze(path):
    raw = path.read_bytes()
    report = json.loads(gzip.decompress(raw) if path.suffix == '.gz' else raw)
    rows = [dict(zip(report['samples']['columns'], row)) for row in report['samples']['rows']]
    frames = [dict(zip(report['presentation']['columns'], row)) for row in report['presentation']['samples']]
    by_target = {r['target_timestamp_s']: r for r in rows}
    work_keys = ('build_ms', 'encode_ms', 'submit_ms')
    work = [sum(row[key] for key in work_keys) for row in rows if all(key in row for key in work_keys)]
    refresh = report['stats']['refresh_interval_ms']
    joined = [(frame, by_target[frame['target_s']]) for frame in frames
              if frame['target_s'] in by_target and frame['presented_s'] > 0]
    base = joined[0][0]['target_s'] if joined else 0
    gaps = []
    unpresented_rows = [by_target[f['target_s']] for f in frames
                        if f['presented_s'] <= 0 and f['target_s'] in by_target]
    for i in range(1, len(joined)):
        gap = (joined[i][0]['presented_s'] - joined[i-1][0]['presented_s']) * 1000
        if gap <= refresh * 1.5:
            continue
        context = []
        for frame, row in joined[max(0, i-4):i+3]:
            context.append({
                'target_ms': (frame['target_s'] - base) * 1000,
                'callback_ms': (row['callback_time_s'] - base) * 1000,
                'acquired_ms': (frame['acquired_s'] - base) * 1000,
                'presented_ms': (frame['presented_s'] - base) * 1000,
                'cpu_ms': row['cpu_ms'], 'build_ms': row['build_ms'],
                'acquire_ms': row['acquire_ms'], 'budget_ms': row['budget_ms'],
            })
        gaps.append({'presented_interval_ms': gap, 'context': context})
    return {
        'report': str(path), 'sha256': hashlib.sha256(raw).hexdigest(),
        'render_queue': report.get('render_queue'), 'refresh_ms': refresh,
        'cpu_percent': report['process']['cpu_percent_while_scrolling'],
        'cpu_ms': report['stats']['cpu_ms'], 'acquire_ms': report['stats']['acquire_ms'],
        'build_encode_submit_wall_ms': distribution(work),
        'unpresented_targets_s': [frame['target_s'] for frame in frames if frame['presented_s'] <= 0],
        'handler_budget_all': budget_comparison(rows),
        'handler_budget_unpresented': budget_comparison(unpresented_rows),
        'work_timing_scope': 'Build + encode + submit wall time; excludes drawable acquisition. Not process CPU time or proof of a compositor cause.',
        'acquired_to_present_ms': distribution([(f['presented_s'] - f['acquired_s']) * 1000 for f, _ in joined]),
        'presentation_gap_events': len(gaps),
        'callback_drops': report['stats']['drops'], 'gaps': gaps,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reports', nargs='+', type=Path)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args()
    result = {'format': 'tekesui-presentation-timeline/1',
              'caution': 'Intervals measure separate pipeline stages; temporal association is not proof of causality.',
              'runs': [analyze(path) for path in args.reports]}
    text = json.dumps(result, indent=2) + '\n'
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text)
    else:
        print(text, end='')


if __name__ == '__main__':
    main()
