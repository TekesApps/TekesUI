#!/usr/bin/env python3
"""Summarize both callback and actual Metal presentation evidence; never hide failed runs."""
import argparse
import hashlib
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('reports', nargs='+', type=Path)
parser.add_argument('--out', type=Path)
args = parser.parse_args()
results = []
for path in args.reports:
    d = json.loads(path.read_text())
    stats, presented = d['stats'], d.get('presentation', {})
    interval = presented.get('interval_ms', {})
    budget = stats['refresh_interval_ms']
    reasons = []
    if not d['completed']: reasons.append('measurement did not complete')
    if abs(budget - 1000 / 120) > 0.05: reasons.append('not a 120Hz display run')
    if stats['drops']['dropped_refreshes']: reasons.append('display callback skipped refreshes')
    if presented.get('receipts', 0) != stats['frames']: reasons.append('missing presentation receipts')
    if presented.get('not_presented', 0): reasons.append('drawable was not presented')
    if presented.get('gaps_over_1_5_refresh', 0): reasons.append('actual presentation skipped refreshes')
    pacing_passed = not reasons
    if stats['cpu_ms']['max'] > budget: reasons.append('cpu_ms peak exceeds refresh budget')
    if not stats.get('gpu_ms'): reasons.append('GPU counters disabled; GPU time unverified in this run')
    elif stats['gpu_ms']['max'] > budget: reasons.append('gpu_ms peak exceeds refresh budget')
    if d.get('idle_after', {}).get('display_link_callbacks') != 0: reasons.append('idle display callbacks did not stop')
    results.append({
        'report': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
        'content': d['config']['content'], 'seconds': d['config']['seconds'],
        'frames': stats['frames'], 'passed_presentation_gate': pacing_passed,
        'gpu_counters_enabled': d.get('gpu_timing', {}).get('enabled', stats.get('gpu_ms') is not None),
        'passed_strict_gate': not reasons, 'failures': reasons,
        'refresh_budget_ms': budget, 'actual_interval_ms': interval,
        'actual_gaps': presented.get('gaps_over_1_5_refresh'),
        'callback_dropped_refreshes': stats['drops']['dropped_refreshes'],
        'cpu_frame_ms': stats['cpu_ms'], 'acquire_ms': stats['acquire_ms'],
        'gpu_ms': stats['gpu_ms'], 'cpu_percent': d['process']['cpu_percent_while_scrolling'],
        'cpu_peak_250ms_percent': d['process']['cpu_rate_peak_250ms_percent'],
        'idle': d['idle_after'],
        'target_to_present_ms': presented.get('target_lateness_ms'),
    })
output = {'format': 'tekesui-scroll-repair-evidence/1',
          'cpu_unit': '100% = one logical core; process user + system CPU',
          'latency_note': '8.333ms is refresh cadence, not input-to-display latency; target-to-present is reported separately.',
          'runs': results}
text = json.dumps(output, ensure_ascii=False, indent=2) + '\n'
if args.out:
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(text)
else:
    print(text, end='')
