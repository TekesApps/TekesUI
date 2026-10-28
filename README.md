# TekesUI

纯 Rust 的 UI foundation 与组件库，mac 优先，之后扩展到 Windows 和 Linux。

它是独立于现有 Swift UI 栈的自研组件层。当前仓库实现保持纯 Rust，不采用 GPUI 或其他现成 Rust UI 框架；wgpu 是 GPU 基础依赖，objc2 是系统绑定，窗口/菜单调用 AppKit、字体调用 CoreText。既有 Kit 源码只读参考，私有 compare 仓库只提供对比证据，不作为构建或运行依赖；不引入 Swift 垫片。

## 当前状态

已接入用户提供的 [v0.3.0-proposal1 完整设计包](design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/README.md)。包有的定义优先，包没有的保留现有补集；原始 ZIP、代码、文档、研究和报告全部保存。提案标识不等于仓库发布版本。

[design/system.json](design/system.json) 是机器入口，[DESIGN.md](DESIGN.md) 说明权威与边界。基础 token、显式消费者品牌配置与统一 policy 生成四主题，再生成 Rust 兼容 API；原生 Gallery 仍为纯 Rust。Gallery 已改为 Overview、Foundations、Components、Patterns、Examples 和 Quality lab 的原生工作台；组件注册、搜索、Story 路由与成熟度集中在 `design/gallery/workbench.json`。关键控件使用真实 Rust 状态模型，Settings 和组合场景可操作；缺失组件保留明确的 planned 坑位。历史状态插图只作为 Quality lab 的比较夹具，不计为真实组件实现。详见 [Gallery 使用与扩展约定](docs/gallery.md)。

公共组件接口：[可编辑数值控件](docs/components/number-field.md)、[Ticking Number](docs/components/ticking-number.md)。

使用、品牌定制与检查命令见 [design/README.md](design/README.md)。原版 24 张 golden 与历史性能结果保留，新提案画面另行审查，不覆盖历史证据。

查询当前公开 API、Gallery route、适用状态与证据边界：

```bash
python3 scripts/component-context.py --component tui.button
```

省略 `--component` 输出完整当前清单。此入口只读，清单或源码过期时拒绝输出，不回退到历史提案的 planned 状态；历史工具仍保留在 `integration.json` 的 `tools.historical_context`。详见[当前 context 入口](docs/design/current-context-20261019.md)。

运行当前 Gallery：

```bash
cargo run --locked -p tekes-ui-gallery
```

## 可独立消费的本地核心阶段（macOS）

