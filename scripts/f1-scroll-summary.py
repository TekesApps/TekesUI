#!/usr/bin/env python3
"""汇总 f1-scroll-gate.sh 的多次 JSON 报告：每次的 p99，中位数，最坏值，门禁判定。

门槛（comparison-method 第 2.3 节）：
  - 滚动帧时间 p99 <= 8.33 ms：CPU（干活，不含等 drawable）与 GPU 各自的 p99；
  - 不得有连续掉帧：一个间隔里连续丢掉的刷新数 <= 1（longest_drop_run）；
  - 在 120Hz 屏上量（刷新间隔 8.33 ms）。
另列出：含等待的 CPU p99、取 drawable p99、晚到帧数、空闲确认。
"""
import json
import statistics
import sys

LIMIT_MS = 8.33


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
    head = r0 = runs[0][1]
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
    if gpu:
        verdict(f"GPU p99 的中位数 <= {LIMIT_MS} ms", statistics.median(gpu) <= LIMIT_MS)
        verdict(f"GPU p99 的最坏值 <= {LIMIT_MS} ms", max(gpu) <= LIMIT_MS)
    else:
        verdict("GPU p99（没有 GPU 样本）", False)
    verdict("没有连续掉帧（最长连续丢 <= 1）", max(drops) <= 1)
    verdict("没有任何掉帧（晚到帧数全为 0）", all(v == 0 for v in col(lambda r: r["stats"]["drops"]["late_frames"])))
    verdict("每次都完整跑完", all(complete))
    verdict("结束后显示链接静默（空闲回调 0）", all(v == 0 for v in idle_cb))
    ok = ok_refresh and bool(cpu) and max(cpu) <= LIMIT_MS and bool(gpu) and max(gpu) <= LIMIT_MS \
        and max(drops) <= 1 and all(complete) and all(v == 0 for v in idle_cb)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
