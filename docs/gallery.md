# Gallery

Gallery 是恒定的人工目测验收目标（章程第 2 节）：同一页、同一布局、每次长得一样。代码在 `crates/tekes-ui-gallery`。F1 起画面里是真文字（`tekes-ui-text`，系统界面字体，字号取自 `TextRole`：正文 13、列头与注释 11、节标题 11 粗；颜色取 `PrimaryText` / `SecondaryText` / `TertiaryText`）。文字放在固定大小的槽里，原点落在整数 pt 上，布局不随文字变化。

## 布局

内容永远排在 1200x800 pt 的固定框里，框锚在窗口左上角，**不随窗口缩放重排**：窗口更大就多露出背景，更小就裁掉右、下两侧。框的右、下边各有一条 `Separator` 线标出边界。

- 左边是目录列，宽 220 pt（180 的 `TruncationSlot` 加列内边距 8 和行内边距 12），每页一行：行高 `ControlSize::RowHeight`，圆角 `CornerRadius::Row`，选中 `RowSelection`，hover `ButtonHover`。
- 右边是页面区：左缘 244，顶上一个页面标题槽，内容区 (244, 62) 起，932 x 714。
- 几何常量和命中测试都在 `gallery/layout.rs`。

## 页面

| 页 | 内容 |
|---|---|
| Primitives | 直角与各档圆角（badge、row、panel、floating、input、column、control）；三档描边；三档 alpha 叠放；一行状态矩阵（类按钮样本）；并排参照槽 |
| Token Swatches | 27 个颜色角色色块（3 行 x 9 列，顺序固定为 `ColorRole::ALL`，色块下面是角色名，驼峰拆词、最多两行）；间距刻度（名字加值）；圆角一排；描边一排（名字加值） |
| 静态文字（⌘3） | 夹具选择器、宽度选择器、三个上下叠放的窗格（TekesUI 实时、参照图、差异热图），见下 |

类按钮样本：rest 透明，hover `ButtonHover`，pressed `HoverWash` 取 `Alpha::Soft`，disabled 整体 45% 不透明度，focus 是 1pt `BorderFocused` 轮廓，没有系统焦点环和阴影。样本自带一圈 hairline `Border`（Gallery 自己加的，否则 rest 与 disabled 看不见）。

## 静态文字页

夹具来自 `fixtures/text/`（编译时嵌入，文本原样含文件末尾的换行）：`sentences-a`（默认）、`mixed-cjk-latin`、`kinsoku`、`emoji`、`combining`、`rtl`、`long-paragraph`；宽度取自 `manifest.json`（默认 700，其余夹具另有 500、320；`sentences-a` 只有 700）。两排固定的小按钮选择，点一下即切换，只有状态变了才重绘；切夹具时若新夹具没有当前宽度，退回 700。字体是系统界面字体 14 pt，行间距 0，段落原点是窗格左上角（整数 pt）。

**布局取舍**：700 pt 的文字加三个窗格并排放不下（页面区宽 932 pt），所以这一页把参照槽改成**上下叠放**，每个窗格宽 = 所选宽度、高 170 pt（`ReferenceSlot::vertical`，边框画在窗格外 1 pt，窗格内部就是文字画布）。文字比窗格高时三个窗格都只显示放得下的前几行整行（场景里没有裁剪图元，所以按整行裁），标题行右侧写着“显示前 N 行 / 共 M 行（画布 宽x高 pt，倍率）”。差异数字是整张画布的结果，不受显示裁切影响。

**参照图**：环境变量 `TEKESUI_REFERENCE_DIR` 指向本地目录（私有伴随仓库的 `reference/`，只在运行时读取，不进本仓库）。目录里按 `<夹具>/<夹具>-w<宽度>@<倍率>x-<light|dark>.png` 找图，倍率是窗口 backing scale 四舍五入，深色与深色加高对比度取 dark，浅色与浅色加高对比度取 light；没有高对比度版本，所以高对比度主题下显示普通对比度的图并在标题行注明。图 1:1 画在设备像素上（图像矩形 = 像素尺寸 / 倍率）。可选的 `<目录>/appearances.json` 给出前景与背景色；没有时浅色 (0,0,0)/(255,255,255)，深色 (230,230,230)/(30,30,30)。实时窗格用同样的前景与背景色，三个窗格底色一致。

没设环境变量时，参照窗格写“未提供参照图（设置 TEKESUI_REFERENCE_DIR）”，差异窗格写“需要参照图才能比较”；图不存在、读不了、尺寸不符、离屏渲染失败时窗格里写明原因（只写文件名，不写目录）。

**差异**：参照图尺寸符合预期（宽 x 倍率，高 ceil(段落高度) x 倍率）时，把同一段落离屏渲染成同样大小的画布（`render_to_rgba8`，独立的 atlas），用 `compare::diff_rgba8` 比较，门槛取 `DiffOptions::for_scale`。窗格 3 是热图（不透明，没有差异的像素是深色底），下面一行是通过与否的标记和三个数字：超阈值比例、最大连通域、最大单像素差。结果按 `(夹具, 宽度, 倍率, 浅/深)` 缓存（最多 6 项），状态没变就不重算，也不重新登记图像；图像只登记显示区那几行。

## 文字引擎

`gallery/text.rs`：Gallery 在主线程拥有一份字体登记表、一个排版器、一个光栅化器和窗口用的 `GlyphAtlas`（`Host` 只通过 `Gallery::prepare` 和 `atlas_mut` 用它）。标签按（文字、`TextRole`、换行宽度）排一次版并缓存，之后每帧只 `push_paragraph`（atlas 命中）；段落与主题无关，颜色在编图元时才给，所以切主题和切页都不重新排版。每次重绘 `begin_frame` 一次并整个重编 scene，`AtlasGlyph` 只在编出它的那一帧有效，atlas 淘汰因此不需要额外处理。

