# #88 SearchFieldContent accepted finite design

# #88 SearchFieldContent 有限设计候选（待 main 接收）

日期：2026-10-24（Asia/Shanghai）。源仅为 `/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI`，HEAD `1a7614a273e1a897ddc8a7ad8cb41d303a4a1290` + 既有 WIP。本目录独占 QA；没有生产实现、构建、GPU、commit/push、正式文档或 Issue 更新。`source-manifest.json` 记录所读完整文件 SHA256、片段范围和逐文件采样时间；`source-excerpts/` 为带原行号片段。它是设计取证，不是可构建全树快照，也不声称整个 WIP 在同一瞬间冻结。实施前须重核共享文件差异，由 main 串行接入。

## 1. 权威范围与 Issue 现状

权威原文保存在 `issue-88-authority.json`；GitHub 只读复核结果在 `issue-88-current.json`。本次读取时 open、0 comments，正文与指定快照相同。父 #56，关联 #55/#65。五项全文如下，不缩减、不开新关闭前提：

1. 有限设计固定原SearchField与TextFieldContent/TextSurface单IME owner的复用边界、query/submit/clear命令及动作来源/mount规则；不造搜索引擎、异步结果或下拉。
2. 公共SearchFieldContent挂原Host，search/clear各一次，clear按原策略保编辑焦点；typing/selection/marked/Backspace/只读/disabled和原send gate不变，无第二编辑状态机。
3. inherited Surface/Theme/clip/生命周期、有限约束/非法更新原子拒绝；卸载/替换后旧命令拒绝，原reading按适用范围保持。
4. Search可访问语义如需新增明确角色/bridge和兼容策略；不能将普通TextInput冒称已具search特有语义。独立publicconsumer、原View参照、必要真实GPU与原输入回归/strict通过；分层未验回填。
5. 公共库存/Workbench/设计/父票同步。

#41 CJK 注入归因独立，本候选既不解决它，也不以整票关闭作为前提。没有联系或唤醒其他会话，没有委派。仅以本会话 final 交 main。

## 2. 实际原模型、View 与调用链

下面均为读源观察，未运行：

| 真实入口 | 链路和已存在行为 | 不能据此声称的能力 |
|---|---|---|
| `text_demo.rs:582` `TextDemo::search()` | 单个 `Surface::new("", false, false)`；Surface 内只有一个 `view: TextSurface`，通过 Deref 访问其 TextInput。`query():685` 直接读取它的 text | 没有名为 SearchField 的独立 Rust 模型/View，也没有 SearchFieldContent |
| `build_search_clipped:691` | 原 layout_preserving_caret_clipped → TextSurface draw；空且未焦点时显示“按名称搜索…”；原 ancestor clip/selection/caret | 未实现 search/clear 按钮或专属 submit/clear 事务 |
| `gallery/mod.rs:1133,1628` | navigation_search.query → sidebar.set_query；过滤归 Gallery 所有 | 查询消费不属于新 Content，不能复制搜索引擎 |
| `pages/showcase.rs:1196` `Field::Search` | 静态 Painter 标本，search icon + 原 token/inset | 不是活动输入控件，不可拿状态标本作 search/clear 运行证据 |
| `composition/adapters.rs:179` | TextFieldContent 保留 Field/FieldView、一个 TextSurface、可选 ReadingController；measure/arrange/paint/event/key/native/AX 都归这一编辑器 | 当前 semantics 为 TextInput，不是 Search 角色 |
| `design/gallery/workbench.json:355` | 登记 public API 为 TextSurface，docs 写 controlled filtering / clear | 文案不是已经交付 clear 的证据；实施时必须纠正登记并接真实 consumer |

公共输入链：原 Host `event` → `key_routed_event` → `TextKeyStart/End` 的 mount/focus_epoch/native binding/IME barrier → TextFieldContent `key_event` / `event` → TextSurface `command` 或 TextInput `insert/preedit/select_range`。Text queries 走 `text_surface`、`ContentTextInput::from_surface`、`semantic_text_layout`；不得由包装器缓存另一份 text/selection/marked/undo。

