# 修正统计与低后台负载重测

本目录记录后续正式重测，旧 symbols-performance 与 symbols-gpu-cache 原始数据不覆盖。

## 统计修正

- 跳过渲染的回调不继承上一个 drawable serial；只有新渲染进入呈现回执匹配。
- 每个 drawable serial 只匹配一次，重复数量单列；missing receipts 以实际渲染次数为分母。
- 明确记录 total/rendered/skipped callbacks、occluded/non-key 回调和窗口状态变更。
- 对正值、实际呈现时间先过滤再计算相邻间隔，避免一个未呈现的收据把两侧真实长间隔遮掉。保留 harness 自报分布与重新计算分布供比对。
- 严格通过要求：120Hz、测量完整、无模型错误、无 skipped/缺失/未呈现/重复、全程可见且前台、实际长间隔0、callback漏刷新0、CPU整帧最大不超过8.33ms、所有模型结束暂停、idle额外callback0。旧版缺少可见性/重复统计时不自动认定通过。

CPU低或p99<8.33ms均不能替代上述呈现验收。所有失败与未测保持原状。

## 复现

```sh
python3 scripts/test-symbol-performance-scripts.py
python3 scripts/measure-symbols-quiet.py \
  --binary 'target/TekesUI Display Check.app/Contents/MacOS/TekesUI-Display-Check' \
  --driver metal --frame-latency 2 \
  --out target/symbol-perf-corrected --prefix corrected
python3 scripts/summarize-symbol-perf.py \
  --dir target/symbol-perf-corrected \
  --out design/reviews/symbols-corrected-performance/summary.json
```

候选二进制需以最终签名路径与哈希为准，上述路径为调用示例。

`measure-symbols-quiet.py` 默认顺序测量24混合播放、24调参、24中途切目标、72组合压力、24匀速旋转，各60秒。每组前连续15秒低后台CPU才启动，最多等待180秒。单个外部进程30%、合计100%（100%=一个逻辑核心），编译/扫描等已知重负载活动额外阻止启动。用累积CPU时间差分，避免ps长期均值误判。完整门槛/每次采样与进程名写入本地ambient.json，不停止其他进程。测试中继续采样；WindowServer可能由基准自己的合成负载引起，不能仅凭其CPU阈值超出认定外部干扰。

这些门槛是测量环境筛选，并非120Hz验收阈值，也不能证明GPU/I/O完全无干扰。仍需确认没有其他GPU任务或用户切换窗口；正式报告同时核查实际前台/可见性。该脚本不会放宽性能标准来获得通过。

## 当前进展

统计脚本7项回归通过，覆盖跳过/遮挡/失焦/重复/缺失/旧harness、未呈现回执桥接、CPU预算/idle、跳过渲染时的完整回调成本、差分CPU门槛与时间格式。共享桌面的30秒、180秒、180秒三次门槛探测均未找到低后台负载窗口，没有启动正式矩阵；本地证据为target/symbol-perf-corrected/{probe,wait,await-final}-gate-only-ambient.json。显示路径实验与最终候选仍由主控整合，尚未宣布稳定120Hz通过。


## 正式候选预检结果

最终Gallery候选已独立冻结（build-measured.json；签名SHA d025dc1bb1cefde04d7461b674c43be58477a13c2d1610f83096e69530b5db34）。正式预检又等待600秒，仍未获得连续15秒低负载，因此退出3，五组原生测试均未启动。门槛未通过是环境阻塞，不能写成性能失败，也不能写成通过。全程审计摘要见preflight-result.json，逐次进程记录仅保留本地，不含任何伪造或拼接测量。等待系统储存/磁盘扫描结束后重试；当前严格120Hz仍未验收。
