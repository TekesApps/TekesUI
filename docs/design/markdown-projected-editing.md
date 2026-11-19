# F5 Markdown 投影编辑与 AppHost `others`（Markdown + Diff）契约设计（#132）

- **设计票**：[#132](https://github.com/TekesApps/TekesUI/issues/132)
- **父规划**：[#56](https://github.com/TekesApps/TekesUI/issues/56)
- **关联基线**：[#17](https://github.com/TekesApps/TekesUI/issues/17)、[#18](https://github.com/TekesApps/TekesUI/issues/18)、[#25](https://github.com/TekesApps/TekesUI/issues/25)、[#26](https://github.com/TekesApps/TekesUI/issues/26)、[#128](https://github.com/TekesApps/TekesUI/issues/128)
- **执行拆分**：[#362](https://github.com/TekesApps/TekesUI/issues/362)（M1 投影模型）、[#363](https://github.com/TekesApps/TekesUI/issues/363)（M2 编辑器与事务）、[#364](https://github.com/TekesApps/TekesUI/issues/364)（M3 Gallery 与 `AppHost::others`）

---

## 1. 复核现有能力、复用边界与非范围

### 1.1 明确复用的既有能力
1. **文本状态、IME 与原生桥接（`#17` / `#18` / `#26` / `#128`）**：
   - `TextFieldContent` + `TextSurface` + `TextInput` 已经是唯一拥有原生 `NSTextInputClient` 协议、UTF-16/字素光标安全（CJK / Emoji 代理对 / ZWJ）、`SetMarked` preedit 分节与 `replacement`、`TextEditCommand`、滚动条回弹以及 `ContentTextInput` / `ContentTextLayout` 导出的文本 owner。
   - `MarkdownEditorContent` 直接内部组合一个多行 `TextFieldContent`（与 `RichContent` 同一模式），**绝不另造第二套文字输入机、第二套 CoreText 命中测试引擎或修改 `tekes-ui-module-runtime` / `tekes-ui-app-protocol` 的 `wire::TextState`**。
2. **只读富文档与 Diff 视图（`#25`）**：
   - `RichDocument` 与 `RichContent` 保持**严格只读**，继续服务 Documents 面板、代码预览和 `RichDocument::from_diff(&DiffView)` 只读 Diff 检视；不把 `RichContent` 改写成混合可变编辑器，以免破坏只读阅读、锚点与不可变版本假设。
   - 代码块语法着词直接复用并抽取 `crates/tekes-ui-controls/src/composition/rich_content/syntax.rs` 中的 `parse_code_slice`（共享 `OnceLock<SyntaxSet>` 与 `c-code-keyword` / `c-code-string` / `c-code-number` / `c-code-comment` token）。
3. **AppHost 容器与 `others` 面板**：
   - `LocalContainer` 与 `ProcessContainer` 已通过 `PresentationOptions::others(...)` 支持在宿主窗口附栏同时挂载多个独立容器。AgentKit 可将 Markdown Live Preview 视图与只读 Diff 视图分别作为 `others` 容器按需挂载。

### 1.2 实际缺口
- `RichDocument` 为只读展示裁剪了换行与围栏空行，无法无损回写源码（例如无法区分空行、未闭合代码围栏、光标进入某块时展开标记）。
- 缺少 **Canonical Source ⇄ Live Preview Display** 的双向增量对账与活跃块展开模型、规范源码级 Undo/Redo 栈，以及输入法 preedit / 指针拖拽期间的**投影冻结**保障。

### 1.3 非范围（Non-Goals）
- 不将 `RichContent` 改造为可编辑控件。
- 不支持完整 CommonMark（GFM 表格、HTML 嵌入、脚注、嵌套引用、图片编辑）。
- 不提供富文本工具栏 / 斜杠菜单 / 浮动格式栏，也不修改 `DiffView` 支持内联代码编辑。
- 不在未分配专项窗口的情况下替代 VoiceOver 真机验收（#5）或系统拼写/查词/翻译服务（#133）。

---

## 2. 唯一状态 Owner 与 Live Preview 投影编辑契约

### 2.1 状态所有权
`MarkdownEditorContent` 唯一拥有规范文档状态：
- `source: String`：经 LF 规范化（`\r\n` 与孤立 `\r` → `\n`）的权威 Markdown 源码，**严格保留 1:1 换行结构与前导/尾随空行**。
- `source_anchor: usize`、`source_caret: usize`：规范源码坐标下的 UTF-16 字素边界选区端点。
- `history: SourceHistory`：基于规范源码快照 `(source, anchor, caret)` 的 `undo` / `redo` 栈及单调递增的 `source_epoch: u64`（仅文本变更或受控事务触发历史步进；纯光标移动/展开折叠只更新 `source_anchor`/`source_caret`，绝不产生 Undo 步）。
- `identity: String`、`revision: u64`、`last_acknowledged: Option<(u64, String)>` 与 `MarkdownEditorVersion`：用于受控/流式更新校验。
- `field: TextFieldContent`：仅拥有当前可见投影 `projection.text()` 的 CoreText 排版、点击命中、选区高亮、preedit 临时展示及滚动画布。

### 2.2 支持的 Markdown 语法子集与 Live Preview 展开规则
采用 **Obsidian 风格块/行级 Live Preview 展开**：
- **Unfocused 状态**：`expanded_source_range = None`，全部块以折叠投影渲染（与阅读态一致）。
- **Focused 状态**：`expanded_source_range = Some(source_anchor.min(source_caret)..source_anchor.max(source_caret))`：
  - 若为空选区光标 `p..p`：满足 `block.source.start <= p && p <= block.source.end` 的块处于展开态；
  - 若为非空选区 `s..e`（含 `Cmd+A` 全选）：所有满足 `s < block.source.end && e > block.source.start` 的相交块均展开。这样跨块选区、`Copy` 与 `Cut` 均能直接复制/剪切**无损规范 Markdown 源码**。
- **各块与行内语法规则**：
  1. **Heading (`#..###### ` + 0..=3 前导空格)**：`TextRole::UiHeading`。折叠隐藏前导空格与 `#...# `；展开时完整显示源码前缀，光标可在前缀与正文间自由移动。
  2. **Bullet (`- ` / `* ` + 0..=3 前导空格)**：`TextRole::UiBody`。折叠时将前缀映射为渲染符号 `"• "`；展开时完整显示 `- ` / `* `。
  3. **Fenced Code (```` ```lang ```` ... ```` ``` ````，含输入中尚未闭合的尾部围栏)**：`TextRole::Code` + `SurfaceKind::Code` 背景。
     - 折叠态隐藏首行 ```` ```lang ````（及紧随换行）与闭合 ```` ``` ````（及前置换行），仅显示并着词代码体 `body_display` 与行号（若代码体为 0 行，仍保留 0 宽空 `Code` 槽位）；
     - 展开态保留完整首尾围栏行（围栏行使用 `c-code-comment` 着色且不占行号），`body_display` 按语言使用共享 `syntax::parse_code_slice` 着词并绘制行号。
  4. **Paragraph & Inline Constructs**：普通段落按行保留原始 `\n`（不折叠软换行为空格，确保行编辑 1:1 可逆），支持 3 种行内构造：
     - 链接 `[label](target)`：折叠态仅展示带下划线的 `label`；块展开态展示 `[label](target)` 完整语法并对 `label` 保持下划线与语义 `SemanticPart` 定位；支持 `⌘-click`、`⌘-Enter` 与 AX `Press` 发出 `RichLinkAction`。
     - 行内代码 `` `code` ``：折叠态隐藏反引号，内部 span 使用 `c-code-string` 着色；块展开态保留反引号并对整段着 `c-code-string`。
     - 粗体 `**bold**`：折叠态隐藏前后 `**` 标记；块展开态保留 `**`。

### 2.3 双向 UTF-16/字素映射与显示编辑回写
`MarkdownProjection` 维护按 display 与 source 同步单调递增、互不重叠的 `MapSpan { source: Range<usize>, display: Range<usize> }`（包含行间 `\n` 分隔符），保证：
1. `display_to_source(range)` 与 `source_to_display(range)` 对空范围、非空跨块范围及包含不可分割字符（CRLF、CJK、Emoji 代理对、ZWJ 序列）均校验证实位于 Unicode 字素边界，否则返回 `None`。
2. `apply_display_state(new_display, new_anchor, new_caret) -> (String, usize, usize)` 基于旧 `self.text` 与 `new_display` 做字素对齐的公共前缀/公共后缀 diff（`common_prefix` + `common_suffix`）：
   - 将显示态改动范围映射回精确 `source` UTF-16 替换区间 `edit_src`，保留未改动区域中所有被折叠的 Markdown 标记（如折叠链接目标 `(url)` 或未展开代码块围栏）；
   - 通过旧/新前缀长度与增量映射回规范 `(new_source, new_source_anchor, new_source_caret)`。

### 2.4 IME 组字与指针拖拽投影冻结不变量
- **冻结条件**：`self.field.surface.input.marked.is_some() || self.field.editor_captured`。
  - 在中文 IME（系统拼音 / 搜狗 / 微信输入法）连续 `SetMarked` 期间，或鼠标按下拖动跨块划选期间，**严格禁止重投影（展开/收起语法标记）**，避免候选窗跟随坐标与高亮矩形在半途跳变。
  - 在上述期间的输入同步仅更新非折叠内编辑或保持现有投影稳定；一旦收到 `Commit` / `Unmark`（`marked == None`）或 `PointerPhase::Up` / `Lifecycle::CancelInteraction` 解除冻结，立即执行一次状态对账并刷新 Live Preview 展开态与光标重映射。

### 2.5 受控与流式外部更新冲突契约
- `MarkdownEditorContent::check_external_source(&self, expected: &MarkdownEditorVersion, payload: &str, new_revision: u64, selection: ExternalSelection, undo: ExternalUndo) -> Result<ExternalValueResult, MarkdownEditorError>`（无副作用预检）
- `MarkdownEditorContent::apply_external_source(&mut self, expected: &MarkdownEditorVersion, payload: &str, new_revision: u64, selection: ExternalSelection, undo: ExternalUndo) -> Result<ExternalValueResult, MarkdownEditorError>`
- 语义规则：
  1. `expected.identity != self.identity` → `Err(MarkdownEditorError::WrongDocument)`；
  2. `new_revision < self.revision` → `Err(MarkdownEditorError::StaleVersion)`；
  3. `new_revision == self.revision` 时若规范化后的 `payload` 与已确认版本一致，返回幂等 `Ok(ExternalValueResult::Unchanged)`，否则 `Err(MarkdownEditorError::StaleVersion)`；
  4. `self.is_composing()` → `Ok(ExternalValueResult::Conflict(ExternalValueConflict::Composition))`；
  5. `expected.source_epoch != self.source_epoch`（即在外部更新滞后到达前用户本地已发生源码编辑）→ `Ok(ExternalValueResult::Conflict(ExternalValueConflict::StaleBaseline))`；
  6. `selection` 使用规范 `source` UTF-16 坐标并做字素边界检查；成功应用后推进 `revision` / `source_epoch`，按 `undo` 策略保留/入栈/重置 `SourceHistory`，仅在未冻结时重投影。

---

## 3. AgentKit `AppHost::others` 装配示例（Markdown + Diff）

```rust
use tekes_ui_app_host::{AppWindow, LocalContainer, OthersOptions, PresentationOptions};
use tekes_ui_controls::{
    composition::{ComponentId, Content, MarkdownEditorContent, RichContent, RichDocument},
    extended::DiffView,
};

let md_editor = MarkdownEditorContent::new(
    "agent-notes",
    1,
    "# Plan\n\n- Review `markdown_editor.rs`\n",
)?;
let diff = DiffView::between("let v = 1;\n", "let v = 2;\n");
let diff_viewer = RichContent::new("agent-diff", 1, RichDocument::from_diff(&diff))?;

let notes_panel = LocalContainer::new(
    "agent-markdown",
    "Notes (Markdown)",
    Content::new(ComponentId(5001), md_editor),
)?;
let diff_panel = LocalContainer::new(
    "agent-diff",
    "Diff",
    Content::new(ComponentId(5002), diff_viewer),
)?;

let app = AppWindow::from_containers(
    vec![/* focus & nav containers */, notes_panel.into(), diff_panel.into()],
    PresentationOptions::new("agent-focus")
        .others(OthersOptions::new(vec!["agent-markdown".into(), "agent-diff".into()])),
)?;
```

---

## 4. 阶段执行拆分与验收映射

| 阶段 | 执行票 | 范围与依赖 | 有限验收与验证命令 |
| :--- | :--- | :--- | :--- |
| **M1** | [#362](https://github.com/TekesApps/TekesUI/issues/362) | 抽出共享 `syntax::parse_code_slice`；实现 `MarkdownProjection` 双向 UTF-16/字素映射、行内语法与块/行级展开模型。无前置执行票依赖。 | `cargo test -p tekes-ui-controls --locked --offline -- markdown_projection` |
| **M2** | [#363](https://github.com/TekesApps/TekesUI/issues/363) | 实现 `MarkdownEditorContent`、`SourceHistory` Undo/Redo、IME/拖拽投影冻结、`check/apply_external_source`、`precheck_dispatch` 与表面/审计清单。依赖 `#362`。 | `cargo test -p tekes-ui-controls --locked --offline`、capabilities/composition/Gold7 门禁 |
| **M3** | [#364](https://github.com/TekesApps/TekesUI/issues/364) | Gallery `component-markdown` / `component-diff-view` Playground 演练场 + `AppWindow::others`（Markdown 编辑 + Diff 只读检视）集成测试 + 组件文档更新。依赖 `#363`。 | `cargo test -p tekes-ui-app-host --locked --offline`、`cargo test -p tekes-ui-gallery --bins --locked --offline -- apphost_`、`cargo clippy --workspace --all-targets --locked --offline -- -D warnings` |

*注：各阶段完成并验收通过的物理实测范畴仅限源码、CPU 单元测试、集成测试与门禁脚本；真机物理 IME 候选窗随行（#12）与 VoiceOver（#5）继续在各自专项追踪，不因本设计关闭而代签。*
