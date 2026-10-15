#!/bin/bash
# F1 滚动门禁：在 120Hz 内建屏上跑 N 次恒速滚动，逐次写 JSON 报告，最后汇总。
#
# 用法：scripts/f1-scroll-gate.sh [次数，默认 5] [每次秒数，默认 10] [输出目录，默认 ./f1-scroll-out]
# 前提：已 cargo build --release -p tekes-ui-gallery。
# 环境变量：
#   CONTENT=mixed|quads|text   滚动的内容，默认 mixed（Gallery 各页整屏加夹具文字段）
#   SPEED=1200                 滚动速度，点/秒
# 窗口放哪：perf 模式请求 120Hz，摆放规则选内建屏；本机没有 120Hz 屏时刷新间隔会在报告里体现，
# 汇总里“刷新间隔”不是 8.33 ms 就说明没在 ProMotion 屏上，结果不能用来过门禁。
#
# 每次运行：启动 Gallery（TEKES_PERF=scroll:秒数）→ 预热 1.5 秒 → 记录 N 秒 → 关动画等 5 秒确认显示链接静默 → 退出。
# 退出码 0 表示完整跑完且空闲确认通过。汇总见 scripts/f1-scroll-summary.py（取每次 p99 的中位数和最坏值）。
set -euo pipefail
runs="${1:-5}"
secs="${2:-10}"
out="${3:-./f1-scroll-out}"
here="$(cd "$(dirname "$0")" && pwd)"
bin="$here/../target/release/tekes-ui-gallery"
mkdir -p "$out"

if [[ ! -x "$bin" ]]; then
  echo "没有 $bin，先 cargo build --release -p tekes-ui-gallery" >&2
  exit 2
fi

commit="$(git -C "$here/.." rev-parse HEAD 2>/dev/null || echo unknown)"
if [[ -n "$(git -C "$here/.." status --porcelain --untracked-files=no 2>/dev/null)" ]]; then
  commit="$commit+dirty"
fi
echo "提交 $commit"
echo "负载 $(uptime | sed 's/.*load averages*: //')（越低越好；别的编译或渲染任务会影响结果）"
others="$(pgrep -fl 'tekes-ui-gallery' || true)"
if [[ -n "$others" ]]; then
  echo "提示：已有 tekes-ui-gallery 进程在跑（不会动它）："
  echo "$others"
fi

status=0
for i in $(seq 1 "$runs"); do
  echo "== 第 $i/$runs 次"
  TEKES_PERF="scroll:$secs" \
  TEKES_PERF_OUT="$out/scroll-run$i.json" \
  TEKES_PERF_CONTENT="${CONTENT:-mixed}" \
  TEKES_PERF_SPEED="${SPEED:-1200}" \
  TEKES_PERF_COMMIT="$commit" \
    "$bin" 2> "$out/scroll-run$i.log" || { echo "第 $i 次未完整通过（退出码 $?），见 $out/scroll-run$i.log"; status=1; }
  tail -n 12 "$out/scroll-run$i.log" | sed 's/^/  /'
done

echo
python3 -I "$here/f1-scroll-summary.py" "$out"/scroll-run*.json || status=1
exit $status
