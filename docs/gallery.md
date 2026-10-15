# Gallery

> 2026-10-15 最新设计权威：用户提供的 v0.3.0-proposal1 有定义时优先，缺项保留现有补集。参见 [DESIGN.md](../DESIGN.md) 和完整导入索引；本文此前固定尺寸、旧值和原版优先约定仅在不冲突时适用。规范存在不等于原生实现已验收。


Gallery 是恒定的人工目测验收目标（章程第 2 节）：同一页、同一布局、每次长得一样。代码在 `crates/tekes-ui-gallery`。F1 起画面里是真文字（`tekes-ui-text`，系统界面字体，字号取自 `TextRole`：正文 13、列头与注释 11、节标题 11 粗；颜色取 `PrimaryText` / `SecondaryText` / `TertiaryText`）。文字放在固定大小的槽里，原点落在整数 pt 上，布局不随文字变化。

## 布局

当前内容排在 JSON 定义的 1440×960 pt 固定框里，锚在窗口左上角，不随窗口缩放重排。窗口更大时露出背景；原三页在较小窗口中裁切，新增八类组件页支持局部横向和纵向滚动。

- 左边是目录列，宽 288 pt，每页一行：行高 `ControlSize::RowHeight`，圆角 `CornerRadius::Row`，选中 `RowSelection`，hover `ButtonHover`。
- 右边是页面区：左缘 312，顶上一个页面标题槽；内容区由 JSON 的标题行高、Section 间距和捕获框派生。
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

**布局取舍**：700 pt 宽的三个文字窗格无法并排放进固定内容区，所以这一页把参照槽改成**上下叠放**，每个窗格宽 = 所选宽度、高 170 pt（`ReferenceSlot::vertical`，边框画在窗格外 1 pt，窗格内部就是文字画布）。文字比窗格高时三个窗格都只显示放得下的前几行整行（场景里没有裁剪图元，所以按整行裁），标题行右侧写着“显示前 N 行 / 共 M 行（画布 宽x高 pt，倍率）”。差异数字是整张画布的结果，不受显示裁切影响。

**参照图**：环境变量 `TEKESUI_REFERENCE_DIR` 指向本地目录（私有伴随仓库的 `reference/`，只在运行时读取，不进本仓库）。目录里按 `<夹具>/<夹具>-w<宽度>@<倍率>x-<light|dark>.png` 找图，倍率是窗口 backing scale 四舍五入，深色与深色加高对比度取 dark，浅色与浅色加高对比度取 light；没有高对比度版本，所以高对比度主题下显示普通对比度的图并在标题行注明。图 1:1 画在设备像素上（图像矩形 = 像素尺寸 / 倍率）。可选的 `<目录>/appearances.json` 给出前景与背景色；没有时浅色 (0,0,0)/(255,255,255)，深色 (230,230,230)/(30,30,30)。实时窗格用同样的前景与背景色，三个窗格底色一致。

没设环境变量时，参照窗格写“未提供参照图（设置 TEKESUI_REFERENCE_DIR）”，差异窗格写“需要参照图才能比较”；图不存在、读不了、尺寸不符、离屏渲染失败时窗格里写明原因（只写文件名，不写目录）。

**差异**：参照图尺寸符合预期（宽 x 倍率，高 ceil(段落高度) x 倍率）时，把同一段落离屏渲染成同样大小的画布（`render_to_rgba8`，独立的 atlas），用 `compare::diff_rgba8` 比较，门槛取 `DiffOptions::for_scale`。窗格 3 是热图（不透明，没有差异的像素是深色底），下面一行是通过与否的标记和三个数字：超阈值比例、最大连通域、最大单像素差。结果按 `(夹具, 宽度, 倍率, 浅/深)` 缓存（最多 6 项），状态没变就不重算，也不重新登记图像；图像只登记显示区那几行。

## 文字引擎

`gallery/text.rs`：Gallery 在主线程拥有一份字体登记表、一个排版器、一个光栅化器和窗口用的 `GlyphAtlas`（`Host` 只通过 `Gallery::prepare` 和 `atlas_mut` 用它）。标签按（文字、`TextRole`、换行宽度）排一次版并缓存，之后每帧只 `push_paragraph`（atlas 命中）；段落与主题无关，颜色在编图元时才给，所以切主题和切页都不重新排版。每次重绘 `begin_frame` 一次并整个重编 scene，`AtlasGlyph` 只在编出它的那一帧有效，atlas 淘汰因此不需要额外处理。

## 菜单与快捷键

| 菜单 | 项 | 快捷键 |
|---|---|---|
| Pages | 11 页各一项，勾选当前页 | 前 9 页为 ⌘1 至 ⌘9；其余用目录、菜单或上下键 |
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

