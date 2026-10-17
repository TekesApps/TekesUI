#!/usr/bin/env python3
"""Summarize complete presented timelines; never hide invalid/failed runs."""
import argparse
import json
import math
from pathlib import Path


def distribution(values):
    ordered = sorted(values)
    result = {'count':len(ordered), 'min':min(ordered,default=0),
              'max':max(ordered,default=0), 'mean':sum(ordered)/len(ordered) if ordered else 0}
    result.update({key:ordered[int((len(ordered)-1)*fraction)] if ordered else 0
                   for key,fraction in [('p50',.5),('p95',.95),('p99',.99)]})
    return result


def summarize(report, name, path):
    s, w, pr = report['stats'], report['symbols_workload'], report['presentation']
    refresh = s['refresh_interval_ms']
    # Bridge unpresented receipts: adjacency in the receipt list is not adjacency
    # in the displayed timeline. Keep only actual, positive presented timestamps.
    times = [v[1] for v in pr['samples'] if math.isfinite(v[1]) and v[1] > 0]
    intervals = [(b-a)*1000 for a, b in zip(times, times[1:])]
    gaps = sum(v > refresh*1.5 for v in intervals)
    actual_dist = distribution(intervals)
    rendered = s.get('rendered_frames')
    skipped = s['frames']-rendered if rendered is not None else None
    visibility = report.get('visibility')
    integrity = []
    if skipped != 0: integrity.append('skipped_render_callbacks_or_unknown')
    if visibility is None: integrity.append('visibility_unrecorded')
    elif visibility['occluded_callbacks'] or visibility['non_key_callbacks']:
        integrity.append('window_occluded_or_not_frontmost')
    if pr.get('duplicate_drawable_serials') != 0:
        integrity.append('duplicate_drawable_serials_or_legacy_harness')
    if pr['missing_receipts']: integrity.append('missing_receipts')
    if pr['not_presented']: integrity.append('unpresented_receipts')
    if any(v <= 0 for v in intervals): integrity.append('non_monotonic_presented_timeline')
    if len(times) < 2: integrity.append('insufficient_presented_timeline')
    presented_duration = times[-1]-times[0] if len(times)>1 else 0
    if presented_duration < report['config']['seconds']-2*refresh/1000:
        integrity.append('presented_timeline_shorter_than_configured_duration')
    if abs(refresh-1000/120) > .05: integrity.append('not_120hz')
    raw = report.get('samples',{})
    columns = raw.get('columns',[])
    def callback_dist(name):
        if name not in columns: return None
        index = columns.index(name)
        return distribution([sample[index] for sample in raw['rows'] if sample[index] is not None])
    row = {
        'case': name, 'report': str(path), 'seconds': report['config']['seconds'],
        'gpu_timing': report['gpu_timing']['enabled'], 'sources': w['sources'],
        'animated_views': w['animated_views'], 'mode': w['mode'],
        'linear_rotation': w.get('linear_rotation', False),
        'asset_work': w.get('measured_asset_work'),
        'static_diagnostic': w.get('static_texture_diagnostic', False),
        'completed': report['completed'], 'all_models_stopped': w.get('all_models_stopped'),
        'errors': w['errors'], 'display': report['display'], 'refresh_ms': refresh,
        'display_driver': report.get('display_driver'), 'visibility': visibility,
        'rendered_callbacks': rendered, 'total_callbacks': s['frames'],
        'skipped_render_callbacks': skipped, 'measurement_integrity_issues': integrity,
        'cpu_percent': report['process']['cpu_percent_while_scrolling'],
        'cpu_peak_250ms_percent': report['process']['cpu_rate_peak_250ms_percent'],
        'model_ms': w['model_and_control_ms'], 'raster_upload_ms': w['raster_and_upload_ms'],
        'cpu_frame_ms': s['cpu_ms'], 'cpu_active_ms': s['cpu_active_ms'],
        'all_callback_cpu_ms': callback_dist('cpu_ms'),
        'all_callback_build_ms': callback_dist('build_ms'),
        'dispatch_delay_ms': callback_dist('dispatch_delay_ms'),
        'build_ms': s['build_ms'], 'acquire_ms': s['acquire_ms'], 'gpu_ms': s['gpu_ms'],
        'presentation_interval_ms': actual_dist,
        'harness_presentation_interval_ms': pr['interval_ms'],
        'presentation_gap_events': gaps,
        'harness_presentation_gap_events': pr['gaps_over_1_5_refresh'],
        'presentation_missed_refreshes_estimate': sum(max(0, round(v/refresh)-1) for v in intervals),
        'presented_duration_s': presented_duration,
        'presented_fps': (len(times)-1)/(times[-1]-times[0]) if len(times)>1 and times[-1]>times[0] else 0,
        'callback_dropped_refreshes': s['drops']['dropped_refreshes'],
        'missing_receipts': pr['missing_receipts'], 'not_presented': pr['not_presented'],
        'idle_after': report['idle_after'],
        'edits_including_warmup': w['parameter_edits_including_warmup'],
        'targets_including_warmup': w['target_changes_including_warmup'],
        'images_per_frame': report['scene_per_frame_median']['images'],
    }
    row['strict_zero_drop_pass'] = bool(
        report['completed'] and not row['static_diagnostic'] and not integrity
        and not w['errors'] and w.get('all_models_stopped') is True
        and gaps == 0 and s['drops']['dropped_refreshes'] == 0
        and s['cpu_ms']['max'] <= refresh
        and report['idle_after'].get('display_link_callbacks') == 0)
    return row


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--dir', default='target/symbol-perf')
    p.add_argument('--out', default='design/reviews/symbols-performance/summary.json')
    args = p.parse_args()
    rows = []
    for path in sorted(Path(args.dir).glob('*.json')):
        if path.name.endswith('-run.json'): continue
        report = json.loads(path.read_text())
        if isinstance(report, dict) and report.get('symbols_workload'):
            row = summarize(report, path.stem, path.resolve())
            ambient_path = path.with_name(path.stem+'-ambient.json')
            row['ambient_cpu_audit'] = str(ambient_path.resolve()) if ambient_path.exists() else None
            row['low_background_gate_passed'] = None
            if ambient_path.exists():
                ambient = json.loads(ambient_path.read_text())
                row['low_background_gate_passed'] = ambient.get('gate_passed',False)
                row['external_process_cpu_below_threshold_during_all_samples'] = ambient.get('external_process_cpu_below_threshold_during_all_samples')
                row['background_activity_blocker_samples'] = sum(bool(sample['blockers']) for sample in ambient.get('during',[]))
            # Mechanical zero-drop result and environment evidence are separate:
            # a clear CPU gate cannot certify GPU/I/O silence.
            row['quiet_gate_and_zero_drop_pass'] = bool(row['strict_zero_drop_pass'] and row['low_background_gate_passed'] and row.get('external_process_cpu_below_threshold_during_all_samples') is True)
            rows.append(row)
    output = Path(args.out)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps({
        'cpu_unit': '100% = one logical CPU core, user+system',
        'drop_definition': 'positive actual presented timestamps bridged over unpresented receipts; interval > 1.5 refreshes',
        'rows': rows}, indent=2)+'\n')
    for r in rows:
        print(f"{r['case']}: {r['animated_views']} views, CPU {r['cpu_percent']:.2f}%, gaps {r['presentation_gap_events']}, strict pass {r['strict_zero_drop_pass']}, integrity {r['measurement_integrity_issues']}")


if __name__ == '__main__':
    main()
