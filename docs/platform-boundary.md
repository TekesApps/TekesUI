# 平台层边界：tekes-ui-platform-macos

本文说明 `tekes-ui-platform-macos` 向引擎暴露的窄接口、事件约定、帧节奏与实时缩放的设计、重入处理、窗口摆放规则，以及 F0 阶段的验证记录。分层与边界原则见 [charter.md](charter.md) 第 4、8 节，阶段范围见 [roadmap.md](roadmap.md) F0。

原则：平台层负责窗口、事件泵、菜单、系统外观，向引擎投递规整的事件；引擎不接触任何 objc2 类型，也不依赖这个 crate 的内部。这个 crate 不依赖 `tekes-ui-render`，只通过一个原始的 CAMetalLayer 指针与渲染层衔接。

## 接口一览

```rust
let app = App::new(AppConfig::default());            // 只能在主线程
app.run(window_config, menu_bar, handler);           // 阻塞，直到应用结束；返回后处理器已 Drop

pub trait WindowHandler: 'static {
    fn handle(&mut self, event: Event, window: &mut Window);
}
```

- `AppConfig { name, quit_after_last_window_closed }`：应用名用于标准菜单标题；默认最后一个窗口关闭后 `run()` 返回。
- `WindowConfig { title, size, min_size, placement, frame_autosave_name }`：`size` 是内容区尺寸（pt）。`frame_autosave_name` 默认 `None`（关闭），Gallery 需要固定、可复现的窗口尺寸，不能被上次的拖动改变；设了名字时，有保存的位置就恢复，并跳过 `placement`。
- `MenuBar { menus: Vec<Menu> }`：见“菜单”一节。
- `Window`（只在 `handle` 里临时给出，不是 `Send`）：

| 方法 | 作用 |
|---|---|
| `request_redraw()` | 请求重绘，合并 |
| `set_title(&str)` | 设标题 |
| `size() -> Size` | 视图尺寸，pt |
| `scale() -> f64` | `backingScaleFactor` |
| `drawable_size() -> (u32, u32)` | CAMetalLayer 当前 drawableSize，像素 |
| `metal_layer() -> NonNull<c_void>` | CAMetalLayer 指针，给渲染层建 wgpu surface |
| `close()` | 关窗，不经过 `CloseRequested` |
| `set_content_size(Size)` | 改内容区尺寸 |
| `set_command_checked(CommandId, bool) -> bool` | 菜单勾选；命令不存在返回 false |
| `is_command_checked(CommandId)` | 读勾选状态 |
| `placement_report()` / `screen_info()` | 摆放结果与当前屏幕，日志用 |
| `native_window_number()` / `redraw_count()` | 诊断用 |

## 事件

```rust
pub enum Event {            // #[non_exhaustive]
    Created,
    Resized { size: Size, scale: f64, drawable_size: (u32, u32) },
    Redraw,
    Mouse(MouseEvent),      // Down/Up{button, click_count}、Moved、Dragged{button}、Entered、Exited
    Scroll(ScrollEvent),
    Key(KeyEvent),          // Down/Up，key_code、characters、characters_ignoring_modifiers、is_repeat
    ModifiersChanged(Modifiers),
    Focus(bool),
    Command(CommandId),
    SystemAppearanceChanged { dark: bool, increase_contrast: bool },
    CloseRequested,
}
```

- **坐标**：视图坐标，单位 pt，原点在左上角，y 向下。视图 `isFlipped = true`，由 AppKit 在 `convertPoint` 里翻转，平台层不手写 `height - y`；自测里合成的点击在窗口坐标 (100, contentHeight-50) 处，收到的位置是 (100, 50)，验证了这一点。
- **滚动**：`delta` 取自 `scrollingDeltaX/Y`，符号不改，已包含系统“自然滚动”设置；`delta.y > 0` 表示内容向 y 增大的方向移动。`precise` 为 true（触控板）时单位是点，为 false（普通滚轮）时单位是行，行高由引擎定。`phase` 与 `momentum_phase` 逐值转录 `NSEventPhase`，不解释、不模拟惯性（章程原则 4）：手指离开后系统继续发 `phase = None`、`momentum_phase = Began/Changed/Ended` 的事件。
- **键盘**：`key_code` 与布局无关；`characters` 在功能键、方向键上是 AppKit 的私有区字符。现在不调用 `interpretKeyEvents:`，输入法与死键不工作，F5 再做。未处理的按键不调 super，没有系统提示音。
- **鼠标进出与移动**：靠 `NSTrackingArea`（`MouseEnteredAndExited | MouseMoved | ActiveInActiveApp | InVisibleRect`），不要求窗口是 key window。窗口未激活时的第一次点击也送达视图（`acceptsFirstMouse`）。
- **Focus**：跟随窗口的 key 状态。
- **CloseRequested**：红色按钮和 ⌘W 都会来，平台层不替引擎决定，拒绝关闭并投递事件，引擎调用 `Window::close()` 才真关。不响应它的引擎窗口关不掉。⌘Q 目前直接结束应用，不经过 `CloseRequested`，将来需要“退出前确认”时再加 `QuitRequested`。
- **输入法预留**：`Event` 里没有 preedit 变体，也没有空壳实现。F5 增加时按章程第 4 节：`Preedit` 带组字串、分节区间（clause segments，含起止偏移和是否当前转换节）与组字内光标位置；`Commit` 带文本与 `replacementRange`；`firstRect`、`selectedRange`、`attributedSubstring` 做成 `WindowHandler` 上的同步查询方法，不走事件。位置见 `event.rs` 里 `Event` 的文档注释。