已有公共能力可以从完整公开 commit [`52ed62b1468a332b208cf61f4da06dc45ac72764`](https://github.com/TekesApps/TekesUI/tree/52ed62b1468a332b208cf61f4da06dc45ac72764) 消费，对应 annotated `checkpoint-20261029-2`。这是阶段检查点，不是正式 Release，也不表示整个 Gallery 或全部原生验收完成。独立 checkout 保留 `crates/` 与 `examples/` 的相对位置；下面的消费者各有自己的 Cargo workspace 和锁文件，使用库的公共 API，不依赖 Gallery 类型或外部开发目录。注册表依赖仍按各自锁文件解析。

| 已交付公共能力 | 现有独立消费者与已验行为 | 具体边界 |
| --- | --- | --- |
| `AppWindow`、`SettingsWindow`、Settings schema/draft/file provider、Row/List 与设置项输入 | [AppHost 消费者](examples/app-host-demo/README.md)：原 Main 打开 Settings，Exclusive/Independent 两种模式的显示、编辑自动保存、Undo、关闭及窗口/焦点恢复；最终独立 checkout 和同源 CI 已通过，#70 已关闭。 | 原生证据固定在 #70 的 `cd27d0b`／最终 `45f1cf1`、PR139／checkpoint-20261027-2；当前 pin 包含这些提交，不能将旧运行改签成新运行。物理 IME/VoiceOver 不在此处宣称通过。 |
| `ContentHost`、`PanelContent`、`StackContent`、Button、`TextFieldContent`、`ScrollContent` | [Panel 消费者](examples/panel-content-consumer/README.md)：原树真实 CoreText 编辑几何、动作与 mount、marked/selection、阅读锚点及嵌套 Panel/Split；原 CPU3、Metal450组合与18完整 coverage oracle 各保持原收据身份。 | 450组合不等于450份完整像素 oracle。后续本地原生批次已完成实际 wheel／曲线点击／UTF-16／80行 reveal／Undo／Tab／close；精确源码、程序及远端交付另见 #82。实体中文候选交互和 VoiceOver 未验，#82 保持开放。 |
| 普通本地列表、文本输入及组合、Theme/Palette/Surface 语义消费 | [公共组合消费者](examples/composition-consumer/README.md)和上述已有设置/Panel路径；源码、锁、公共接口与本票适用行为证据沿用各原 Issue。 | 不因此宣布所有组合、物理输入、性能或平台均已验；具体未验项继续留原票。全52根实用 Theme 接入已由 #113/#114/#121–#125 完成，不重复开发，也不替代实际产品验收。 |

当前独立原生窗口、CoreText 和 Metal 验收适用于 **macOS**；Windows/Linux 保持后续平台目标。普通本地组件的消费不等待 #81 B2 跨进程曲线链、#127 完整 Toolbar 有限支持或完整 Gallery。已有 AppHost 多进程样例使用其已交付的普通链路，不冒充新的 B2 能力。

复用现有入口；本仓运行继续使用协调者的共享锁、SSD 输出和源码身份规则，不另建 target。Settings 原生验收使用隔离 HOME，避免把测试写入个人配置。

```sh
cargo run --manifest-path examples/app-host-demo/Cargo.toml --locked --offline -- --verify
cargo test --manifest-path examples/panel-content-consumer/Cargo.toml --test panel_content --locked --offline
cargo test --manifest-path examples/composition-consumer/Cargo.toml --test scenarios --locked --offline
```

这些命令分别运行已有入口，不代表本段文档更新时重新执行，也不包含原生窗口或真实 IME。窗口使用与各层证据见各消费者 README；上述公开 pin 是本地核心阶段起点，不包含后续 #82 原生入口；其新增交付身份由原 Issue 维护，不把旧 pin 改签为新运行。

## 验收方式

- 每个组件都有一个恒定的人工目测验收目标：Gallery（类似 Storybook 的 Rust 应用），固定夹具、固定布局、完整状态矩阵，有参照图就并排展示。
- 与现有 Swift 实现的效果对比有统一的方法（像素差容差、排版数值、性能、输入法与 VoiceOver 验收），见对比方法文档。
- 不设日期，按通过标准推进。

## 文档索引

- [DESIGN.md](DESIGN.md)：用户批准的原版视觉契约及变更边界
- [design/README.md](design/README.md)：24 张基线、来源与更新规则

- [docs/charter.md](docs/charter.md)：定位、原则、crate 分层、平台边界
- [docs/platform-boundary.md](docs/platform-boundary.md)：本地 main 明确代码基线的实际职责/依赖、目标差距、待确认架构决定及后续 Issue 队列；F0 平台记录单列为历史
- [docs/roadmap.md](docs/roadmap.md)：F0 到 F7 各阶段的范围、对照对象与通过标准
- [docs/comparison-method.md](docs/comparison-method.md)：与参照实现对比的方法与容差
- [docs/known-differences.md](docs/known-differences.md)：与参照实现的已知差异
- [docs/dependencies.md](docs/dependencies.md)：外部依赖版本与许可证复核
- [docs/gallery.md](docs/gallery.md)：Gallery 的布局、页面、菜单与快照
- [docs/handoffs/](docs/handoffs/)：各阶段交接与测量记录
- [docs/design/icons-and-tokens-brief.md](docs/design/icons-and-tokens-brief.md)：图标与设计 token 的调研任务书
- [design/README.md](design/README.md)：设计产出的保存约定与导入流转

## 许可

MIT，见 [LICENSE](LICENSE)。不依赖也不复制任何 GPL 代码。
