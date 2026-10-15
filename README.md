# TekesUI

纯 Rust 的 UI foundation 与组件库，mac 优先，之后扩展到 Windows 和 Linux。

它是与现有 Swift 栈（TekesMetalTextKit，以及 Tekes 客户端的 UI 层）平行的长期项目：慢慢做、做扎实，产出就是组件，不绑定任何具体产品。最终目标是 Tekes 系列 app 全部用 Rust 跑，Swift 只保留一层极薄的 Apple 能力垫片。

## 当前状态

F0（平台层、窗口、wgpu、Gallery 骨架）已实现，待人工检查项确认，见 [docs/handoffs/F0.md](docs/handoffs/F0.md)。token 是临时占位值。

运行 Gallery：

```bash
cargo run -p tekes-ui-gallery --release
```

## 验收方式

- 每个组件都有一个恒定的人工目测验收目标：Gallery（类似 Storybook 的 Rust 应用），固定夹具、固定布局、完整状态矩阵，有参照图就并排展示。
- 与现有 Swift 实现的效果对比有统一的方法（像素差容差、排版数值、性能、输入法与 VoiceOver 验收），见对比方法文档。
- 不设日期，按通过标准推进。

## 文档索引

- [docs/charter.md](docs/charter.md)：定位、原则、crate 分层、平台边界
- [docs/roadmap.md](docs/roadmap.md)：F0 到 F7 各阶段的范围、对照对象与通过标准
- [docs/comparison-method.md](docs/comparison-method.md)：与参照实现对比的方法与容差
- [docs/known-differences.md](docs/known-differences.md)：与参照实现的已知差异
- [docs/dependencies.md](docs/dependencies.md)：外部依赖版本与许可证复核
- [docs/platform-boundary.md](docs/platform-boundary.md)：引擎与 mac 平台层的接口、帧节奏、窗口摆放
- [docs/gallery.md](docs/gallery.md)：Gallery 的布局、页面、菜单与快照
- [docs/handoffs/](docs/handoffs/)：各阶段交接与测量记录
- [docs/design/icons-and-tokens-brief.md](docs/design/icons-and-tokens-brief.md)：图标与设计 token 的调研任务书

## 许可

MIT，见 [LICENSE](LICENSE)。不依赖也不复制任何 GPL 代码。
