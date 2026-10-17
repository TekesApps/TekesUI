# TekesUI

纯 Rust 的 UI foundation 与组件库，mac 优先，之后扩展到 Windows 和 Linux。

它是独立于现有 Swift UI 栈的自研组件层。当前仓库实现保持纯 Rust，不采用 GPUI 或其他现成 Rust UI 框架；wgpu 是 GPU 基础依赖，objc2 是系统绑定，窗口/菜单调用 AppKit、字体调用 CoreText。既有 Kit 源码只读参考，私有 compare 仓库只提供对比证据，不作为构建或运行依赖；不引入 Swift 垫片。

## 当前状态

已接入用户提供的 [v0.3.0-proposal1 完整设计包](design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/README.md)。包有的定义优先，包没有的保留现有补集；原始 ZIP、代码、文档、研究和报告全部保存。提案标识不等于仓库发布版本。

[design/system.json](design/system.json) 是机器入口，[DESIGN.md](DESIGN.md) 说明权威与边界。基础 token、显式消费者品牌配置与统一 policy 生成四主题，再生成 Rust 兼容 API；原生 Gallery 仍为纯 Rust。Gallery 已改为 Overview、Foundations、Components、Patterns、Examples 和 Quality lab 的原生工作台；组件注册、搜索、Story 路由与成熟度集中在 `design/gallery/workbench.json`。关键控件使用真实 Rust 状态模型，Settings 和组合场景可操作；缺失组件保留明确的 planned 坑位。历史状态插图只作为 Quality lab 的比较夹具，不计为真实组件实现。详见 [Gallery 使用与扩展约定](docs/gallery.md)。

使用、品牌定制与检查命令见 [design/README.md](design/README.md)。原版 24 张 golden 与历史性能结果保留，新提案画面另行审查，不覆盖历史证据。

运行当前 Gallery：

```bash
cargo run --locked -p tekes-ui-gallery
```

## 验收方式

- 每个组件都有一个恒定的人工目测验收目标：Gallery（类似 Storybook 的 Rust 应用），固定夹具、固定布局、完整状态矩阵，有参照图就并排展示。
- 与现有 Swift 实现的效果对比有统一的方法（像素差容差、排版数值、性能、输入法与 VoiceOver 验收），见对比方法文档。
- 不设日期，按通过标准推进。

## 文档索引

- [DESIGN.md](DESIGN.md)：用户批准的原版视觉契约及变更边界
- [design/README.md](design/README.md)：24 张基线、来源与更新规则

- [docs/charter.md](docs/charter.md)：定位、原则、crate 分层、平台边界
- [docs/roadmap.md](docs/roadmap.md)：F0 到 F7 各阶段的范围、对照对象与通过标准
- [docs/comparison-method.md](docs/comparison-method.md)：与参照实现对比的方法与容差
- [docs/known-differences.md](docs/known-differences.md)：与参照实现的已知差异
- [docs/dependencies.md](docs/dependencies.md)：外部依赖版本与许可证复核
- [docs/platform-boundary.md](docs/platform-boundary.md)：引擎与 mac 平台层的接口、帧节奏、窗口摆放
- [docs/gallery.md](docs/gallery.md)：Gallery 的布局、页面、菜单与快照
- [docs/handoffs/](docs/handoffs/)：各阶段交接与测量记录
- [docs/design/icons-and-tokens-brief.md](docs/design/icons-and-tokens-brief.md)：图标与设计 token 的调研任务书
- [design/README.md](design/README.md)：设计产出的保存约定与导入流转

## 许可

MIT，见 [LICENSE](LICENSE)。不依赖也不复制任何 GPL 代码。
