# TekesUI 章程

> 2026-10-15 最新设计权威：用户提供的 v0.3.0-proposal1 有定义时优先，缺项保留现有补集。参见 [DESIGN.md](../DESIGN.md) 和完整导入索引；本文此前固定尺寸、旧值和原版优先约定仅在不冲突时适用。规范存在不等于原生实现已验收。


> 外部库版本均为 2026-10 调研时状态，开工前复核：wgpu 30、Parley 0.12、objc2 0.6、AccessKit 2026-09 版本、winit 0.31 beta。

## 1. 定位

TekesUI 是纯 Rust 的 UI foundation 与 components 组件库。它与现有 Swift 栈（TekesMetalTextKit，以及 TekesClientKit 的 UI 层）平行，是一个长期项目：慢慢做、做扎实，不赶产品节点。

- 产出就是组件，不绑定具体产品；Tekes agent 客户端、TekesDXF、TekesDrama 将来是它的使用者，而不是它的需求来源。
- 质量靠与现有 TekesMetalTextKit 的效果对比来保证，方法见 [comparison-method.md](comparison-method.md)。
- 仓库以 MIT 许可公开。

### 背景

Agent 内核 TekesKernel、CAD 引擎、DramaKit 的核心已是 Rust，与 Swift 通过进程加协议通信，没有 FFI。UI 是剩下最大的 Swift 部分。TekesMetalTextKit（约 6.7 万行，Metal + CoreText + TextKit 的文本渲染与文档组件库）积累了大量经验，TekesUI 借用它的思路，而不是代码。

### 已否决的路线

- GPUI：没有稳定版；Windows/Linux 的中文输入法有未修 bug；Zed 的 editor、terminal、ui 几个 crate 是 GPL。
- winit 作为 mac 平台层：它的 NSTextInputClient 只是最小实现，撑不住 CJK 输入法。

## 2. Gallery：恒定的人工目测验收目标

Gallery 是一个类似 Storybook 的 Rust 应用（`tekes-ui-gallery`），是所有组件唯一的人工目测验收入口。

- 每个组件一个页面，使用固定夹具、固定布局，同一个页面每次长得一样，才能比较。
- 每页覆盖状态矩阵：hover、pressed、disabled、focus、light/dark、increase contrast。
- 有参照图（来自 Swift 实现或设计稿）就在同一页并排展示。
- 每次确认都看同一个 Gallery，不再临时弹出一堆没有排版、看不出所以然的 view。
- 自动化对比（截图差分、性能、输入法矩阵）与 Gallery 共用同一批夹具，见 [roadmap.md](roadmap.md) 各阶段的 Gallery 页面要求。

## 3. 终态

TekesUI 本仓库保持纯 Rust 实现，不引入 Swift 源码、Swift package 构建或 Swift 启动/运行垫片。macOS 平台能力通过 Rust 可调用的接口接入，例如 objc2 与 security-framework crate；无法这样接入的能力单独评估并推迟，不预设 `tekes-apple-shim`，也不承诺 StoreKit 2 已可纯 Rust 接入。

现有 Kit 源码只读参考，compare 仓库只用于对比证据，不作为本仓库构建或运行依赖。用户批准的原版视觉基线与后续变更边界以根目录 [DESIGN.md](../DESIGN.md) 为准，阶段扩展不能自行推翻现有外观。

## 4. crate 分层

```
                 tekes-ui-gallery (app)
                         |
   +---------------------+----------------------+
   |                     |                      |
tekes-ui-edit -----> tekes-ui-doc          tekes-ui-a11y
   |                     |                      |
   +--------> tekes-ui-layout <-----------------+
                         |
                  tekes-ui-scene  <---- tekes-ui-text
                         |                   |
                  tekes-ui-render      (CoreText / DirectWrite / Parley)
                         |
                   (wgpu / objc2-metal)

tekes-ui-tokens  <-- 被 layout、scene 及控件层读取

tekes-ui-platform-macos  (以后：tekes-ui-platform-windows)
   窗口 / 事件 / 菜单 / 输入法，经窄接口与引擎相连
```

依赖方向自上而下；platform 层只通过窄接口与引擎通信，引擎不依赖任何具体平台。

### 各 crate 职责