### 启动与结束的事件顺序

`applicationDidFinishLaunching` 里，在窗口显示之前同步投递：

1. `Created`：视图、CAMetalLayer 已创建，`metal_layer()` 可用；
2. `SystemAppearanceChanged`：初始值，无条件投递；
3. `Resized`：初始尺寸、缩放、drawable 尺寸；
4. `Redraw`：同步，所以窗口第一次出现时第一帧已经画好；

之后 `makeKeyAndOrderFront` 引起 `Focus(true)`。`run()` 返回之前处理器被 Drop；`terminate:`（⌘Q、最后一个窗口关闭）不会让 AppKit 调 `exit()`，而是停掉事件循环（`applicationShouldTerminate` 返回取消，同时 `stop:` 并补发一个空事件唤醒循环），所以 `run()` 之后调用方还能收尾。

## 帧节奏：按需，空闲时什么都不跑

**空闲状态没有 CVDisplayLink、CADisplayLink 或任何定时器。** 主线程停在 `mach_msg`，只有三类来源会让平台层投递 `Redraw`：

1. 引擎调用 `request_redraw()`；
2. 尺寸或缩放变化（同步，见下一节）；
3. 启动时的第一帧。

`request_redraw()` 的实现：用一个 `redraw_pending` 标志合并，第一次置位时用 `performSelectorOnMainThread:withObject:waitUntilDone:modes:`（common modes）往主 runloop 排一个一次性回调 `tekesPerformRedraw`；回调里清标志并投递 `Event::Redraw`。同一轮 runloop 里多次调用只产生一次 `Redraw`（自测：连续两次 `request_redraw` 只多一次 Redraw）。处理 `Redraw` 时再调用 `request_redraw()` 表示“下一帧还要画”。回调不依赖 AppKit 的显示周期，也不是定时器：没有排队的回调时 runloop 不会被唤醒。

### 为什么不用 `setNeedsDisplay` 加 `updateLayer`

最初的设计是 `setNeedsDisplay(true)`，由 AppKit 调 `updateLayer`。实测（macOS 26，Darwin 25.6）行不通：背衬层是 CAMetalLayer 时，`setNeedsDisplay:` 会被转成层的 `needsDisplay`，视图自己的标志立刻变回 false，但 CAMetalLayer 不执行 display，所以 `updateLayer` 一次也不会被调用，连同步的 `[view display]` 也不会。`wantsUpdateLayer = true` 和 `updateLayer` 的实现仍然保留（AppKit 万一因别的原因调用，照样投递 `Redraw`），但不能依赖它。

### 将来的 CADisplayLink

需要持续动画（滚动惯性、光标闪烁以外的动画、120Hz 验收）时，在 `Shared`（`shared.rs`）里加一个只在“有动画”时存在的 `CADisplayLink`（macOS 14 起 `NSView.displayLink(target:selector:)`，自动跟随窗口所在屏幕的刷新率）。它的回调直接调用 `dispatch(Event::Redraw)`；动画结束时 `invalidate`，回到按需状态。引擎侧的形式可以是 `Window::set_animating(bool)`。现在不实现。

另外，`Window` 不是 `Send`，`request_redraw` 只能在主线程调用。后台任务唤醒 UI 需要一个跨线程的代理（向主队列投递，再 `request_redraw`），也留到有需求时加。

## 实时缩放