原 `TextSubmitPolicy::None/Enter/CommandEnter` 保留。TextFieldContent 默认 None；标记输入中的 raw Enter 交 IME、只读 Enter consumed 无 submit、repeat 不激活；legacy ContentEvent::Key 的 Enter 不自行绕过原 key provenance。Host 的 TextKeyEnd 在 composing、input_context_consumed、已有 edit/text callback 或失效 owner 时不二次 fallback；委托链只确认原结果，不造父动作。这里的 send gate 是提交意图边界；业务发送由调用者决定，本票不增加网络发送。

## 3. 拟议公共 API（草图，非现有符号）

```rust
pub struct SearchFieldContent { /* private retained TextFieldContent + original Button clear */ }
impl SearchFieldContent {
    pub fn new(name: impl Into<String>, query: &str) -> Self;
    pub fn query(&self) -> &str; // borrow original input.text
    pub fn input(&self) -> &TextInput; // inspect same owner
    pub fn set_submit_policy(&mut self, policy: TextSubmitPolicy);
    pub fn set_availability(&mut self, read_only: bool, disabled: bool);
    // Checked controlled publication; delegates original baseline/revision contract.
    pub fn apply_query(&mut self, expected: &TextEditBaseline,
        value: &DraftRevision, selection: ExternalSelection,
        undo: ExternalUndo) -> ExternalValueResult;
}
pub enum SearchCommandKind { Submit, Clear }
pub struct SearchCommand { /* opaque target + mount + native binding + baseline + kind */ }
impl ContentHost {
    pub fn capture_search_command(&self, id: ComponentId,
        kind: SearchCommandKind) -> Result<SearchCommand, SearchCommandError>;
    pub fn execute_search_command(&mut self, command: SearchCommand)
        -> Result<ContentResponse, SearchCommandError>;
}
// Existing Submit action kind remains the submission intent.
// Add exactly one Clear kind, and immutable Search payload (kind + query at delivery).
```

私有 editor 防止将 multiline/reading/presentation 任意改成不合法的 Search 配置；不开放可变 editor 整体替换。构造保单行、inline metadata 隐藏、原 Field chrome 与 TextSurface UiBody。默认 submit policy 建议保持 None 以兼容原编辑别名；独立 consumer 明确设置 Enter，并同时证明 CommandEnter/None。未配置 submit 时键盘不偷偷发起搜索；显式 Programmatic Submit 是独立调用意图，不伪装为键盘 Enter。`apply_query` 是调用者授权的外部发布，保留原 read_only/disabled 不阻断外部同步的契约；它不伪装用户 clear、不发 Submit/Clear 动作。相同已确认发布的 Unchanged、Composition/StaleBaseline/ForeignEditor 等结果完全沿用原 API。

`SearchCommand` 为一次性消费、非 Clone 的同步 UI 命令，不持有业务 Session。capture 仅取证不授权执行；execute 再核窗口活动、当前 scope/active/disabled/read_only、target 类型、mount、native binding、完整 external_baseline 相等及无 marked。使用原 baseline 比较可拒绝 text/edit→undo ABA，不再造 query 版本计数器。失败先完成全部检查再返回，不动队列/文本/焦点/capture。卸载/替换/同 ID 重挂即拒绝；reorder mount 不变且 baseline 未变可继续。重复已经消费的命令在 Rust 所有权上不再可用；新捕获的新 Submit 是新的调用意图。实现若需要可跨 wire 的命令，不得直接序列化此本地凭据，应先交 main 另审最小依赖。

## 4. 动作、clear、输入与焦点

