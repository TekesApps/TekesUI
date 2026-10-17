# Metal 图标动画缩放恢复与性能重测

## 已确认根因与修复

旧候选的原生复现会在第一次缩放时退出：`CAMetalLayerInvalidOperation: -setMaximumDrawableCount should not be called when using CAMetalDisplayLink.` wgpu 的 surface 重配会配置 drawable 数量，不能与仍附着的 Metal 显示链接同时执行。旧失败日志保留在 `target/metal-lifecycle/before.log`，退出码 -6。

- 所有几何/倍率重配前，拆除旧 Metal link，释放排队的旧 drawable，恢复普通 acquire 后再重配和重绘；完成后在主 run loop 延后创建新 link。
- live resize 期间临时采用 CADisplayLink 计时，普通 wgpu drawable 路径绘制；host 的同步 Redraw 在 live resize 期间不再只置 pending。结束后暂停 CAD 并重建 Metal。默认 driver 仍为 CAD；没有无条件提升实验路径。
- 忽略旧 link 和退出 fallback 后尾随的 CAD 回调；run-loop 注册前先持有新 link。暂停、关闭均在释放 RefCell 借用之后调用 ObjC invalidate/释放对象，丢弃 drawable 时也不跨越队列借用。
- 保留 callback 的强引用 guard，避免 callback 内暂停或关闭造成悬垂；不会通过提前释放 guard 规避系统异常。

该路径遵循 Apple 的 [CAMetalDisplayLink](https://developer.apple.com/documentation/quartzcore/cametaldisplaylink) 与 [delegate 的 drawable 契约](https://developer.apple.com/documentation/quartzcore/cametaldisplaylinkdelegate/metaldisplaylink(_:needsupdate:))。具体根因来自本机原生异常；未将未公开的实现细节当成官方保证。

## 功能验证

修复后的 Metal 和默认 CAD 各通过同一份 22 检查点原生脚本：3轮 live resize 期间持续呈现、结束恢复、程序 resize、暂停、继续、目标反向与目标展开。每个活动区间要求至少10次正值实际 presented 收据，暂停稳定后要求0次。测试调用公开 AppKit live-resize 入口，是合成时序回归，不是物理鼠标轨迹采样，也不是120Hz性能通过证明。

独立 native example `metal_reentrant_resize` 在第30/60/90个动画 callback 内直接改窗口尺寸，再在第150个 callback 内关闭。旧 link 的 lifetime guard 在同一 dispatch 循环里仍有效；测试实际跑完150个callback，并收到超过120次正值 presented。这验证 invalidate 足以解除本机的配置冲突，且回调内部关闭没有崩溃。

CUA实际窗口验证：拖拽改变宽度、高度后界面继续更新，Pause速度0、Resume后目标反向速度-0.5、目标0均观察通过。最终手动日志含真实拖拽期间同步绘制及 positive presented 计数；不能据单次拖拽宣称所有尺寸/屏幕组合验收。

完整Gallery157通过/1既有忽略、平台19通过、perf9通过；计量脚本8项回归通过，严格all-targets Clippy与纯Rust guard583文件通过。原始本地日志 `target/metal-lifecycle/` 保留了旧异常、脚本误用Retarget代替Pause的中间失败，以及最终双driver和reentrant结果；不覆盖失败证据。

## 冻结性能矩阵

`build-measured.json` 给出244份编译输入与测量脚本SHA、签后二进制SHA：`6f3bcfc32dee901fca1c9bb342ca10bfcbc7c7cab4fb1b0053ce6f8385f14c0f`。正常Gallery未被覆盖。

同候选顺序测量 CAD 24实例稳态60秒，以及 Metal 24稳态/24调参/24切目标/72组合压力/24匀速旋转，每组60秒；GPU时间戳关闭。正式验收要求每组前低负载CPU门槛连续15秒通过才启动，测试中继续采样外部进程；窗口必须保持可见、前台。本轮两次门槛等待180秒和600秒均未通过、没有启动正式测量。以下是另行标记的共享桌面诊断，不替代正式验收。CPU门槛不能证明没有GPU/I/O干扰，WindowServer可能也承担本次基准的合成工作，不能仅由它的占用作因果推断。

wgpu配置使用FIFO/期望frame latency2；Metal link另请求1帧latency。JSON明确区分配置值与未经测量的Metal内部drawable池大小。完整positive presented时序必须覆盖整段配置时长，且不因未呈现收据间断而漏算长间隔。

共享桌面诊断结果（同一6f候选，非随机顺序，后台负载变化，不是严格A/B）：

|场景|CPU %（100%=一核）|构建p99 ms|实际呈现长间隔数|未呈现收据|
|---|---:|---:|---:|---:|
|CAD 24稳态|21.28|2.290|189|3|
|Metal 24稳态|24.23|2.350|214|174|
|Metal 24调参|20.51|2.221|43|36|
|Metal 24切目标|20.82|1.947|201|200|
|Metal 72组合|31.54|2.313|2|3|
|Metal 24匀速旋转|6.99|0.056|2|2|

所有场景窗口全程可见、前台，没有跳过回调、重复或丢失序号；停止后5秒内额外callback/acquire均为0。所有场景均未达到严格120Hz通过，且后台CPU门槛测试中也未通过。72实例比24实例缺口少，不能推导成实例越多越快：两组外部CPU平均负载分别约189%和764%。

20秒GPU诊断中render pass平均0.199ms、p99 0.261ms、最大0.350ms，实际正值呈现无长间隔，但末帧未呈现，因此仍失败。GPU时间戳只覆盖render pass，不覆盖排队和系统合成。

72组合和匀速旋转在首个测量样本后28.324739秒同时出现缺口，约对应进程启动30秒；旧CAD、静态纹理及滚动场景也存在类似启动时间缺口。当前不能归因于新图标模型或特定Metal shader，仍需定点采样。旧6f候选停止时立即拆除显示链接，可能取消最后一帧；新PresentationDrain候选单独验证，不回写或豁免旧报告的末帧失败。

`summary-shared.json`保留基础门槛；`final-gates-shared.json`额外要求实际平均间隔在8.333ms±0.05ms，避免100Hz无大间隔被误报120Hz。原始测量JSON、ambient审计和失败日志保持不变。不得把功能验证、低acquire成本或发出120Hz请求当作稳定120Hz通过。
