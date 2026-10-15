# TekesUI DesignSystem v1 视觉基线

## 权威与范围

2026-10-15，用户在重新预览首次运行、文字上下自动滚动的原版 Gallery 后确认：“就是这个，以这个设计固定design system，不要完全改掉了”。因此 `a8ab9303ab2f528913f83cfc2c8da2a0eab8b8f3` 的原版视觉与本 crate token 值是当前权威基线。此前私有 artifact 优先、等待未知 Design 导出自动替换的约定不适用于本次冻结；artifact 或其他设计材料将来只能作为补充提案，不能自行推翻用户已批准的设计。

“DesignSystem v1”是设计基线标识，不是仓库 SemVer、发布 tag 或已发布版本。本轮冻结可见设计与现有契约，不宣称图标、无障碍、所有组件或完整产品设计系统已完成。

## 固定的视觉契约

- 以原有 Primitives、Token Swatches、静态文字三页为目标，不套用后续 10 页草稿的统一强制模板，不整体改版。
- 内容固定在左上角 1200×800 pt 校准框；更大窗口露出背景，更小窗口裁切。目录列 220 pt，页面内容原点 `(244, 62)`，保留原版页面标题、目录行、选中/hover、色块与文字对照窗格的风格。
- 颜色以 `design/tokens.json` 定义并生成的 27 个角色、四主题（light、dark、light-ic、dark-ic）解析结果为准，不手抄另一份 RGB 作为真源。
- 字体走系统 UI 字体与现有 CoreText 路径。六个文字角色及各自字号/行高/字重以 `design/tokens.json` 生成的文字 API 为准；静态文字校准夹具保留其独立的 14 pt 字体。不能为“统一”而改变既有视觉层级。
- 间距保留 `0/2/4/6/8/12/16/24 pt` 八档；圆角、描边、透明度、控件尺寸和动效以 `design/tokens.json` 及生成后的既有解析 API 为准。不同语义名称即便同值也保留。
- Gallery 外壳、菜单和页面一起遵守该契约；不能只令组件样本使用 token，同时把外壳改成另一种视觉语言。
- 24 张公开校准图位于 `design/baseline/screenshots/`，覆盖三页 × 四主题 × 两倍率。它们没有私有参照图；新 10 页的 80 张图不属于此基线。

原版源码中的“临时占位”标记是历史状态；JSON source 元信息保留此来源背景。用户现已批准这些现有值作为当前视觉契约；未来完善真值、动态系统色或增强对比度仍需逐项提案与确认，不能直接批量覆盖。

## 实现边界

实现保持纯 Rust，自研组件层不采用 GPUI/其他现成 Rust UI 框架。允许 wgpu、objc2 等 GPU/系统绑定基础依赖；调用 AppKit、CoreText 等系统框架不等于从零重写系统。现有 TekesMetalTextKit / TekesClientKit 源码只读参考，不复制 Swift 实现或加入 Swift 构建/运行垫片；compare 仅用于验证，不成为 Gallery 的运行依赖。旧文档若仍预设 Swift shim 或其他未来路线，以此用户最新边界为准。

## 增量与变更门禁

可以小步新增组件、搜索与 registry，但先保留现有页序、slug、布局和样式。新功能应融入既有视觉，不把导航扩展作为整体改版的理由。

任何影响色彩、字体、几何或整体画面的变更，必须：

1. 说明具体问题、最小改动与受影响的主题/倍率/页面。
2. 使用同一环境与输入提供旧/新并排截图，保留原始差异和未通过项。
3. 取得用户对可见变化的明确确认后，才按确认范围更新基线和来源清单。

禁止自动重生 golden 来绕过差异，禁止将 checker 通过等同于用户批准。不得移动旧基线来掩盖原图；原始来源与历史记录应可追溯。外部 artifact 缺失不阻塞保留当前设计，也不授权猜测或引入未知值。

## 验收边界

用户已亲验旧版滚动预览；24 图提供固定校准证据。历史文字对比/性能证据分别保留，旧性能门禁失败仍为失败，新版本性能、物理无掉帧、60 秒 idle、IME/VoiceOver 等缺口不能由视觉冻结替代。见 [冻结交接](docs/handoffs/design-baseline.md)。

## JSON 真源与机器入口

DesignSystem 不止 token 表。`design/system.json` 是机器入口，关联 token 数值层、布局/组件规范、交互/主题语义、可访问性缺口和原版基线证据。固定画布、列宽和对照夹具属于校准 specs，不新增成全局 token。

`design/tokens.json` 是生产 token 唯一数值来源。正常 Cargo build 由 token crate 的 `build.rs` 调用独立纯 Rust 编译器，将 JSON 生成到 `OUT_DIR/tokens.rs`；源码 `include!` 消费它，不保留旧数值 Rust 回退。JSON 变化通过 `rerun-if-changed` 触发重生成；缺失或无效输入使构建失败。手写 Rust 仅保留主题结构、颜色运算、文字样式结构与 concentric 规则，枚举及数值 getter 从 JSON 生成。

项目 schema 借鉴 DTCG `$type`/`$value`，保留逻辑 pt，不宣称完整 DTCG 兼容，也不暗中转换 CSS px/rem。颜色派生/引用保留原语义，解析后的 508 个 f32 通道须与批准 a8 原 getter 逐位一致。

`design/source-migration.json` 独立记录本次获授权的纯来源迁移，绑定原 manifest、JSON、生成器和适配源码；原 manifest 与 24 golden 不改。迁移不会授权视觉改变，不能自动更新收据绕过数值或画面差异。