不开窗口，用 `Renderer::new_headless` 把每页（含静态文字页，固定为 `sentences-a`、宽 700、不带参照目录，与环境变量无关）x 4 个主题（light、dark、light-ic、dark-ic）x 缩放 2 和 1 渲染成 PNG，文件名 `<页>-<主题>@<缩放>x.png`，如 `primitives-dark-ic@2x.png`、`static-text-light@1x.png`，尺寸是 1440×960 乘缩放，共 16×4×2＝128 张。同样的输入逐字节得到同样的 PNG（测试里断言，没有 GPU 时跳过）。这是以后自动对比的共用夹具。

## 加一页

1. 在 `gallery/pages/` 新建 `xxx.rs`，实现 `pub fn build(ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene)`，只用 token、scene、`ctx.text`（文字引擎）和 `containers`，同一主题同一状态必须编出同一份图元。
2. 在 `pages/mod.rs` 里加 `pub mod xxx;`、`Page` 的一个变体，并补 `ALL`、`name`、`slug`、`build`。快照文件名由 `Page::ALL` 派生；在 `design/gallery/navigation.json` 的 section/items 中登记页面，每页恰好一次。菜单和 ⌘1…9 跟随该导航顺序。
3. 组件页用 `StateMatrix`（rest / hover / pressed / disabled / focus 五列，样本由回调按 `ComponentState` 画）和 `ReferenceSlot`（TekesUI live | 参照图 | 差异热力图，可左右并排也可上下叠放；图像图元由页面按窗格矩形自己画，见静态文字页）。
4. 补测试：图元不越出内容区、同一页同一主题编两次相同。

## 组件状态展示

原三页之后新增八类组件状态页，覆盖提案40个ID。状态选择与控件自身交互的边界、短窗滚动及验收见 [component-gallery.md](handoffs/component-gallery.md)。

## Icons 页面

左侧目录与 Pages 菜单中的 Icons 展示 Gallery 当前组件共用的 10 种基础图标，以及本机旧 Tekes 图标集，每项预览 12 / 16 / 24 pt。两组按名称一起搜索，不区分大小写，执行包含匹配（等价于 `%query%`，无需输入百分号；百分号和下划线作为普通字符）。空查询显示全部，无匹配时显示空状态。支持选择、复制粘贴、撤销和 IME；切页保留查询并释放输入焦点。列表按筛选结果扩展滚动范围，屏外图标不绘制，SVG 预览按主题与屏幕倍率缓存。

完整图标集通过 `TEKESUI_ICON_DIR` 指定，只读支持 `*.imageset/<name>.svg` 或平铺 `*.svg`。未设置时查找同级 Tekes 仓库或 `~/dev/Tekes/Tekes/Assets.xcassets/TekesIcons`。只加载 imageset 中的单个矢量母版，避免将 2x/3x 重复列成图标。按既有清单排除 5 个已作废原稿及其他带 C2PA 标记的文件，解析失败的数量显示在页内。当前本地旧集为 133 项，加基础 10 项共 143 项；来源缺失时明确显示未加载，不将缺失伪装成完整集。

旧图标资产不复制或嵌入公开源码，不改来源目录；此处为本机只读预览，不表示完成 MIT 导入或原创性核对。SVG 由纯 Rust `resvg` 光栅化，无字体加载和外部图像解析。固定快照仍采用不依赖本机资产的 10 项基础夹具。


## 基础组件与统一整合

导航采用 `design/gallery/navigation.json` 的 section/items 两层结构。基础组件包含 ScrollArea、Text、TextEditor 和 Icons；Motion 单独归入交互与动效。页面与侧栏均使用同一 Rust ScrollArea 状态模型，滚动由系统手势和 momentum 驱动，渲染器嵌套 scissor 裁剪，不挪动或重建被裁剪的圆角形状。

Text 支持只读选择与复制；TextEditor 展示单行与多行输入，复用 UTF-16 选区、字素移动/删除、预编辑、撤销重做和剪贴板。文字、选区、光标与命中共用 CoreText 的排版位置。单行垂直居中，多行按首行排版；内外层滚动共同保证编辑光标可见。窗口失活保留逻辑焦点与选区，停止绘制光标，恢复后继续输入。

`TEKESUI_GALLERY_PAGE=text-editing` 可直接打开 TextEditor，`motion-foundation` 打开 Motion。`TEKES_SELFTEST=text` 验证原生文字桥，`TEKES_SELFTEST=foundation` 验证整合后的图标搜索、窗口缩放、焦点恢复、Motion 离页恢复和减运动。需提供 `TEKES_SELFTEST_OUT` 目录；自测剪贴板使用独立命名空间，不覆盖用户剪贴板。

`TEKES_PERF=scroll:12 TEKES_PERF_CONTENT=editor|icons|motion` 分别测三类实际页面。逐项的通过、峰值和未测边界见 [基础组件整合验收](handoffs/text-foundation.md)。40 项组件状态展示仍是固定状态插图，不等于全部生产组件已实现。
