#!/usr/bin/env python3
"""Summarize native scroll reports using actual presentation receipts.

A good p99 never certifies zero drops. Older reports without receipt integrity
and visibility fields remain useful diagnostics but cannot pass this gate.
"""
import json
import math
import statistics
import sys

LIMIT_MS = 8.33


def strict_verdict(report):
    """Return all rejection reasons; never convert missing evidence to a pass."""
    reasons = []
    stats = report['stats']
    refresh = stats['refresh_interval_ms']
    if not math.isfinite(refresh) or abs(refresh-1000/120) > .05:
        reasons.append('not_120hz')
    if not report.get('completed'):
        reasons.append('incomplete')
    frames, rendered = stats['frames'], stats.get('rendered_frames')
    if not frames or rendered != frames:
        reasons.append('skipped_or_unknown_render_callbacks')
    visibility = report.get('visibility')
    if visibility is None or visibility.get('occluded_callbacks') != 0 or visibility.get('non_key_callbacks') != 0:
        reasons.append('visibility_invalid_or_unrecorded')
    drain = report.get('presentation_drain')
    if drain is not None and (drain.get('finished') is not True or drain.get('timed_out') is not False or drain.get('unpresented') != 0):
        reasons.append('presentation_drain_failed_or_unfinished')
    if drain is not None and drain.get('timer_only_diagnostic'):
        reasons.append('drain_timer_only_diagnostic')
    presentation = report.get('presentation', {})
    for key in ('missing_receipts', 'not_presented', 'duplicate_drawable_serials'):
        if presentation.get(key) != 0:
            reasons.append(key+'_or_unknown')
    samples = presentation.get('samples', [])
    if len(samples) != rendered:
        reasons.append('receipt_count_mismatch')
    times = [row[1] for row in samples if math.isfinite(row[1]) and row[1] > 0]
    intervals = [(b-a)*1000 for a,b in zip(times,times[1:])]
    if len(times) != len(samples) or len(times) < 2 or any(v <= 0 or v > refresh*1.5 for v in intervals):
        reasons.append('actual_presentation_gap_or_invalid_timeline')
    if not intervals or abs(sum(intervals)/len(intervals)-1000/120) > .05:
        reasons.append('actual_refresh_not_120hz')
    if len(times) < 2 or times[-1]-times[0] < report['config']['seconds']-2*refresh/1000:
        reasons.append('insufficient_presented_duration')
    if stats['drops']['dropped_refreshes'] != 0:
        reasons.append('callback_missed_refreshes')
    cpu = stats['cpu_ms'].get('max', float('nan'))
    raw = report.get('samples', {})
    columns = raw.get('columns', [])
    if 'cpu_ms' in columns:
        values = [row[columns.index('cpu_ms')] for row in raw['rows']]
        if any(not math.isfinite(v) for v in values):
            reasons.append('invalid_callback_cpu_samples')
        cpu = max([cpu]+values)
    if not math.isfinite(cpu) or cpu > refresh:
        reasons.append('cpu_exceeds_frame_period_or_unknown')
    wait = (report.get('idle_after') or {}).get('wait_s')
    if not isinstance(wait, (int, float)) or not math.isfinite(wait) or wait < 5:
        reasons.append('idle_observation_short_or_unknown')
    if (report.get('idle_after') or {}).get('display_link_callbacks') != 0:
        reasons.append('idle_callbacks_or_unknown')
    return reasons