- **尺寸变化同步投递**：`setFrameSize:`（缩放过程中 AppKit 每一步都会调）里，先调 super，再把层的 `drawableSize` 设成 `size * scale`（四舍五入，每维至少 1），然后同步投递 `Resized` 与 `Redraw`，不等下一轮 runloop。这样缩放过程中窗口与内容同一步更新，不会出现窗口已变而内容还是旧尺寸。
- **缩放因子变化**（`viewDidChangeBackingProperties`：换屏幕、改分辨率）与 `viewDidMoveToWindow` 走同一个 `sync_geometry`；层的 `contentsScale` 同步到 `window.backingScaleFactor`。几何信息（尺寸、缩放、drawable）与上次投递的相同则不重复投递。
- **drawableSize 归渲染器管**：wgpu 配置 surface 时设置它，平台层不碰（在渲染器接管之前，CAMetalLayer 自己按 `bounds * contentsScale` 算）。`Resized.drawable_size` 是平台层算出的像素尺寸（`size * scale` 取整），渲染层用它调用 `Renderer::resize`。`contentsScale` 归平台层管，随 `backingScaleFactor` 同步。
- **色彩空间**：层的 `colorspace` 设为 sRGB，渲染层在 gamma 空间混合，与 AppKit 一致，不让系统再做一次色域转换。层 `opaque = true`，视图 `isOpaque = true`。
- **`layerContentsRedrawPolicy = OnSetNeedsDisplay`**：不让 AppKit 在缩放时自己重绘层内容。默认的 `DuringViewResize` 会在我们的同步重绘之后再触发一次，同一次缩放画两帧，还会与渲染层写入的 drawable 冲突。注意 `setWantsLayer(true)` 会把策略重置成 `Never`，必须在它之后设置（实测）。
- **`layerContentsPlacement = TopLeft`**：新 drawable 还没来时，旧内容按原尺寸钉在左上角，不拉伸，也不居中漂移；窗口变大时露出的是层的背景色。不能直接设层的 `contentsGravity`，AppKit 管着背衬层，会按视图的 `layerContentsPlacement` 改写它（实测：直接设的值被改回 `resize`）；视图设 `TopLeft` 后，AppKit 把层的 gravity 设成 `bottomLeft`（因为层是 geometry-flipped，两者是同一个视觉位置）。验证：自测给层塞一张 200x200 像素（2x 下 100pt）上红下蓝的图，窗口放大后截图，图仍在左上角、大小不变、上红下蓝，没有拉伸。
- **`presentsWithTransaction`**：平时关闭；`viewWillStartLiveResize` 时自动打开，`viewDidEndLiveResize` 时关闭，不是公开 API。wgpu 在每次取 drawable 时读取它，为 true 时改用 `commit + waitUntilScheduled + present`，实时缩放时内容与窗口边框在同一个 Core Animation 事务里更新。平台层的同步 `Resized` + `Redraw` 就在 `setFrameSize:` 里、主线程上完成，满足这个路径的要求。程序化缩放（`setContentSize`）不算实时缩放，不开。

## 重入

AppKit 常常在我们调用它的过程中同步回调回来：引擎在 `handle` 里调 `set_content_size` 时，AppKit 立刻调 `setFrameSize:`；调 `close` 时立刻调 `windowWillClose:`。回调里又要投递事件，如果直接再调一次 `handle`，同一个 `&mut handler` 就有了两个可变引用，对 `RefCell` 则是二次借用而 panic。做法（见 `shared.rs`）：

1. 所有 `RefCell` 的借用都不跨越外部调用（AppKit 调用、`handle`），取出、放回都在一条语句里完成；
2. 处理器“取走再放回”：分发期间 `handler` 槽是 `None`；
3. `dispatching` 标志表示有一个分发循环在运行。重入的 `dispatch` 只把事件放进队列就返回，由外层循环在当前 `handle` 返回后按顺序投递，任何时刻最多一个 `handle` 在运行，事件顺序不变；
4. 队列里相邻重复的 `Redraw` 合并；
5. `Window` 只是一组 `Retained` 引用和一个 `Rc<Shared>`，每次分发时临时构造，不持有任何借用，`handle` 里调它的任何方法都不会二次借用。

验证：示例的 `handle` 入口有 `assert!(!busy)`，菜单项 “Grow Window (reentrancy)” 在 `handle` 里调用 `set_content_size`，日志显示 `set_content_size` 先返回，随后才依次收到排队的 `Resized` 与 `Redraw`，没有嵌套；示例在 `Redraw` 里每次 `set_title` 也没有问题。处理器 panic 会穿过 AppKit 的 C 栈帧，objc2 会直接终止进程，不做恢复。

## 菜单

引擎提供纯数据的 `MenuBar`：菜单 → 项，项有标题、快捷键（`KeyEquivalent { key, modifiers }`）、`CommandId(u32)`、初始勾选状态，以及分隔线。`MenuBar::validate()` 检查标题非空、命令编号唯一，`run()` 对不合法的描述 panic。

