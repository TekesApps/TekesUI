#!/usr/bin/env python3
"""Analysis-only final gates; raw reports and frozen measurement scripts stay intact."""
import argparse
import copy
import hashlib
import json
from pathlib import Path


def finalize_row(source, report):
    row = copy.deepcopy(source)
    actual = report['presentation']['samples']
    targets = report['samples']['rows']
    last_target = targets[-1][0] if targets else None
    row['last_measured_frame_not_presented'] = any(
        frame[0] == last_target and frame[1] <= 0 for frame in actual)
    row['nonterminal_not_presented'] = row['not_presented']-int(row['last_measured_frame_not_presented'])
    # These are still failures, including the terminal receipt; no exemption.
    row['base_strict_zero_drop_pass'] = source['strict_zero_drop_pass']
    positive = [frame[1] for frame in actual if frame[1] > 0]
    mean = (positive[-1]-positive[0])*1000/(len(positive)-1) if len(positive)>1 else None
    row['actual_120hz_mean_interval_ms'] = mean
    row['actual_120hz_cadence_pass'] = mean is not None and abs(mean-1000/120)<=.05
    drain = report.get('presentation_drain')
    row['presentation_drain_pass'] = (None if drain is None else bool(
        drain.get('finished') and not drain.get('timed_out')
        and drain.get('unpresented') == 0
        and not drain.get('timer_only_diagnostic', False)))
    idle_wait = report.get('idle_after', {}).get('wait_s')
    row['full_idle_observation_pass'] = isinstance(idle_wait, (int, float)) and idle_wait >= 5.0
    row['strict_120hz_pass'] = bool(
        source['strict_zero_drop_pass'] and row['actual_120hz_cadence_pass']
        and not any(frame[1] <= 0 for frame in actual)
        and row['presentation_drain_pass'] is not False
        and row['full_idle_observation_pass'])
    row['quiet_120hz_acceptance_pass'] = bool(
        row['strict_120hz_pass'] and not row['gpu_timing']
        and not row['static_diagnostic'] and row.get('low_background_gate_passed')
        and row.get('external_process_cpu_below_threshold_during_all_samples') is True)
    columns = report['samples']['columns']
    if 'cpu_ms' in columns and 'budget_ms' in columns:
        cpu, budget = columns.index('cpu_ms'), columns.index('budget_ms')
        row['cpu_work_exceeding_available_budget'] = sum(r[cpu] > r[budget] for r in targets)
    return row


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--summary', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    if a.summary.resolve() == a.out.resolve(): p.error('Preserve the original summary; choose a different output')
    raw = a.summary.read_bytes()
    source = json.loads(raw)
    result = {'format':'tekesui-symbol-final-gates/1',
              'source_summary_sha256':hashlib.sha256(raw).hexdigest(),
              'cadence_rule':'actual mean interval within 0.05ms of 1000/120; all upstream integrity/zero-drop gates still required',
              'stop_edge_rule':'terminal unpresented receipts remain failures; never dropped or exempted',
              'rows':[]}
    for row in source['rows']:
        report = json.loads(Path(row['report']).read_text())
        result['rows'].append(finalize_row(row,report))
    a.out.parent.mkdir(parents=True,exist_ok=True)
    a.out.write_text(json.dumps(result,indent=2)+'\n')


if __name__ == '__main__': main()