| 输入/动作 | 有限行为与来源 |
|---|---|
| typing、selection、marked、Backspace、剪贴板、undo/redo | 原 editor 原样委托；不拦 Backspace 改成整框 clear；不把每次 changed 当 Submit/Clear；UTF-16/字素归原 TextInput |
| raw/native Enter | 委托 TextFieldContent 与原 TextKey gate；仅真实 activated 映射既有 Submit 一次，来源 Keyboard；payload 捕获当时 query |
| clear 指针 | 一个普通共享 Button 拥有 press/drag/cancel；在可见合法 hit 内 down/up，原 pressed/capture 状态机完成后清除一次；来源 Pointer |
| clear AX Press | 稳定语义 part，只在当前 clear 可用时接受；同一 clear 事务一次，来源 Accessibility；用现有 semantic_part_preserves_activation_focus=true |
| 程序命令 Submit/Clear | 通过 Host 再授权入口，来源 Programmatic；不得假调用 AX Press 或伪造 Keyboard 来源 |
| clear 事务 | 非空、可编辑、未禁用、未 marked 才执行 `input.insert("", Some(0..input.len()))`；保留同一 editor/binding、正常单步 undo、selection 0..0；清 clauses 并原 reveal。不 new TextInput、不直接赋 text、不 external reset |
| clear 空值/只读/disabled/marked | 拒绝，无动作、无隐式 unmark/cancel、无焦点改动；隐藏/禁用 clear 的发布与 hit/AX 一致 |

clear 策略为保持既有编辑焦点：已聚焦 Search 时按下 clear 不触发 editor blur，不提交 preedit；clear 完成后 Host focused ID/native binding 不变。不抢其他编辑器焦点；另一 owner 正在组字时辅助 clear 默认拒绝，避免借保焦点跨 owner 改草稿。Search 自身未焦点时，clear 可以在同一已授权 scope 清非组字查询，但维持原焦点；是否需要点击后获得 Search 焦点由 main 接收时确认，不将其默认为新行为。原源码没有现成 clear 策略可引用，本段是任务给定保焦点要求的最小落地。

Button 可以作为包装器内部保留模型/控制实例和固定语义 part（参照 NumberFieldContent），不再挂第二个文本 ContentHost。原 Component `preserves_activation_focus()` 是 event-time 钩子：仅 clear 命中路径为 true；编辑区域继续原 focus/pointer 行为。AX 使用对应 part 钩子。cancel/unmount 清 Button 手势与派生 action payload，不清 editor 草稿；重新挂载凭原 Host 分配新 mount。不要调用整个 Host.cancel 来实现 clear，它会失焦并取消其他内容。

所有动作在原 delivery 边界入 `ContentActionRecord` 一次；不得 paint、poll query 或 drain 后重发。保留 action source、mount 与 immutable delivery query。drain 沿原 mount/current authorization；Search payload 的 current 检查只验证当前可用性及相同 editor/document binding，不能因后续合法 typing 改变 query 而重写/误删已接受 Submit。状态改为 readonly/disabled/marked 或移出 scope 时应拒绝未交付的 Search 请求；这部分必须显式挂到 `action_payload_current`，原默认 true 不够。

## 5. Theme、几何、约束与生命周期

沿原 TextFieldContent 的 field_layout → layout_surface → paint，不复制 TextSurface 绘制。继承 `ContentTheme.surface()` 和 palette；Field/TextSurface/clear Button 使用原 Surface resolver，未知 Surface 在现有 setter/validate 阶段拒绝。没有新 token 或新 Theme schema 依赖。

clear Button 宽度来自其原测量与已有 spacing，先扣除其占用再将剩余 slot 交原 editor；按钮与 editor 共同服从父 frame/clip。默认保持 editor 原 chrome/高度，不为接入整 DS 重设圆角/inset。原 Search icon 来自静态 specimen，不是活动 Search 输入既有 View 的一部分，本次不以它新增必需绘制抽象。空值未聚焦提示若要保留则从原 Gallery 字串作为显式 placeholder/host copy 传入，使用原文本 renderer，不能塞进 query；不得硬编码此中文为库默认。