def main(paths):
    runs = []
    for p in paths:
        with open(p, encoding="utf-8") as f:
            runs.append((p, json.load(f)))
    if not runs:
        print("没有报告文件")
        return 1

    def col(get):
        return [get(r) for _, r in runs]

    def p99(key):
        return col(lambda r: (r["stats"].get(key) or {}).get("p99", float("nan")))

    rows = [
        ("CPU 干活 p99 (ms)", p99("cpu_active_ms")),
        ("CPU 含等 p99 (ms)", p99("cpu_ms")),
        ("取 drawable p99 (ms)", p99("acquire_ms")),
        ("建场景 p99 (ms)", p99("build_ms")),
        ("GPU p99 (ms)", p99("gpu_ms")),
        ("回调间隔 max (ms)", col(lambda r: r["stats"]["target_interval_ms"]["max"])),
        ("晚到帧数", col(lambda r: r["stats"]["drops"]["late_frames"])),
        ("丢刷新数", col(lambda r: r["stats"]["drops"]["dropped_refreshes"])),
        ("最长连续丢", col(lambda r: r["stats"]["drops"]["longest_drop_run"])),
        ("帧数", col(lambda r: r["stats"]["frames"])),
        ("刷新间隔 (ms)", col(lambda r: r["stats"]["refresh_interval_ms"])),
        ("空闲回调数", col(lambda r: (r.get("idle_after") or {}).get("display_link_callbacks", float("nan")))),
        ("空闲 CPU (ms/5s)", col(lambda r: (r.get("idle_after") or {}).get("cpu_ms", float("nan")))),
    ]
    head = runs[0][1]
    print(f"{head['machine'].get('model')} {head['machine'].get('chip')} {head['machine'].get('os')} "
          f"({head['machine'].get('os_build')})  屏幕 {head['display']}")
    print(f"内容 {head['config']['content']}，{head['config']['speed_pt_s']} pt/s，每次 {head['config']['seconds']} s，"
          f"{len(runs)} 次，提交 {head['tekesui'].get('commit')}")
    print()
    print(f"{'指标':<22}" + "".join(f"{'#'+str(i+1):>9}" for i in range(len(runs))) + f"{'中位数':>10}{'最坏':>10}")
    for name, vals in rows:
        nums = [v for v in vals if v == v]
        med = statistics.median(nums) if nums else float("nan")
        worst = max(nums) if nums else float("nan")
        cells = "".join(f"{v:>9.3f}" if isinstance(v, float) else f"{v:>9}" for v in vals)
        print(f"{name:<22}{cells}{med:>10.3f}{worst:>10.3f}")

    def verdict(label, ok):
        print(f"  {label:<44}{'通过' if ok else '未通过'}")

    print()
    cpu = [v for v in p99("cpu_active_ms") if v == v]
    gpu = [v for v in p99("gpu_ms") if v == v]
    drops = col(lambda r: r["stats"]["drops"]["longest_drop_run"])
    refresh = col(lambda r: r["stats"]["refresh_interval_ms"])
    idle_cb = col(lambda r: (r.get("idle_after") or {}).get("display_link_callbacks", 1))
    complete = col(lambda r: r["completed"])
    ok_refresh = all(abs(v - 1000 / 120) < 0.2 for v in refresh)
    verdict("在 120Hz 上量（刷新间隔 8.33 ms）", ok_refresh)
    verdict(f"CPU 干活 p99 的中位数 <= {LIMIT_MS} ms", bool(cpu) and statistics.median(cpu) <= LIMIT_MS)
    verdict(f"CPU 干活 p99 的最坏值 <= {LIMIT_MS} ms", bool(cpu) and max(cpu) <= LIMIT_MS)
    if len(gpu) == len(runs):
        verdict(f"GPU p99 的中位数 <= {LIMIT_MS} ms", statistics.median(gpu) <= LIMIT_MS)
        verdict(f"GPU p99 的最坏值 <= {LIMIT_MS} ms", max(gpu) <= LIMIT_MS)
    else:
        print(f"  GPU 计时：未测或不完整（{len(gpu)}/{len(runs)} 组有样本）；不推断 GPU 耗时")
    verdict("回调连续丢刷新诊断（最长 <= 1）", max(drops) <= 1)
    verdict("回调截止时间诊断（晚到帧数为 0）", all(v == 0 for v in col(lambda r: r["stats"]["drops"]["late_frames"])))
    verdict("每次都完整跑完", all(complete))
    verdict("结束后显示链接静默（空闲回调 0）", all(v == 0 for v in idle_cb))
    strict = []
    for path, report in runs:
        reasons = strict_verdict(report)
        strict.append(not reasons)
        print(f"  实际呈现严格门禁 {path}: {'通过' if not reasons else '未通过: '+', '.join(reasons)}")
    return 0 if all(strict) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