## 菜单与快捷键

| 菜单 | 项 | 快捷键 |
|---|---|---|
| Pages | 每页一项，勾选当前页 | ⌘1、⌘2、⌘3 … |
| Appearance | Follow System / Light / Dark（单选） | ⌃⌘1 / ⌃⌘2 / ⌃⌘3 |
| Appearance | Increase Contrast: Follow System / On / Off（单选） | ⌃⌘4 / ⌃⌘5 / ⌃⌘6 |

快捷键选型：页面用 ⌘ 加序号；外观和对比度统一用 ⌃⌘ 加数字（外观 1 到 3，对比度 4 到 6，顺序同菜单）。

最初的 ⌥⌘S / ⌥⌘L / ⌥⌘D 里，⌥⌘D 是系统的“显示/隐藏 Dock”，到不了应用，所以整套改掉。检查方法：对照 macOS 系统快捷键默认值（Mission Control、Dock、Spotlight、截屏、辅助功能缩放/反转/对比度、锁屏 ⌃⌘Q、表情 ⌃⌘Space、查词 ⌃⌘D、Finder 搜索 ⌥⌘Space）和 AppKit 标准菜单（⌘H、⌥⌘H、⌘M、⌥⌘M、⌘W、⌥⌘W、⌘Q、⌃⌘F、⌘`、⌘,、⌘?）；读了本机 `defaults read com.apple.symbolichotkeys`（只有 ⌃Space、⌃⌥Space 输入法切换被显式启用，另有 Mission Control 的 ⌃←/→ 切空间，其余是系统默认值）、`NSUserKeyEquivalents`（无）和 `com.apple.universalaccess`（缩放、反转、对比度快捷键都未启用）。⌃⌘ 加数字 1 到 6 在这些清单里都没有占用。旧方案里只有 ⌥⌘D 确认冲突；⌥⌘L、⌥⌘S、⌥⌘C 不是系统全局键（在个别 App 里有含义，如 Safari、Finder 的 ⌥⌘L），⌃⌥⌘C 也未见占用；⌘1、⌘2 无冲突。

单元测试（`commands.rs`）断言菜单项快捷键互不相同，且不在一份已知占用组合的常量表里；这份表来自上面的系统默认值和 AppKit 标准菜单，不求完整。自测（`TEKES_SELFTEST=gallery`）合成本地 keyDown，经 `NSApp.mainMenu.performKeyEquivalent:` 触发 ⌃⌘1 到 6，标题和日志随主题变化；旧键 ⌥⌘D 不再被菜单接住。这条路径不经过系统全局热键，不能证明系统没有拦截，真实按键需要人工确认。

目录里点一行，或按 ↑ / ↓（到头不循环），也能切页。静态文字页的选择器只响应鼠标点击（没有快捷键，也没有 hover 效果）。窗口标题是 `TekesUI Gallery — <页面名> — <主题名>`。默认跟随系统外观和系统的 Increase Contrast。

重绘规则：只有可见状态真的变了（hover 行、选中页、生效主题）才请求重绘；鼠标在同一行内移动、重复选同一项都不重绘。

## 运行

```bash
cargo run -p tekes-ui-gallery            # 开窗口（窗口摆放按 WindowPlacement::Auto，见 platform-boundary.md）
TEKES_LOG_EVENTS=1 cargo run -p tekes-ui-gallery   # 同时把事件和重绘次数打到 stderr
```

自测：`TEKES_SELFTEST=gallery TEKES_SELFTEST_OUT=<目录>`（可同时设 `TEKESUI_REFERENCE_DIR` 看参照图和热图；末尾 t01 到 t07 是静态文字页：切页、深浅色、点选夹具与宽度）按时间表触发菜单命令、方向键、点击、鼠标移动和窗口外观切换，每步截图后正常退出（脚本见 `main.rs` 的 `selftest_gallery_script`）。动作由 `tekes_ui_platform_macos::selftest` 在主线程里调 AppKit 入口，不合成全局输入事件。

## 快照

```bash
cargo run -p tekes-ui-gallery -- --snapshot <目录>
```

不开窗口，用 `Renderer::new_headless` 把每页（含静态文字页，固定为 `sentences-a`、宽 700、不带参照目录，与环境变量无关）x 4 个主题（light、dark、light-ic、dark-ic）x 缩放 2 和 1 渲染成 PNG，文件名 `<页>-<主题>@<缩放>x.png`，如 `primitives-dark-ic@2x.png`、`static-text-light@1x.png`，尺寸是 1200x800 乘缩放。同样的输入逐字节得到同样的 PNG（测试里断言，没有 GPU 时跳过）。这是以后自动对比的共用夹具。

## 加一页

1. 在 `gallery/pages/` 新建 `xxx.rs`，实现 `pub fn build(ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene)`，只用 token、scene、`ctx.text`（文字引擎）和 `containers`，同一主题同一状态必须编出同一份图元。
2. 在 `pages/mod.rs` 里加 `pub mod xxx;`、`Page` 的一个变体，并补 `ALL`、`name`、`slug`、`build`。目录行、菜单项（⌘ 加序号）、快照文件名都由 `Page::ALL` 派生，不用另外改。
3. 组件页用 `StateMatrix`（rest / hover / pressed / disabled / focus 五列，样本由回调按 `ComponentState` 画）和 `ReferenceSlot`（TekesUI live | 参照图 | 差异热力图，可左右并排也可上下叠放；图像图元由页面按窗格矩形自己画，见静态文字页）。
4. 补测试：图元不越出内容区、同一页同一主题编两次相同。
