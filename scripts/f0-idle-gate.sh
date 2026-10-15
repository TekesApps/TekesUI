#!/bin/bash
# F0 空闲门禁：启动 Gallery（真实 wgpu 渲染），窗口可见、内容静止，量空闲 CPU 与重绘计数。
#
# 用法：scripts/f0-idle-gate.sh <次数，默认 5> [每次秒数，默认 60] [输出目录，默认 .]
# 前提：已 cargo build --release -p tekes-ui-gallery。
# 环境变量 SETTLE：启动后等多少秒再开始量，默认 5。
# 每次：启动 → 等 SETTLE 秒让启动稳定 → 记 Redraw 计数 → measure-idle-cpu.sh → 再记计数 → 结束进程。
# 第一次另外跑 sample 5，列出所有线程的栈顶，用来确认没有线程在空转。
set -euo pipefail
runs="${1:-5}"
secs="${2:-60}"
out="${3:-.}"
here="$(cd "$(dirname "$0")" && pwd)"
bin="$here/../target/release/tekes-ui-gallery"
mkdir -p "$out"

for i in $(seq 1 "$runs"); do
  log="$out/idle-run$i.log"
  TEKES_LOG_EVENTS="${LOG_EVENTS:-1}" "$bin" > "$log" 2>&1 &
  pid=$!
  trap 'kill $pid 2>/dev/null || true' EXIT
  sleep "${SETTLE:-5}"
  before=$(grep -c "Redraw #" "$log" || true)
  if [[ "$i" == "1" ]]; then
    sample "$pid" 5 -file "$out/idle-sample.txt" > /dev/null 2>&1 || true
  fi
  "$here/measure-idle-cpu.sh" "$pid" "$secs" > "$out/idle-run$i.cpu.txt"
  after=$(grep -c "Redraw #" "$log" || true)
  echo "run $i: pid $pid  $(tail -1 "$out/idle-run$i.cpu.txt")  Redraw 计数 $before -> $after"
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  trap - EXIT
done