约束边界：可用尺寸必须有限且非负；clear 占位大于可用宽度时 editor 宽度归零并依原 clip 隐藏不可见 hit，不溢出父级。不自行发明 fixed min width。新增布局参数如果没有需求就不公开；以后需宽度/按钮策略 setter 时先验证候选，拒绝 NaN/Inf/负值后再提交。`validate()` 保单行和可用服务，Host 原 text service 缺失错误保持。非法更新先用原 controlled preflight 检查，不调用 cancel_subtree/换 mount 后再发现错误。Surface setter 继续先 canonicalize 再赋值。

Reading：Search 单行查询不创建 ReadingController。保留同 Host 的 readonly transcript/reading sibling，clear/submit 不通知其 follow_latest、不卷回旧 caret、不取消其 selection。Search readonly 仍通过原 editor 允许复制与选区；不冒称拥有多行 reading。原 TextFieldContent/ReadingController 共享代码不作 Search 特判，回归其既有 reading 用例即可。

## 6. 必要最小语义/平台/wire 依赖（交 main 接收）

现状：ContentRole 与 wire::SemanticRole 只有 TextInput；平台 Content project 和 AppHost project 均映射 TextInput→TextField，macOS AccessibilityRole 只有 TextField/TextArea。仅改 accessible name 为“搜索”无法成为 Search 特有语义。

建议一个有限依赖 hunk：增加 `ContentRole::SearchInput`、wire `SemanticRole::SearchInput`、native `AccessibilityRole::SearchField`。macOS 对外仍使用 TextField role，并发布真正的 SearchField subrole；实现前在本机 objc2 SDK/bindings 核对该 subrole 常量/selector，当前设计未验证系统行为。所有 `TextField | TextArea` 原文字选择/查询/编辑 selector capability、值/选区/text layout 和 readonly 路径必须把 SearchField 当文本 owner，不可只加 role 字串导致原文字服务丢失。`enrich` 不可将 SearchInput 错变多行；私有 editor 已约束单行。

必要链：controls role/interactive → module-runtime/content_accessibility 导出 → app-protocol enum → container_semantics 验证/导入 → platform-macos/content_accessibility project/enrich → platform-macos/accessibility role/subrole/text selectors；AppHost 自有 project 同步，避免其 `_ => Group` 吞掉 Search。clear 复用已有 Button part/Press 与 preserves_activation_focus wire 字段，无新增 Clear AX action 或另一文本 owner。

兼容策略：不改普通 TextInput 的编码与默认行为；新 SearchInput 要求发送者、ProcessContainer 接收者同版支持。旧 peer 对未知 role 必须显式拒绝整个 snapshot，不能降级成 Group/TextInput 后声称 Search；新增 roundtrip、unknown role 和原旧 TextInput 兼容测试。原 envelope VERSION 是否保持由 main 依据现有 role 扩展策略接收；此处建议沿既有 Tree/Tab 的匹配 schema 策略，不擅自扩大为通用协商框架。没有必要新增 wire Submit/Clear 业务协议，本地 typed action 仍由原 consumer 消费。

若 main 要先拆票，最多拆出“SearchInput 最小语义投影与 wire 兼容”依赖；其余 adapter/consumer/库存保持 #88。当前不自行建票。依赖未接收前不能伪造 Search 特有语义或开始生产实施。

## 7. shared hunk 归属与接入顺序

| 拟议 hunk | 内容 | writer |
|---|---|---|
| `composition/search.rs` 新文件 | retained editor + clear Button + 输入委托、命令/事务、语义 part | 后续独立候选，main 接收后 |
| `composition.rs`、`composition/actions.rs` | export/SearchInput、最小 Host 命令入口、Clear kind/Search payload/current gate | main 串行；不由候选并行写 live |
| protocol/runtime/native/AppHost 上列文件 | 精确 Search role 链、旧 peer 拒绝、全部文本 capability 保留 | main 接收有限依赖后串行 |
| `examples/composition-consumer` 与 tests | 纯 public API 独立 consumer，原 Host，无私有模块 import | 后续独立候选 |
| Gallery Workbench、inventory、registry test、正式设计/父票 | #88 精确绑定新 public API、真实适配页/动作日志、能力及未验同步 | main 串行 |

