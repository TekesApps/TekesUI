#!/bin/bash
# 测一个进程的空闲 CPU。
#
# 用法：scripts/measure-idle-cpu.sh <pid> [秒数，默认 30]
#
# 每秒采样一次（top 的 %CPU 与 IDLEW 累计计数），逐秒打印，
# 最后打印 CPU 的中位数、最大值和首末保留样本间的唤醒增量（不含已丢弃首样本边界）。
# top 的第一个 CPU 样本缺少前一采样基线，丢弃。
#
# 环境变量 MAX_CPU：设了就在最大值超过它时以退出码 1 结束，便于脚本化判定。
# 想看主线程是否停在 mach_msg，另外运行：sample <pid> 5
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "用法：$0 <pid> [秒数]" >&2
  exit 2
fi
pid="$1"
secs="${2:-30}"

if ! kill -0 "$pid" 2>/dev/null; then
  echo "进程 $pid 不存在" >&2
  exit 2
fi

# top -l N：共 N 个样本，间隔 1 秒；第 1 个丢弃，所以要 secs+1 个。
samples=$(top -l $((secs + 1)) -s 1 -pid "$pid" -stats pid,cpu,idlew \
  | awk -v pid="$pid" '$1 == pid { print $2, $3 }' | tail -n +2)

if [[ -z "$samples" ]]; then
  echo "没有采到样本（进程是否中途退出？）" >&2
  exit 2
fi

echo "$samples" | awk '{ printf "t=%2d  cpu=%6s%%  idle_wakeups=%s\n", NR, $1, $2 }'

echo "$samples" | awk -v max_cpu="${MAX_CPU:-}" '
  { cpu[NR] = $1 + 0; if (NR == 1) wake_first = $2 + 0; wake_last = $2 + 0; n = NR }
  END {
    # 排序求中位数（n 很小，插入排序足够）
    for (i = 1; i <= n; i++) s[i] = cpu[i]
    for (i = 2; i <= n; i++) { v = s[i]; j = i - 1; while (j > 0 && s[j] > v) { s[j+1] = s[j]; j-- } s[j+1] = v }
    median = (n % 2) ? s[(n+1)/2] : (s[n/2] + s[n/2+1]) / 2
    max = s[n]
    printf "\n样本数 %d  中位数 %.1f%%  最大值 %.1f%%  观察区间空闲唤醒增量 %d（首末保留样本之差，不含已丢弃首样本边界）\n", n, median, max, wake_last - wake_first
    if (max_cpu != "" && max > max_cpu + 0) { printf "超过阈值 %s%%\n", max_cpu; exit 1 }
  }'
