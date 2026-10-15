# F0 门禁测量记录

测量日期：2026-10-15。对象：`tekes-ui-gallery` 最小宿主（平台层 `tekes-ui-platform-macos` 加渲染层 `tekes-ui-render`，真实 wgpu 渲染到 CAMetalLayer），release 构建。测量时的提交是 09183af（Gallery 宿主的主体从 ae5f86d 起；09183af 只多了 `TEKES_LOG_EVENTS=all` 的日志与脚本，不影响运行路径；第一组空闲测量用的是 09183af 之前的二进制，此后只加了日志）。

对应 roadmap F0 的通过标准：窗口可开关缩放、菜单可用、空闲时不重绘（0 CPU）。本文只覆盖“窗口、缩放、空闲、Retina”几项，Gallery 的 light/dark/increase contrast 切换页与参照图槽位还没做（见 handoff 的后续工作）。

## 1. 机器

| 项 | 值 |
|---|---|
| 机型 | Mac16,1（`sysctl hw.model`） |
| 芯片 | Apple M4 |
| 系统 | macOS 26.6.2（25G83） |
| 工具链 | rustc 1.95.0，wgpu 30.0.1，objc2 0.6.5 |
| 显示器 1 | 内建 Liquid Retina XDR：3024x1964 像素，1512x982 pt，2x，`maximumFramesPerSecond` = 120（ProMotion） |
| 显示器 2 | 外接 U2790B：3840x2160 像素，缩放后 1920x1080 pt，2x，系统报 30Hz（`maximumFramesPerSecond` = 30） |

按窗口摆放规则（外接屏优先），除非特别注明，所有测量的窗口都在外接屏上。窗口缩放的截图测量在两块屏幕上各做了一次。机器上同时有别的会话在工作（编译、截图），第 2 节会说明它对结果的影响。

## 2. 空闲 CPU

### 方法

- 内容：Gallery 占位页，静止，无动画；窗口可见，位于外接屏，启动后成为 key window。
- 每次：启动 → 等 SETTLE 秒 → `scripts/measure-idle-cpu.sh <pid> 60`（`top -l 61 -s 1 -pid <pid> -stats pid,cpu,idlew`，丢弃第一个样本，因为它是进程启动以来的平均值）→ 结束进程。comparison-method 第 2.3 节要求 60 秒、至少 5 次；这里共 12 次，分三组：
  - 第一组 5 次，SETTLE = 5 秒；
  - 第二组 3 次，SETTLE = 30 秒；
  - 第三组 4 次，SETTLE = 15 秒，`TEKES_LOG_EVENTS=all`（记录包括鼠标、键盘、焦点在内的全部事件）。
- 重绘计数：进程日志里 `Redraw #` 的行数，在量 CPU 之前和之后各数一次。
- 线程：第一次运行另外用 `sample <pid> 5` 看所有线程的栈顶。

### 复现命令

```bash
cargo build --release -p tekes-ui-gallery
scripts/f0-idle-gate.sh 5 60 /path/to/out                      # 第一组
SETTLE=30 scripts/f0-idle-gate.sh 3 60 /path/to/out            # 第二组
LOG_EVENTS=all SETTLE=15 scripts/f0-idle-gate.sh 4 60 /path/to/out   # 第三组
```

脚本每次输出一行汇总，`idle-runN.cpu.txt` 是逐秒原始数据，`idle-runN.log` 是进程日志，`idle-sample.txt` 是 `sample` 输出。单独测一个已在运行的进程：`scripts/measure-idle-cpu.sh <pid> 60`。

### 原始数据（每次 60 个样本）

| 组 | 次 | CPU 中位数 | CPU 最大值 | 空闲唤醒合计 | Redraw 计数（前 → 后） |
|---|---|---|---|---|---|
| 1 | 1 | 0.0% | 1.5% | 0 | 2 → 2 |
| 1 | 2 | 0.0% | 0.1% | 0 | 2 → 2 |
| 1 | 3 | 0.0% | 0.0% | 0 | 2 → 2 |
| 1 | 4 | 0.0% | 0.1% | 0 | 2 → 2 |
| 1 | 5 | 0.0% | 0.3% | 0 | 2 → 2 |
| 2 | 1 | 0.0% | 0.0% | 0 | 2 → 2 |
| 2 | 2 | 0.0% | 0.0% | 300（每秒恒为 5） | 2 → 2 |
| 2 | 3 | 0.0% | 0.0% | 0 | 2 → 2 |
| 3 | 1 | 0.0% | 0.0% | 0 | 2 → 2 |
| 3 | 2 | 0.0% | 0.0% | 0 | 2 → 2 |
| 3 | 3 | 0.0% | 0.0% | 0 | 2 → 2 |
| 3 | 4 | 0.0% | 1.5% | 22 | 2 → 2 |