只在必要时给 TextFieldContent 增最小内部 delegation/helper（例如 placeholder 绘制），不能复制 `keyboard/input_event/layout_surface` 或改其既有 submit 默认。现在没有判定需要拆 TextSurface/Theme/整 DS。

## 8. 后续验证计划（本次全部未执行）

| 五项 | 必须给出的实际证据 | 实施时断言 |
|---|---|---|
| 1 | 本设计、原源锚点；main 接收记录 | 仅一个 TextInput/IME owner，无搜索业务状态/异步/下拉 |
| 2 | 独立 public consumer 挂原 ContentHost 的实际 event/program/log；controls 输入回归 | 设置 Enter 后 search 一次，clear 一次；两次 drain 无重放；Pointer/Keyboard/Accessibility/Programmatic 各来源；None/CommandEnter、repeat、Start/End、IME consumed/marked 的零次或一次正确；typing/UTF-16 selection/Backspace/undo/readonly/disabled |
| 3 | consumer lifecycle/atomic tests + 同 Host reading sibling 回归 | stale target/mount/binding/baseline、remove/reinsert/replace/hidden/modal/inactive 拒绝；非法更新完整状态不变；reorder 合法保持；clip、有限窄宽/零宽；clear 过程中 focused ID/native binding 不变，readonly sibling reading 位置不变 |
| 4 | public consumer real-GPU + 原 View 参照 + role/wire/platform 单测；原 input 回归和 strict | 真 GPU 像素不能由 Scene 数量代替；SearchInput 保真实 text services；旧 peer 显式拒绝；见下方矩阵 |
| 5 | inventory/Workbench/正式设计/父 #56 与 #88 回填 diff | 新类型可从 public import 使用；库存登记真实 query/submit/clear/层级未验；附结果与 PR（若有），不虚构链接 |

GPU 必要矩阵有限为新 adapter 与原活动 `TextDemo::search`/原 `TextFieldContent` 对照：同字体、同 frame、同 Theme/Surface、同 query/selection/marked；保留原 field chrome/edit region 像素，clear 新增区域单独界定。用真正 CoreText host/shaper 和 `tekes_ui_render::Renderer::new_headless` → `render_to_rgba8`；原 View 对照由 Gallery 原 text_demo 模块受控测试入口采集（静态 showcase 不替代），新 adapter 由独立 consumer 采集。保存 actual 输入、driver 程序、命令、stdout/stderr、exit code、图片与 SHA；所有失败也保留，产物使用全新证据目录，不覆盖本设计或旧 receipts。

覆盖 Theme::ALL、实际继承 Surface（canvas + 一种不同背景容器）、@1x/@2x、默认/大字号、正常/窄/部分 clip；状态取 empty unfocused、nonempty focused、selection、marked、readonly、disabled、clear press/清除后。静态原 Search 缺 clear 不要求像素相等：对照 editor/chrome，新增 Button 按原 Button View 对照；clear 前后输入结果以动作日志和原 editor 状态核对。记录实际 case 列表与失败，不预设“全 DS/Gold7 通过”。

适用回归入口（名字是现有文件，不声称现已运行）：controls `input_contract`、`action_contract`；consumer `editor_keys`、`native_key_translation`、`text_mount`、`content_actions`、`controlled_text`、`input_surface`、`text_viewport`、`reading_stream`、`controlled_reading`；platform `content_accessibility`；runtime wire roundtrip/容器验证；registry public API。strict 为受影响 crates 的 Clippy `-D warnings` 和适用格式/设计门禁，不能拿性能脚本的 strict 当 Rust/输入通过。

main 批准实施与 writer 时段后才使用原 `scripts/tekesui_build.py` 的 `build_lock()` / `environment()`，toolchain `+1.95.0 --locked --offline`、原 target/review-cache；本设计不调用 helper、不读取/运行 target、不 prepare/clean/install、不新 target。共享源冻结后再运行，后续修改需新证据身份，不刷新旧 receipts。