- **tekes-ui-tokens**：设计 token（颜色、间距、字号、圆角、动效），含 light/dark/increase contrast 变体。是否自有 token 取决于 [icons-and-tokens-brief.md](design/icons-and-tokens-brief.md) 的结论。
- **tekes-ui-text**：三个 trait：`Shaper`、`Rasterizer`、`FontFallback`。mac 用 CoreText 实现，以后 Windows 用 DirectWrite，Linux 用 Parley。
- **tekes-ui-scene**：display list 加 glyph atlas。上层把内容编译成绘制指令，这一层不关心 GPU API。
- **tekes-ui-render**：基于 wgpu，接口保持窄，留出换成 objc2-metal 的余地（例如 mac 上需要更直接地控制 CAMetalLayer 和帧节奏时）。
- **tekes-ui-doc**：rope 文档模型、节点词汇、Markdown、diff。
- **tekes-ui-layout**：文档类内容用虚拟布局；界面 chrome 用 taffy。
- **tekes-ui-edit**：编辑状态机、组字（preedit）状态、undo；断行用 icu_segmenter。
- **tekes-ui-a11y**：基于 AccessKit，把控件树和文档结构映射成无障碍树。
- **tekes-ui-platform-macos**：基于 objc2，实现 NSApplication、窗口、菜单，以及完整的 NSTextInputClient（selectedRange、attributedSubstring、replacementRange、firstRect）。
- **tekes-ui-platform-windows**（以后）：Win32 加 TSF。
- **tekes-ui-gallery**：Gallery 应用，见第 2 节。

### 引擎与平台层的边界

保持窄，参考 Ghostty 的 apprt 模式：平台层负责窗口、事件泵、输入法、剪贴板、菜单，向引擎投递规整的事件，引擎向平台层请求"设置光标矩形""开始拖放"之类的少量能力。有一点要特别设计：preedit 事件必须带上分节区间（clause segments）和组字内的光标位置，否则 CJK 输入法的下划线分节和候选窗定位做不对。

## 5. 原则

1. 不依赖 GPUI，不复制任何 GPL 代码。Zed 的 editor、terminal、ui 几个 crate 是 GPL；GPUI 本体和 warpui（MIT）只作阅读参考。
2. mac 上排版、字体回退、光栅化都用 CoreText，因为 macOS 15 苹方的 hvgl 字形表 FreeType 读不了。
3. mac 平台层不用 winit，它的 NSTextInputClient 是最小实现。
4. mac 惯性滚动使用系统的 momentum phase，不自己模拟。
5. 先对照、后抽象：每个组件先在 Gallery 里与参照实现并排达标，再考虑泛化。
6. 不设日期，按通过标准推进。

## 6. 两张清单

### 从 TekesMetalTextKit 借用的思路（只借思路，不搬代码）

- 实例化 quad 渲染和片元模式；
- glyph atlas 的 slab / pin / LRU；
- 驱逐后锚点依然有效的虚拟布局；
- 文档编译器和 MetalRenderNode 节点词汇；
- Markdown 投影和编辑模型；
- diff 模型；
- 流式和打字机效果；
- 选择模型。

### 必须自己补上的能力

MetalTextKit 目前把下列能力全部交给 NSTextView，TekesUI 必须逐项自己实现：

- 编辑和输入法（NSTextInputClient 全量语义、组字、replacementRange）；
- 滚动（含惯性、橡皮筋、滚动条）；
- 选择粒度（字、词、行、段，双击三击拖选）；
- 复制（富文本、纯文本、跨节点）；
- 无障碍（VoiceOver 的文本导航、选区、朗读）；
- 拼写检查、查词、翻译、朗读等系统文本能力。

## 7. 背景记录：为什么仍然押注单 surface

2026-09，Tekes 的 transcript 转向 NSTextView 的 Native 路线，原因是选择、复制和无障碍：这些能力自绘实现成本高，NSTextView 白送。

TekesUI 仍然押注单 surface（整个内容区一个绘制面，自己管布局与输入），理由有两条：

1. 消除 CoreText 布局与 TextKit 输入层各算各的所导致的 CJK caret 漂移：两套排版引擎对同一段文字的度量不可能永远一致。
2. 避免 AppKit 视图树的开销：拖动分隔条时 CPU 约 55%。

坦白这个押注的代价：上面"必须自己补上的能力"一节，就是当初转向 Native 的理由。TekesUI 必须逐项补齐，才算这个押注成立；补不齐的部分，不能靠降低对比标准来过关。

## 8. 平台边界

| 放在引擎（跨平台） | 放在平台层 |
|---|---|
| 文档模型、布局、编辑状态机、undo | 窗口、事件泵、菜单、剪贴板 |
| display list、glyph atlas、渲染 | 输入法协议实现（NSTextInputClient / TSF）并转成规整事件 |
| 无障碍树的构造（AccessKit） | 无障碍树向系统的挂接 |
| 滚动的物理参数与手势识别后的行为 | 惯性滚动本身（mac 用系统 momentum phase） |
| 文本 trait 的定义与使用方 | 文本 trait 的各平台实现（CoreText / DirectWrite / Parley，在 tekes-ui-text 内按平台条件编译） |

- mac 优先：objc2 + CoreText + wgpu（Metal 后端）。
- Windows 在后：Win32 + TSF + DirectWrite，见 roadmap F7。
- Linux 再后：Parley 等；输入法（IBus/Fcitx/Wayland text-input）届时单独调研。
