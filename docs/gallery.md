# Gallery

Gallery 是恒定的人工目测验收目标（章程第 2 节）：同一页、同一布局、每次长得一样。代码在 `crates/tekes-ui-gallery`。F0 还没有文字渲染（F1），所以画面里没有文字：页面名在窗口标题和菜单里，画面里凡是以后要放文字的地方都留了固定大小的槽，现在画一条占位条。

## 布局

内容永远排在 1200x800 pt 的固定框里，框锚在窗口左上角，**不随窗口缩放重排**：窗口更大就多露出背景，更小就裁掉右、下两侧。框的右、下边各有一条 `Separator` 线标出边界。

- 左边是目录列，宽 220 pt（180 的 `TruncationSlot` 加列内边距 8 和行内边距 12），每页一行：行高 `ControlSize::RowHeight`，圆角 `CornerRadius::Row`，选中 `RowSelection`，hover `ButtonHover`。
- 右边是页面区：左缘 244，顶上一个页面标题槽，内容区 (244, 62) 起，932 x 714。
- 几何常量和命中测试都在 `gallery/layout.rs`。

## 页面

| 页 | 内容 |
|---|---|
| Primitives | 直角与各档圆角（badge、row、panel、floating、input、column、control）；三档描边；三档 alpha 叠放；一行状态矩阵（类按钮样本）；并排参照槽 |
| Token Swatches | 27 个颜色角色色块（3 行 x 9 列，顺序固定为 `ColorRole::ALL`，没有文字所以数格子对照）；间距刻度；圆角一排；描边一排 |

类按钮样本：rest 透明，hover `ButtonHover`，pressed `HoverWash` 取 `Alpha::Soft`，disabled 整体 45% 不透明度，focus 是 1pt `BorderFocused` 轮廓，没有系统焦点环和阴影。样本自带一圈 hairline `Border`（Gallery 自己加的，否则 rest 与 disabled 看不见）。

## 菜单与快捷键

| 菜单 | 项 | 快捷键 |
|---|---|---|
| Pages | 每页一项，勾选当前页 | ⌘1、⌘2 … |
| Appearance | Follow System / Light / Dark（单选） | ⌥⌘S / ⌥⌘L / ⌥⌘D |
| Appearance | Increase Contrast: Follow System / On / Off（单选） | ⌃⌥⌘C / ⌥⌘C / ⇧⌥⌘C |

目录里点一行，或按 ↑ / ↓（到头不循环），也能切页。窗口标题是 `TekesUI Gallery — <页面名> — <主题名>`。默认跟随系统外观和系统的 Increase Contrast。

重绘规则：只有可见状态真的变了（hover 行、选中页、生效主题）才请求重绘；鼠标在同一行内移动、重复选同一项都不重绘。

## 运行

```bash
cargo run -p tekes-ui-gallery            # 开窗口（窗口摆放按 WindowPlacement::Auto，见 platform-boundary.md）
TEKES_LOG_EVENTS=1 cargo run -p tekes-ui-gallery   # 同时把事件和重绘次数打到 stderr
```

自测：`TEKES_SELFTEST=gallery TEKES_SELFTEST_OUT=<目录>` 按时间表触发菜单命令、方向键、点击、鼠标移动和窗口外观切换，每步截图后正常退出（脚本见 `main.rs` 的 `selftest_gallery_script`）。动作由 `tekes_ui_platform_macos::selftest` 在主线程里调 AppKit 入口，不合成全局输入事件。

## 快照

```bash
cargo run -p tekes-ui-gallery -- --snapshot <目录>
```

不开窗口，用 `Renderer::new_headless` 把每页 x 4 个主题（light、dark、light-ic、dark-ic）x 缩放 2 和 1 渲染成 PNG，文件名 `<页>-<主题>@<缩放>x.png`，如 `primitives-dark-ic@2x.png`，尺寸是 1200x800 乘缩放。同样的输入逐字节得到同样的 PNG（测试里断言，没有 GPU 时跳过）。这是以后自动对比的共用夹具。

## 加一页

1. 在 `gallery/pages/` 新建 `xxx.rs`，实现 `pub fn build(theme: Theme, content: Rect, scene: &mut Scene)`，只用 token、scene 和 `containers`，同一主题必须编出同一份图元。
2. 在 `pages/mod.rs` 里加 `pub mod xxx;`、`Page` 的一个变体，并补 `ALL`、`name`、`slug`、`build`。目录行、菜单项（⌘ 加序号）、快照文件名都由 `Page::ALL` 派生，不用另外改。
3. 组件页用 `StateMatrix`（rest / hover / pressed / disabled / focus 五列，样本由回调按 `ComponentState` 画）和 `ReferenceSlot`（TekesUI live | 参照图 | 差异热力图，参照窗格现在是空占位，等图像图元）。
4. 补测试：图元不越出内容区、同一页同一主题编两次相同。