GUI 真实交互、物理 IME、VoiceOver、全库性能、全 DS、公开发行均本次未验；不增加为本票关闭前提。语义 bridge 如只有纯投影/selector tests，应明确写“系统 Search subrole/VoiceOver 未验”；GPU 不证明原生输入法。#41 依然独立，人工/物理输入失败要原样分层记录，不推论其归因已修好。

## 9. 交付与阻碍

设计可交 main 评审；生产实施仍未授权。需要 main 接收两处明确新增契约：SearchInput 最小语义/wire/platform hunk，以及本地 mount/baseline 授权命令 + Clear 动作 payload。没有源码缺失或 SSD 构建阻碍；本阶段禁止构建，因此无运行验收结果。待接收后按上述五项推进并回填 Issue；本次没有向 Issue 写入设计或关闭任何票。


## Main acceptance amendment (2026-10-24)

Main accepts this finite design after advisor source review. Original five acceptance criteria remain unchanged. Only isolated implementation is authorized; no implementation/native acceptance is claimed. Earlier pending wording above is historical.

- SearchCommand is a private non-Clone local delayed request, consumed by execute. Capture is read-only. Execute checks existing target/mount/binding/baseline/activity/scope/no-marked before any side effect, then delivers once through the existing action queue with its real source. No nonce, timer or second input engine. Accepted payload-current checks owner/availability/binding, not full query baseline: normal later typing must not erase a previously accepted Submit, whose query is immutable.
- Pointer clear preserves focus only for the current routed event. Reset the transient policy every relevant event; a previous clear must not affect a later editor click. AX uses existing part-specific hook. Do not steal another editor focus; refuse clear while another owner has marked text.
- Split the SearchInput semantics closure into a dependency issue: controls/wire/container/platform/AppHost plus real text capability, not a new search model. Verify actual macOS SearchField subrole and text selectors in implementation. Existing active Search is the TextSurface/TextFieldContent chain.
- Follow existing Tab/Tree matched-schema strategy: keep envelope VERSION and old TextInput encoding unchanged. Old peers explicitly reject the entire unknown SearchInput snapshot. Prove this with a real old-schema decoder, and prove container validation completes before publishing any semantic state. A new enum rejecting random strings is insufficient. No generic negotiation framework.
- The #88 adapter can be prepared in an isolated candidate before the semantics dependency is accepted; final integration and closure wait for it. #41, all-library native and whole Design System completion are not added prerequisites.


## Main correction: preserve original remote failure authorization (2026-10-24)

Advisor verified the real ProcessContainer owner: container.rs accepts a newer content snapshot even when RemoteSemantics::update fails. Thus the coordinator-added #101 phrase "failure leaves current semantic snapshot unchanged" was incorrect: retaining usable old A could authorize callbacks against content B. This corrects that additional assumption, not any original #88 deliverable.

Publish only after whole-snapshot decode, validation and import; no failed candidate prefix is published. Old authority follows original dirty/revoke policy: decoded validation failure revokes entries/snapshot/attachment; import-ID capacity failure keeps at least dirty barrier, no half-publication or allocator wrap (local staged next may remain unchanged); decode/unknown role follows actual request-owner failure/dirty handling. Preserve original disconnect, hide, deactivation, cancel and retirement revocation. Complete valid recovery C must not revive revoked callbacks. Do not cancel a neighboring editor or invent a second failure engine.

Replace the earlier #101 keep-A assertion with real A->malformed-B, ID-capacity, old-schema SearchInput decoder/owner failure, recovery/disconnect/inactive stale-target tests. Prove old callbacks/text layout cannot authorize after failure, candidate prefixes never publish, monotonic identity remains, and neighboring Composer is untouched. Historical incorrect candidate and tests remain separately archived; they do not count as accepted evidence. All other Search role, single text owner, selectors, clear/submit, input and compatibility requirements remain.


## Main #101 finite bridge acceptance — 2026-10-24