汇总：12 次的中位数全部是 0.0%；12 次里 9 次最大值 ≤ 0.1%。

重绘计数两次都是 2：#1 是启动时的同步重绘，#2 是窗口第一次可见时平台层因遮挡状态变化补的一帧（见 platform-boundary.md“窗口遮挡”），都发生在量 CPU 之前。12 次里量 CPU 的 60 秒内 Redraw 计数没有任何增加；第三组 4 次的日志记录了全部事件，量 CPU 期间没有任何鼠标、键盘或其他事件。

### 对异常点的说明（没有放宽门槛）

门槛是“0，允许 < 0.1% 的采样噪声，不得有定时器空转或持续 redraw”。严格按“最大值 < 0.1%”，12 次里有 3 次（第一组第 1、5 次，第三组第 4 次）最大值超过，还有 1 次（第二组第 2 次）CPU 为 0 但有每秒 5 次的唤醒：

- 超标的都是**孤立的单秒尖峰**，不是持续占用：第一组第 1 次是第 9、10 秒各一个（1.5%、0.4%）；第一组第 5 次在前 11 秒里有几个 0.1%–0.3%；第三组第 4 次在第 49–53 秒有 0.7%、1.0%、0.1%、1.5%。第三组第 4 次的日志里，量 CPU 末尾（约 75.9 秒）有一个 `Focus(false)`，即窗口被别的应用抢走焦点，AppKit 在处理前后台切换，这很可能是这次尖峰的原因（机器上同时有别的会话在工作）。其余几次没有记录到事件，原因未查明。
- 第二组第 2 次：每秒 5 次空闲唤醒，持续整整 60 秒，CPU 显示 0.0%，Redraw 计数没变，没有记录事件（这一次没开全部事件日志）。之后又做了 4 次短跑加第三组 4 次，没有再出现。原因未查明，可能来自进程外部（别的会话在同一时段操作机器），也可能来自 AppKit/Metal 的内部定时器；没有证据表明是我们的代码，我们没有注册任何定时器或 display link。
- 所以结论是：**中位数 0 与重绘计数不变两项通过；“最大值 < 0.1%”这一项在 12 次里有 3 次不满足，且有 1 次未解释的 5Hz 唤醒**。建议在一台没有其他会话干扰的机器上重跑，或者用 `sample` 在出现唤醒的那一刻抓栈。我没有据此改动门槛。

### 线程

第一次运行的 `sample <pid> 5`（4454 个采样点）：

| 线程 | 栈顶 |
|---|---|
| 主线程（`com.apple.main-thread`） | `mach_msg2_trap`，4454/4454，在 `NSApplication run` 里 |
| `com.apple.NSEventThread` | `mach_msg2_trap`，4454/4454 |
| 另有 6 个无名线程（libdispatch 工作队列） | 全部在 `__workq_kernreturn`（空闲的线程池线程）；其中 3 个只在部分采样点上存在（1170、1170、745 个采样点），是线程池在退役线程 |

没有 wgpu 或 Metal 自己创建的有名字的线程，没有线程在自旋；所有线程都停在内核等待里。

## 3. Retina：drawable 尺寸与线宽

### drawable == round(pt × scale)

Gallery 日志（`Resized` 事件的值），外接屏（scale 2）与内建屏（scale 2）一致：

| 视图尺寸（pt） | drawable（像素） |
|---|---|
| 1200x800 | 2400x1600 |
| 800x600 | 1600x1200 |
| 1000x520 | 2000x1040 |
| 1500x900（外接屏） | 3000x1800 |
| 1500x827（内建屏；请求 1500x900 被屏幕可用高度限制） | 3000x1654 |

全部等于 pt 的两倍。截图的像素尺寸与 drawable 一致（宽度逐项对上，高度 = drawable 高 + 64 像素标题栏）。

### 线宽（设备像素）

