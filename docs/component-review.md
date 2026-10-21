# 组件小宿主、公共缓存与产物回收

本地整改以全部 WIP 为准。小宿主跟随正在整改的组件，不因为示例而增加组件范围。
复用 `examples/composition-consumer/examples/native.rs` 的公共 API / CoreText / Metal / 窗口 / 输入与 AX 宿主。
目前已有 `list`、`dialog`、`slider`、`command`、`reading`、`plain` 六个真实场景；只运行所选场景，不载入 Gallery 工作台。
这是独立消费者，不是“仅编译某个组件”：controls 仍是一个 crate，GPU / 文本依赖仍需构建。
其他组件推进到独立消费者时接入同一宿主，不提前创建所有组件场景，也不复制公共状态机。

## 每轮流程

从仓库根目录运行；Python 3.9+，完整 Gallery 图标打包仍要求 Python 3.11+。
下面的 `slider` 仅说明已有场景用法，实际选择当前工作组件。

```sh
python3 scripts/component-review.py build slider slider-current-round
python3 scripts/component-review.py check slider-current-round
```

`build` 不启动 UI。它使用当前工作树和 `--locked --offline`，创建一个可启动、ad-hoc 签名的
`target/component-review/<round>/TekesUI Component Review.app`。
只允许一个未收尾轮次，避免并行替换同一宿主。原生操作通过已授权的 UI 工具进行；
非 120Hz 验收优先外接屏，记录显示器、scale、主题及输入方式。不运行性能测试。

`check` 分别记录共享 controls API 测试和独立消费者接入测试；这些结果不代表已观察到 GPU、
物理输入、VoiceOver 或 Gold7。后续可将现有离屏 GPU 工具与原生观测的独立证据文件登记：

```sh
python3 scripts/component-review.py record slider-current-round gpu /absolute/gpu-result.json --result passed --note '实际 GPU 场景和边界'
python3 scripts/component-review.py record slider-current-round native /absolute/native-result.json --result passed --note '外接屏、交互和可访问性实际覆盖范围'
python3 scripts/component-review.py record slider-current-round gold7 /absolute/gold7-result.json --result passed --note '实际体验对照范围和未验项'
python3 scripts/component-review.py finish slider-current-round
```

`record` 是显式登记已观察的结果，不自动判断截图是否合格；原始截图/日志应先放入稳定的
验收证据目录，登记文件列出这些附件的路径与哈希，不引用待清理 App 内的附件。
源码哈希变化会使本轮结果失效，必须新建轮次。轮次名不得复用，不覆盖证据。
任何一层失败就保留失败记录；修复后开新轮次。需要清理未验收或失败的临时产物时使用：

```sh
python3 scripts/component-review.py finish slider-current-round --discard
```

这会标记 `discarded`，不会声称组件通过。普通 `finish` 要求 API、消费者、GPU、原生和 Gold7
均已通过并检查证据哈希。源码、测试、回执和日志留在 `design/reviews/component-runs/<round>/`；
只清除有所有权标记的该轮 App 和 Cargo 报告的组件示例产物，公共依赖缓存保留。

### 原生批次与活动状态

原生通过要求实际输入发生时窗口处于活动状态；读取日志时的当前前台状态不能替代它。
连续执行确定的测试步骤，每步读取实际 AX 状态；以原生事件、事件前/后状态和递增序号绑定结果，
同时校验唯一进程、签名二进制、源码身份及外接屏。依赖前一步焦点的链若出现 `Focus(false)`，
保留中断证据并拒绝该链；不能仅凭选中值不变或后来重新激活就认定通过。

证据完整时可以在操作后读取事件历史，不要求用户为读取日志一直保持测试窗口在前台。
这不允许修改原生活动标志、补造输入或跳过失败步骤。工具驱动的键鼠/AX、物理输入和 VoiceOver
分别记录，互不替代。当前实例见 `design/reviews/segmented-focus-order-20261020/record-native-batch.py`。
关闭后先检查准确进程已退出；不要为了确认关闭而再次获取已关闭应用的状态，部分 UI 入口会重新启动应用。

## 公共缓存与完整接入检查

本流程与 `scripts/package-gallery.py` 共用唯一 `target/review-cache`。
现有品牌消费者检查和 motion 合约检查也复用同一目录与构建锁；品牌测试仍显式切换各自
设计产物，Cargo 按其构建输入重建，品牌隔离验收不变。
motion 检查的日志改为每轮独立目录，可用 `--output <新目录>` 指定；已存在的目录会拒绝，
不再覆盖历史 `target/motion-state-transition-check` 下的工作日志。
固定 dev/test 配置为无 debug symbols、无 incremental；保留构建日志和源码哈希以支持诊断。
工具链、编译覆盖参数变化时拒绝混用旧缓存。缓存另记录当前 checkout 的绝对路径；
切换 checkout 时先确认缓存没有被使用，再仅清理本地 workspace package 的旧产物，保留 registry
公共依赖。跨 checkout 调用 `prepare_cache` 须显式传入 `workspace_root`；同一 checkout 不重复清理。
这避免 Cargo 误用另一 checkout 中同名 path package 的旧产物。普通源码更新由 Cargo 按依赖重建；旧缓存并非
按文件日期删除。完成一批组件、升级公共基础或磁盘盘点发现增长时，结束所有轮次后执行：

```sh
python3 scripts/component-review.py cache-reset
```

仅删除本工具标记的公共缓存，下一轮重建当前版本，不保留多套“公共基础版本”。
清理前检查进程打开的文件、可执行映射及 cwd；发现占用、路径符号链接或无法读取占用清单就拒绝。
新入口用同一文件锁串行化构建/清理；直接运行的 Cargo 也应使用这个配置并避免与清理并发。

共享契约变化或组件族结束时，创建带 `--integration` 的轮次，运行 `check <round> --gallery`
并保留完整 Gallery 的测试与真实接入证据。小宿主通过不能替代完整 Gallery 接入验收。
`python3 scripts/package-gallery.py` 默认也复用紧凑 dev 缓存；`--release` 才创建 release 构建。
它替换单个固定 Gallery App，拒绝替换正在使用的 App，不发布。

历史 `target` 中混有源码快照、截图、日志和 trace，不能 `rm -rf target`。
旧产物只能按盘点出的具体 Cargo profile 目录、进程占用和保护哈希清理；不按目录日期猜测。
独立源码重放仍须保留干净源码树以证明可复现，完成后只清理它自己的编译目录，保留源码与结果。

`plain` 是 #23 的既有文本适配器内容呈现场景，非新增组件；该测试 App 使用已有进程私有 pasteboard 验证复制，避免触碰用户总剪贴板。

### 小宿主自身窗口恢复

需要验证最小化／恢复时，优先在测试宿主提供仅操作自身窗口的 AppKit 生命周期命令。参考 `design/reviews/overlay-deactivation-20261020/host/src/main.rs` 的 Restore test window 与 Open then minimize test window：用 CUA 调用已声明的测试菜单，实际 AppKit 决定 Focus 和 `Window.is_active()`；不得赋值或伪造活动状态。生命周期命令与仍需拒绝的失活组件输入分别核验，开始交互前必须保存真实激活证据。

这是原生功能测试入口，不修改系统设置、不发送其他应用消息，不是物理 Dock／Command+Tab 或 VoiceOver 证据。开始过渡并立即最小化可避免依赖自动化两次调用之间的时序；不记录刷新率或性能指标。旧宿主与冻结证据保持原样，后续新宿主按需复用该测试接法。