Revision2 necessary19 hunks are integrated with newer DataTable/NumberField WIP; five shared files were merged by context, not whole-file copies. Main10 lanes pass:153 lib tests, old-schema7, platform5 plus337 actual native-object checks and originalLink6, AppHost5, controls295+12+2, public consumer63 (one original expensive GPU test explicitly ignored), currentTable19+14, strict2 lanes andGallerycheck. Total575 ordinary test invocations, not575 unique behaviors. Original matched schema/outer VERSION and TextInput encodings preserved; real old complete decoder rejects SearchInput whole snapshot. SearchInput uses original TextFieldContent/TextSurface owner/binding, TextField role+SearchField subrole and existing text/selection/read-only selectors.

Malformed decoded B revokes entries/snapshot/attachment; import capacity error keepsdirty, no prefix/wrap; validC doesnot revive oldprojectedIDs. Original container may accept content B evenif semantic validation fails, so historical keep-A assertion remains rejected. Owner tests are bounded realmethods/fixtures, notOS-child disconnector negotiated workertransport faulttests. Nativeobjects windows=0; noNSWindow, physicalIME, VO/GPU/performance/release claim. NoPR/commit/push.

Root evidence: design/reviews/search101-main-intake-20261024/root-acceptance.json, source-stage.json, root-lane-verification.json. Main independently verified24 immutableprogramarchives and187 dep-infoSHA; source1600inputs/runtime stableperlane. Failed rounds retained: main initially omitted established --test-threads=1 and exposed known globalMAX fixture interference, nowtracked#106; separate nonexistentintegrationtarget command was corrected before tests, no productionchange tohidefailure. Final successful lanes span SSD artifacts/main-scene-notice-20261024/search101-main-01(first4),02(next4),03(last2). Canonical#8 observation refresh and fulltokenextraction remain separate; nohistoricalrehash.

APIinventory ContentRole reason andWorkbench dependency note synchronized; registered Search route notpromotedtoadapterdone. #101 originalfive accepted; #88 adapter/clear/submit/GPU remainsOPEN andnowcanbuildagainstacceptedbridge. Selectbridge#105 andselectionbridge#107 maystartfromcurrentacceptedrolecontract; theirroles/behavior arenotprovenby#101.


## Main actual Workbench integration (2026-10-24, in progress)

Only component-search-field/playground replaces its TextDemo owner with private StoryInput::Search(ContentHost + SearchFieldContent). Other variants, Gallery sidebar and Icons keep their original owners. Search supplies original native TextKey markers through existing transport; clipboard claims, composition, focus/capture and one-shot actions remain original Host operations. Search never falls back to detail editor when unfocused or rejected. Inline clear is the original adapter Button; obsolete standalone id22 is removed.

Native nodes, valid shaped UTF-16 layouts and actions all use the accepted original content_accessibility bridge. A private identifier projection translates complete original (mount,part) keys into a checked monotonic high Gallery domain disjoint from existing handwritten low keys. It owns no edit/semantic/action state. Nodes, parent links, layouts and reverse actions share the mapping. Current-node disappearance prunes mappings; page and activity changes explicitly revoke them. No universal unobserved hide/restore ABA guarantee is inferred from snapshot pruning. Original Host availability/exposure controls remain authoritative. Capacity failure publishes no partial scope; consumed unpublished identifiers are not reused. Window publishes layouts before corresponding nodes and removes them on route exit. Sidebar routing precedes Workbench full-key dispatch; original cross-owner marked gate remains, same-Search capability decisions go to the original Host.

Static immutable snapshots explicitly borrow the existing finite TextEngine fixture document cache; live editable documents retain original per-TextSurface layout caches. This does not add editor state or a global cache for typed strings. Main round08 passes 7 native-AX route tests +1 field/area/search family test +8 real Workbench/cache tests; these are programmatic/CoreText evidence, not physical IME/VoiceOver/shown native-window acceptance. Earlier failing compile/test rounds remain unchanged. Public classification adds the five Search types; current four R1 type paths are classified as source-visible only, not accepted #98 behavior. Canonical full capability freshness still requires normal probes/generation; no historical observation is re-signed. #79 Gallery clock follow-up remains a separate original issue.