占位页里有一组验证条：1pt 与 0.5pt 的横线、竖线，位置按逻辑点对齐（2x 下 1pt = 2 像素、0.5pt = 1 像素）。用 `screencapture -o -x -l <窗口号>`（`-o` 不含阴影，截图是设备像素）截取窗口，`scripts/analyze-capture.py` 用标准库解 PNG，沿一列/一行数连续的非底色像素：

- 1pt 横线、竖线：2 像素宽；
- 0.5pt 横线、竖线：1 像素宽；
- 两种线的像素值相同（外接屏 (33,33,33)，内建屏 (37,37,37)；截图里是显示器色彩空间转换后的值，所以两块屏数值不同），没有半透明的过渡像素，说明边缘与像素网格对齐、不糊；
- 外接屏和内建屏都通过。

**1x 屏幕没有测**：这台机器的两块屏都是 2x。`scale = 1` 的路径（`contentsScale = 1`、拖动窗口跨 1x/2x 屏幕触发 `viewDidChangeBackingProperties`）未验证。

## 4. 缩放与遮挡（自测模式）

### 方法与复现

```bash
cargo build --release -p tekes-ui-gallery
mkdir -p /path/to/out
TEKES_SELFTEST=1 TEKES_SELFTEST_OUT=/path/to/out target/release/tekes-ui-gallery
# 内建屏重复一次：
TEKESUI_WINDOW_PLACEMENT=builtin TEKES_SELFTEST=1 TEKES_SELFTEST_OUT=/path/to/out2 target/release/tekes-ui-gallery
# 分析一张截图（内容区尺寸 pt）：
scripts/analyze-capture.py /path/to/out/s1_800x600.png 800 600
```

自测由后台线程按时间表向主队列投递步骤（`tekes_ui_platform_macos::selftest`），步骤里调 AppKit 的 `setContentSize`、`miniaturize`/`deminiaturize`、`performClose`，并在后台线程里用 `screencapture` 截图；不合成任何全局鼠标事件。时间表：初始截图 → 依次缩放到 800x600、1000x520、1500x900、1200x800，每次缩放 0.7 秒后截图 → 最小化 → 等 2 秒 → 恢复 → 等 1.4 秒后截图 → `performClose` → 正常退出（退出码 0）。

### 结果

`scripts/analyze-capture.py` 对每张截图检查：宽度、标题栏高度（64 像素 = 32 pt）、左上标记块（20,20 起 40x40 pt）与右下标记块（距右下角各 20pt）的包围盒（换算成 pt）、线宽。右下标记块在新尺寸下贴着新的右下角，说明内容按新尺寸排版，没有停在旧尺寸，也没有拉伸；左上块始终是 40x40 pt，没有缩放变形。

外接屏：6 张截图（初始、四次缩放、恢复后）共 48 项检查全部通过。例如 800x600：右下块包围盒 (740,540)–(780,580) pt，预期相同；1500x900：(1440,840)–(1480,880) pt，预期相同。

内建屏：同样的 6 张截图通过；其中 1500x900 的请求被屏幕可用高度限制成了 1500x827（`Resized 1500x827`，drawable 3000x1654），按 1500x827 分析通过（右下块 (1440,767)–(1480,807) pt）。

每次缩放恰好一个 `Resized` 紧跟一个同步 `Redraw`（日志中 Redraw #3–#6 对应四次缩放）。

### 遮挡

最小化再恢复：最小化期间没有 Redraw；恢复时平台层在 `windowDidChangeOcclusionState:` 里请求重绘，日志里恢复时刻出现 `Redraw #7`；恢复后的截图（s5）内容完整，各项检查通过。注意：这个检查能证明“恢复时确实触发了重绘、内容在”，但分不开“内容本来就留在层里”与“重绘重新画出来”，因为 CAMetalLayer 会保留最后一帧；wgpu 在遮挡期间返回 `Occluded` 的路径没有单独构造场景去验证（例如遮挡期间改变内容）。

## 5. 未测与手动检查项

