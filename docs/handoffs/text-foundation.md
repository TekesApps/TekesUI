# ScrollArea / Text / TextEditor 与统一 Gallery

本轮冻结新增功能，将主任务的滚动和文字、动画任务的 Motion、图标任务的 Icons 汇入同一个 Gallery。实现为 Rust + WGSL，系统窗口、输入服务、CoreText 与 Metal 通过 Rust 绑定访问；没有 Swift、GPUI 或第三方现成 UI 组件框架。JSON 仍是 Design System 唯一真源，原三页校准内容通过逐像素回归。

## 交付

- `design/gallery/navigation.json` 定义 section/items；16 页各登记一次，菜单与前九个快捷键跟随可见导航顺序。短窗口的侧栏和各页面均可滚动。
- `tekes-ui-controls` 提供 ScrollArea 与 UTF-16 TextInput 状态模型。系统 momentum 原样驱动惯性；边界回弹、方向锁定、两轴滚动、滚动条拖动、内容 reveal、Reduce Motion 均走共同实现。手势开始后固定 owner，跨侧栏、内外文本区和惯性阶段不串区；反向拖动先收回弹性拉伸。
- 渲染器支持嵌套 GPU scissor；裁剪不改变圆角形状与文字排版。文本只提交可见行及一行预取。
- Text 是只读选择/复制示例；TextEditor 包含单行和多行。字素移动/删除、中文预编辑、撤销重做、剪贴板、选择拖动及双/三击由 Rust 实现。CoreText 只提供字形排版/栅格；没有嵌入 NSTextView 代替文字组件。
- 文字、选区、光标和命中共用排版器的字素位置。单行光标/选区与输入框垂直居中；多行首行上对齐，显式换行选区延伸到行尾，软换行保留实际边界。编辑时内外滚动共同显露光标；闪烁不会把手动滚动拉回。失活保留逻辑焦点和选区，恢复后继续输入。
- Motion 使用 JSON 的十个预设及纯 Rust 动画内核；参数/目标中断、系统与手动 Reduce Motion 合成、离页与焦点恢复均已接入统一 Host。空闲不持续开显示链接。
- Icons 保留 10 个基础图标及本机 133 个只读旧图标，搜索共用 TextEditor。私有 SVG 未复制进 Git；没有来源目录时不能声称可复现 143 项。详情见 [图标交付](icons-gallery.md) 和 [动画交付](motion-preview.md)。

## 验证

完整工作区测试 347 项通过，0 失败；另有 1 项默认忽略的本机完整图标测试，由图标任务显式运行通过。严格 Clippy、格式、Design System 合并/生成新鲜度、纯 Rust 边界及其 8 项自测均通过。

生成 16 页 × 4 主题 × 1x/2x 共 128 张确定性快照；原三页校准内容的已提交像素回归通过。另生成 32 张选区/光标排版图，目检浅/深色单行、中文多行选区及单行光标；原生截图检查图标顶部/末端、缩放后位置、搜索、焦点恢复、Motion 及窄窗口裁剪。完整 PNG 和日志位于本机验收目录，未把私有图标截图加入公开源码。

原生文字脚本覆盖 ASCII 键盘解释、合成中文预编辑与提交、组合 emoji/附加符号删除、撤销、私有测试剪贴板复制/剪切/粘贴、多行替换、文首/文尾滚动。整合脚本覆盖名称搜索、长页 Home/End、缩放、最小化/恢复后的输入焦点、Motion 离页返回和 Reduce Motion；终段静止窗口没有额外重绘。

性能表与完整证据清单见 [verification.json](../../design/reviews/text-foundation/verification.json)。实际运行在内建 ProMotion 屏、2x、1440×827 pt，每次 12 秒测量，加预热与 5 秒空闲确认。采用 120 Hz / 8.333… ms 刷新间隔，CPU 工作/GPU p99 门槛均为 8.33 ms。

**边界：** 所有记录保留 GPU 最大值，包括超出 8.33 ms 的孤立峰值；p99 与显示回调间隔通过，不能解释为每帧呈现都达标。wgpu 当前没有暴露 drawable presentedTime，回调未丢刷新不能替代合成器的实际呈现验收。未进行物理触控板/真实中文候选窗口、多语言双向编辑完整验收、VoiceOver/完整无障碍、NSScrollView 同场 A/B；因此不声称已达到 NSScrollView 全功能与性能等价。逐字输入目前逐次撤销，尚无连续键入事务合并。40 个组件状态展示仍为固定状态插图；本轮没有补齐按钮/选择器生产交互。

## 重现

在活动 Gallery 工作树中运行：

```sh
cargo test --offline --locked --workspace
cargo clippy --offline --locked --workspace --all-targets -- -D warnings
cargo build --offline --locked --release -p tekes-ui-gallery
TEKESUI_GALLERY_PAGE=text-editing target/release/tekes-ui-gallery
TEKES_SELFTEST=foundation TEKES_SELFTEST_OUT=<已存在目录> target/release/tekes-ui-gallery
TEKES_PERF=scroll:12 TEKES_PERF_CONTENT=editor TEKES_PERF_OUT=<报告路径> target/release/tekes-ui-gallery
```

性能内容可替换为 `icons` / `motion`。测试和截图不能替代用户最终外观确认；未进行发布、推送或打标签。

## 最终二进制的三次测量

| 页面 | CPU 工作 p99 / ms | GPU p99 / ms | GPU 最大 / ms | 丢刷新数 | 空闲回调 |
|---|---:|---:|---:|---:|---:|
| editor | 0.902 | 2.602 | 2.635 | 1 | 0 |
| icons | 2.236 | 2.487 | 2.904 | 2 | 0 |
| motion | 1.168 | 2.498 | 2.573 | 0 | 0 |

最终三次均是 120 Hz、12 秒。Editor 和 Icons 分别记录 1、2 个孤立丢刷新，最长连续丢均为 1；Motion 为 0。p99 门槛通过，严格零掉帧未通过。此前的完整记录也保留：Icons / Motion GPU 最大值分别出现过 9.191 / 10.361 ms，不推断其原因，也不以复测覆盖它们。