平台层自动生成并安装：

- 应用菜单：About、Hide、Hide Others（⌥⌘H）、Show All、Quit（⌘Q）；
- 引擎的菜单（按给定顺序）；
- Window 菜单：Minimize（⌘M）、Zoom、Close（⌘W）、Enter Full Screen（⌃⌘F），并设为 `windowsMenu`，窗口列表由 AppKit 追加。

标准项使用 AppKit 标准 selector，target 为 nil，沿响应链处理（`terminate:`、`performClose:`、`toggleFullScreen:` 等），不需要引擎参与。引擎的命令项统一指向委托的 `menuItemSelected:`，`tag` 携带 `CommandId`，选中时投递 `Event::Command`。勾选状态用 `Window::set_command_checked` 修改（遍历主菜单按 tag 查找），Gallery 用它做 light/dark/increase contrast 的单选勾选：选中一个时由引擎自己取消其他项。暂无启用/禁用状态。

## 系统外观

`SystemAppearanceChanged { dark, increase_contrast }` 在启动时无条件投递一次，之后只在值变化时投递：

- 深浅色取视图的 `effectiveAppearance`（`bestMatchFromAppearancesWithNames([Aqua, DarkAqua])`），变化时 AppKit 调 `viewDidChangeEffectiveAppearance`，所以窗口或应用单独设置的外观覆盖也会反映；
- 增强对比度取 `NSWorkspace.accessibilityDisplayShouldIncreaseContrast`，变化由 `NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification` 通知（委托作 observer）。

## 窗口摆放

开发规则（用户规则，适用于示例、测试窗口和 Gallery）：

1. 有外接显示器，放外接显示器，除非检查需要 120Hz（ProMotion）；需要 120Hz 时放内建显示器；
2. 没有外接显示器、且检查与窗口尺寸无关时，放在内建显示器的右半边；
3. 缩放、窗口尺寸类检查免除右半边规则（居中）。

实现在 `placement.rs`（纯逻辑，可单测）与 `screens.rs`（枚举 `NSScreen.screens`）：

```rust
pub enum WindowPlacement {
    Centered,                                          // AppKit center()，默认
    Auto { need_120hz: bool, size_check: bool },       // 按上面的规则
    RightHalf(ScreenChoice),                           // 指定屏幕的右半边
    Screen(ScreenChoice),                              // 指定屏幕居中
}
pub enum ScreenChoice { Builtin, External }            // 缺外接屏时 External 退回 Builtin
```

- 内建屏判定：`CGDisplayIsBuiltin`（取自屏幕 `deviceDescription["NSScreenNumber"]`）；`ScreenInfo` 带名称、是否内建、缩放、`maximumFramesPerSecond`、frame 与可用区域。
- “右半边”指内建屏可用区域（扣掉菜单栏与 Dock）的右半边，窗口在其中居中；窗口比半边宽时靠右对齐，右边缘不出屏；窗口比屏幕高时保证标题栏可见。
- 理解说明：规则 1 里“需要 120Hz 放内建屏”之后是否再取右半边，原文没有说；这里的做法是：落在内建屏就套用规则 2/3（右半边，尺寸类检查居中），因为同样要避免盖住左边的工作窗口。
- 环境变量 `TEKESUI_WINDOW_PLACEMENT` 覆盖配置：`auto`（保留配置里 `Auto` 的两个标志，配置不是 `Auto` 时都为 false）、`external`、`builtin`、`builtin-right-half`、`centered`。不认识的值打印警告后忽略。
- 库的默认值是 `Centered`，不替产品应用做摆放；示例和 Gallery 在配置里写 `Auto`。
- 日志：`Window::placement_report()` 给出选中的屏幕（名称、是否内建、缩放、最大帧率）、布局、原因与全部屏幕；`Window::screen_info()` 给出窗口当前所在屏幕。示例在 `Created` 时打印。

## 窗口遮挡

`windowDidChangeOcclusionState:`：窗口变为可见（最小化后恢复、被完全盖住后露出）时调用 `request_redraw()`。不可见期间渲染层取 drawable 会得到 Occluded，跳过帧并返回 Ok，不补这一帧就会停在旧画面。

## 与渲染层集成要点