- **鼠标拖边框的实时缩放**：不能在不接管用户鼠标的前提下自动化，没有合成全局鼠标事件。需要手动检查：拖动窗口边框或角时，内容与窗口边框同步、没有撕裂、没有拉伸的旧图像、右下角不抖。这条路径依赖 `viewWillStartLiveResize`/`viewDidEndLiveResize` 里自动开关的 `presentsWithTransaction`（wgpu 为 true 时用 `commit + waitUntilScheduled + present`）；程序化缩放（`setContentSize`）不进入实时缩放，所以这条路径目前**完全没有被执行过**。检查时留意两点：缩放过程是否卡顿（`waitUntilScheduled` 在 120Hz 内建屏上的开销），以及缩放结束后是否恢复正常。
- **1x 屏幕**：见第 3 节。
- **120Hz 内建屏上的帧节奏**：本阶段没有动画，没有测 120Hz 下的持续渲染；F1 的“120Hz 下滚动或动画不掉帧”需要 `CADisplayLink`（platform-boundary.md 里记了接入点），还没做。
- **增强对比度的变化通知**：改它要动系统设置，没有测，只验证了启动读取。
- **空闲 CPU 的干净机器复测**：见第 2 节，建议在没有其他会话干扰的机器上重跑；第二组第 2 次的 5Hz 唤醒未查明。
- **真实触控板与真实键鼠输入**：没有测，自测用 AppKit 入口合成事件，见 platform-boundary.md。
- **多窗口、菜单的真实点击**：菜单命令通过 `sendAction` 验证，没有真实鼠标点菜单。
- **Gallery 本身**：这一阶段只有最小宿主，light/dark/increase contrast 切换页、并排参照图槽位、导航都没做，不属于本文的测量范围。

## 6. Gallery 骨架实测（真实 Gallery，空闲与交互）

对象：带固定布局、目录、两页（Primitives、Token Swatches）、Pages 与 Appearance 菜单的 Gallery（用法见 [../gallery.md](../gallery.md)），release 构建，窗口摆放 Auto：外接屏 U2790B（外接屏优先）。时间：2026-10-15。

### 交互自测（`TEKES_SELFTEST=gallery`）

脚本通过 AppKit 入口触发菜单命令（`sendAction`）、合成窗口内的 NSEvent（点击经 `sendEvent`，鼠标移动与移出直接送内容视图），不合成全局事件。日志核对结果：

- 菜单切页、切外观、切对比度、方向键、点击目录，每次状态变化恰好一次 Redraw，窗口标题随之变成 `TekesUI Gallery — <页> — <主题>`；
- 重复动作不重绘：到头后再按 ↑ 没有 Redraw；
- hover：鼠标进入第 0 行重绘一次（#15），在同一行内再移动三次零次重绘，移到第 1 行重绘一次，移出重绘一次；
- 跟随系统：把窗口外观设为深色（`NSWindow.appearance`，不碰系统设置）后收到 `SystemAppearanceChanged`，主题变深并重绘；设回后变浅。固定为浅/深时，系统外观变化不影响。
- 截图（`screencapture -o -l`）13 张逐张目测：两页在 light、dark、light+IC、dark+IC 下都明显不同，hover/选中底色、状态矩阵五列、参照槽三个窗格、27 个色块都在位。

### 空闲门禁（真实 Gallery）

`LOG_EVENTS=all scripts/f0-idle-gate.sh 3 60 <目录>`，窗口可见、静止，SETTLE 5 秒：

| 次 | CPU 中位数 | CPU 最大值 | 空闲唤醒合计 | Redraw 计数（前 → 后） |
|---|---|---|---|---|
| 1 | 0.0% | 0.0% | 0 | 2 → 2 |
| 2 | 0.0% | 0.0% | 0 | 2 → 2 |
| 3 | 0.0% | 0.1% | 0 | 2 → 2 |

3 次的中位数都是 0.0%，最大值 0.0%、0.0%、0.1%（3 次的中位数的中位数 0.0%，最大值的最大 0.1%）；Redraw 计数没有变化。只跑了 3 次（任务要求至少 3 次），没有重复第 2 节的 12 次规模；机器上同时有别的会话时的尖峰这次没有出现。

### 未验证

- 菜单没有被真实鼠标点过（用 `sendAction`）；快捷键没有用真实键盘按过，只测了菜单项的 action。
- 窗口大于或小于 1200x800 时内容不重排、右下裁切：由布局不依赖窗口尺寸的单测和代码保证，这一版没有再做缩放截图。
- 内建屏（120Hz）和 1x 屏上的 Gallery 没有跑。
- 快照只在本机 GPU 上验证了逐字节可复现，没有跨机器比较。

