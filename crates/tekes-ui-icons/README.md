# TekesUI 默认图标

用户于 2026-10-16 批准以下 8 个优化稿作为默认使用资源：Settings、New Goal、Goal Completed、Memory、Think、Router、Provider、Appearance。

`assets/` 为正式 SVG 真源，随 crate 打包；`Icon::named("tekes-settings")` 或 `Icon::Settings.svg()` 取得默认新稿。名称不变，24 pt viewBox、1.5 pt 圆头描边、currentColor，由消费者负责着色和渲染。此 crate 不包含旧稿，且不依赖 Gallery 或私有仓库。

Gallery 用同一 API 取得这 8 项的默认稿；旧稿从本机旧图标目录只读加载，保留并排对比。其余 43 个新增/重绘提案仍待选择，不因这次批准变成默认图标。三个 Swift 应用的依赖更新和发布属于独立接入步骤，本次未修改。

## Animation annotations

`Icon::animation_layers()` exposes stable layer IDs, 24pt anchors, SVG element /
contour references and correspondence keys. Geometry remains in the approved SVG;
annotations do not duplicate or alter it. `animation_partner()` identifies the
Gallery transition target. NewGoal / GoalCompleted share a validated goal ring;
other pairs use ordinary replacement. The Gallery adapter parses these references
once and renders the original paths. Stroke layers support ordered arc-length
reveal; filled dots/regions fade as layers. See
[Symbol Effects](../../docs/design/symbol-effects.md) for contracts and verification.