- 渲染层从 `Window::metal_layer()` 得到 CAMetalLayer 指针，用 `SurfaceTargetUnsafe::CoreAnimationLayer` 建 surface。指针在窗口存活期间有效，渲染层不要 release 它。
- `Created` 时层已存在、`drawableSize` 已设好；`Resized` 在 `Redraw` 之前，每次 `Resized` 后必然紧跟一次同步 `Redraw`。渲染层应在 `Resized` 里重新 `configure`，在 `Redraw` 里 acquire、绘制、present。
- 平台层设置 `contentsScale`、视图的 `layerContentsPlacement`（层 gravity 由 AppKit 据此生成）、`opaque`；渲染层（wgpu 的 configure）设置 `drawableSize`、`pixelFormat`、`framebufferOnly`、`device`、`maximumDrawableCount`、`displaySyncEnabled`，并在每次 configure 之后重新把 `colorspace` 设成 sRGB（wgpu 会把它重置为 nil；平台层建层时设的 sRGB 因此会被覆盖，以渲染层为准）。
- `presentsWithTransaction` 由平台层在实时缩放期间自动开关，渲染层不用管，见“实时缩放”。
- 窗口重新可见时平台层会 `request_redraw()`，见“窗口遮挡”。
- 引擎的处理器 `Redraw` 里发现还要画下一帧时，调用 `request_redraw()`。

## 验证记录（F0）

环境：MacBook 内建 Liquid Retina XDR（1512x982 pt，2x，120Hz）加一块外接 4K 显示器 U2790B（1920x1080 pt，2x，系统报 30Hz）。按摆放规则，示例默认落在外接屏。

自测：`TEKES_SELFTEST=1 cargo run -p tekes-ui-platform-macos --example window`（`hold` 取值则做完保留窗口）。示例从后台线程按时间表向主队列投递步骤，步骤里直接操作 AppKit（合成鼠标、键盘、滚轮 NSEvent，菜单 `sendAction`，`setContentSize`，窗口 `setAppearance`，`performClose`），最后正常退出，退出码 0。该自测覆盖：

- 点击位置翻转、双击 `click_count = 2`、拖动、键盘字符与 key_code、⌘1 触发 `Command`；
- 滚轮 `phase`/`momentum_phase` 逐值转录（Began/Changed/Ended，momentum Began/Changed/Ended）；
- 菜单命令、连续两次 `request_redraw` 合并为一次；
- 缩放：22 步 `set_content_size`，每步一个 `Resized` 紧跟一个 `Redraw`，drawable 尺寸正确；
- 窗口外观切换产生 `SystemAppearanceChanged`；
- `handle` 内改窗口尺寸的重入；
- 红色按钮路径 `performClose` → `CloseRequested` → `close()` → `run()` 返回；
- 层的滞留图像不拉伸（截图）。

空闲（`scripts/measure-idle-cpu.sh <pid> [秒数]`，基于 `top -l N -s 1 -pid`，丢弃首个样本）：启动稳定后 30 个样本，CPU 每秒均为 0.0%，中位数 0.0%，最大 0.0%，空闲唤醒合计 0；同一时段示例日志里 Redraw 计数保持 27 不变。`sample <pid> 5`：主线程的全部样本都在 `mach_msg`（`NSApplication run` 里），另一个线程是 `NSEventThread`，同样停在 `mach_msg`。

单元测试：修饰键转换（含对照 objc2-app-kit 常量）、鼠标键号、滚动阶段、drawable 尺寸取整与下限、菜单描述校验、摆放规则与坐标计算。

未验证：

- **1x 屏幕**：没有 1x 显示器，Retina 以外的缩放因子路径（`contentsScale = 1`、拖动窗口跨 1x/2x 屏幕触发 `viewDidChangeBackingProperties`）未测。两块屏幕都是 2x。
- **增强对比度的变化通知**：改它是修改系统设置，没有测；启动时的读取路径已验证（`false`），变化路径只由代码保证。深浅色变化用窗口级外观覆盖验证了视图回调，没有切换系统外观。
- **真实触控板滚动与真实鼠标输入**：滚轮用合成事件验证；真实鼠标的移动、进出事件在示例运行时有过（窗口收到过 Entered/Exited），没有系统化验证。
- **实时缩放的视觉效果**（用鼠标拖边框时有没有撕裂或抖动）：用 `setContentSize` 序列验证了事件序，没有对着真实 wgpu 呈现看；`viewWillStartLiveResize` 没有接入。
- **多窗口**：当前一个 `run()` 一个窗口。
- **⌘Q 之前的确认**、菜单项启用/禁用、剪贴板、输入法：未做。
- **手动拖动、真实键盘快捷键**：自测通过 `NSApp.sendEvent` 合成，与真实输入走同一套 AppKit 响应链，但没有用 USB/辅助功能真实输入验证。
- 提示：自测的步骤间隔由后台线程的 sleep 决定，窗口被遮挡或后台时 App Nap 可能把它拖慢，日志里的时间戳会偏大，步骤顺序不变。