## Main finite acceptance (2026-10-24)

Original five Issue #88 criteria are satisfied on HEAD 1a7614a + reviewed WIP. Main root acceptance: design/reviews/search88-main-intake-20261024/root-acceptance.json. Actual Search Workbench routes, adapter/public API and current generated bindings are integrated; original TextDemo::search remains the View reference. Native TextKey mode is enabled only for the actual focused Search text owner. The original native tracker may refuse an in-flight mode switch; desired mode is retained and retried after Finished. A transport-only id-to-original-owner map returns Finished to its original Host after blur/page/sidebar change, without delivering it to the new Legacy owner or fabricating a Started event. Native tracker tests keep original nested/refusal policy.

Normal/external public consumers pass 36 calls each; normal/external adapter GPU each 756 conditions and original View GPU each 84 conditions (1680 total,4144 PNG). Explicit external-inputs is selected by public adapter lanes; original Theme-only TextDemo reference uses each artifact's legacy/default branch, with no claim of arbitrary Palette definition support. Latest Gallery all-target strict and consumer strict pass. Workbench9, AX7, original tracker6 and default/SVG bindings3+3 are actual main executions; earlier rounds/repeated calls are not unique behaviors. Physical IME/VO/shown-window/performance/release remain unverified.

Normal 18-group/6544-case bounded refresh,351 public types,448 generator inputs,generated fixture and main default/SVG compilation are accepted as finite metadata synchronization. Each observation remains complete=false; #8 full52 closure,#55 wholeDesignSystem,#79 clock follow-up and #41 CJK attribution remain independent open work. Failed rounds are retained, including full Gallery diagnostic443pass/10fail/14ignored (without GPU acceptance lock, not GPU acceptance); #79 clock regression is reopened separately, Search cache and obsolete AX identity assertions are corrected, and unrelated missing GPU output env is not mislabeled as an adapter failure. No PR or publication.


## #199 当前主线恢复接缝（2026-10-31）

恢复输入已在独立来源组装，基于当前 Combo/Text/Menu 修复前缀 02456a6；旧有限接收与旧 GPU 仍保原身份。Search 自身包装当前 TextField 的原 prepared decision，由同一个 editor 负责校验与应用，不直接把 TextField effect 当作 Search effect。prepared pointer capture 同步原 Search editor_captured，保留跨 clear 区域拖选与抬起归原 editor。Host 在 prepared 分派前复用原 search_pointer_blocked/search_ax_blocked，保持本设计另 owner 组字时辅助 clear 拒绝的原合同。新增公开回归仅是 source，尚未运行；当前编译、行为、GPU、native 与公开交付均未通过本说明确认。


## r4 接纳前缀修订（2026-10-31）

本轮以实际冻结 `8f274af682289b92400a29187c04bcfed3288c57` 的22-role前缀为准，仅新增 SearchInput/Select 至24角色。保已接Combo/五Selection角色、三Selection payload、Host退役与文本能力规则，以及SetExpanded和当前Combo can_show_menu fixture。旧r3全角色并集及其派生decoder不作本轮输入；旧peer直接捕获该22-role的完整生产decoder。主体/消费者/原业务算法及原AC保持，本轮仅重基必要接缝，仍未编译或执行行为验收。


## r4 实际接纳范围（2026-10-31）

本次候选固定在 `8f274af682289b92400a29187c04bcfed3288c57`：保留实际22角色及现有Combo/Selection，只新增SearchInput与Select至24角色。不采用r3的完整角色并集或其派生旧decoder。兼容fixture直接捕获该前缀的完整生产decoder，保留全部22角色和SetExpanded；只为同目录编译改一处module路径。原主体/consumer/Gallery及有限AC延续，仅必要接缝重基和已发现的Gallery未定义变量一行修正；未编译或执行行为验证。
