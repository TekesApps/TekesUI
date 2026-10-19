# #17 共享编辑按键契约及多行最小场景（2026-10-18）

状态：公共最小契约和本批修复已实现并验证；#17 整体仍为部分完成。全部现有 WIP 是基线，不以 bc3de79 单独重现。未执行 Git 提交、暂存、推送、tag、Issue 更新/关闭或发布。

## 根因和共享位置

1. 公共输入丢失修饰键/repeat/up，文本服务回调是否送达也不等于编辑器是否处理。复用已验证的平台 Started/Finished 来源，在现有 composition/Component/ContentHost 内补齐 ContentKeyEvent、TextKeyOutcome 和编辑确认。
2. ContentHost 在编辑器之前截获 Tab。现在先给原组件处理，未处理时才走原有焦点遍历；没有增加第二套焦点系统。
3. 焦点 A-B-A 不能恢复旧原生按键的所有权。原有 focus 方法维护代次；按键链记录原组件和代次，迟到的 Commit/Command 不修改新焦点，重放/乱序结束也不触发第二次动作。
4. 多行 TextSurface 的最低高度能力已经存在，Field 适配器未接入，导致内区只有8pt、文字行需要20pt。复用 minimum_outer_height 和 FieldPresentation；测量、排列、绘制使用相同槽位，不新增 token、样式或布局框架。
5. 编辑后 reveal 使用旧段落，最新光标仍在视区外。修正原 TextSurface.reveal：只有段落过期时保留一次请求，实际重排后完成；后来发生的滚动优先。此修复同时覆盖单行水平和多行垂直滚动，未在页面上补偿像素。

适用组件：TextFieldContent、正确转发 key_event 的消费者包装器（已接入 AgentText），以及使用 TextSurface.reveal 的输入族。实际回归样例包括独立编辑器、Combo 既有组字边界、Gallery 和 AgentKit 原包装器，不以一个样例代表全库完成。没有扩大组件迁移范围。

## 公共接入及合理例外

```rust
field.surface = TextSurface::new(value, true, false); // 复用现有显式多行模式
field.tab_policy = TextTabPolicy::Insert;
field.submit_policy = TextSubmitPolicy::CommandEnter;
```

默认是 Navigate Tab、None 提交策略。Insert 适用于多行，Backtab 消费但不做自动反缩进；单行继续导航。Enter/CommandEnter 提交策略只产生意图，重复键不重复提交；不配置策略不会发送。是否发送、目的地与校验属于消费方。

原生宿主必须启用 Window::set_text_key_events(true)，按原序转发 Started、原回调/Key、Finished；独立示例 support/native_key.rs 演示纯数据适配。不能从 Finished 回放文本或再次调用文本服务。无按键来源的 TextCommand 可以编辑，但不会猜成提交；裸 KeyInput 只用于未交给编辑/IME 的控制输入，不能同时重放已消费的回调。

平台 TextKeyDisposition 描述回调分发；composition 的同名枚举描述实际路由确认。结束响应的 text_key 带原按键 ID、目标与 InputConsumed/EditHandled/Submitted/Navigated/Unhandled/Cancelled。Submitted 同时带 activated，仍不是业务送达。旧显式 ContentResponse 字面量须补 text_key:None 或使用 Default/consumed；消费者包装器需像 pointer_event 一样转发 key_event，才能保留修饰键。

组字开始状态、当前 marked text、实际 Commit/Preedit/Unmark、已处理编辑命令和无回调的 IME 消费都阻止按键再次落入提交/导航。标准换行和 Tab 控制效果在链结束时最多执行一次。边界删除/空 undo 等已识别的无变化编辑命令仍明确确认处理。组字中的裸 Enter 保持未处理给 IME，host 的原有组字屏障隔离浮层和焦点。

本批未把整个编辑器宣称完成：现有纵向及行边界编辑命令仍需原 #17 后续核验；#18 受控事务、#20 Fill/Grow/尺寸策略、#21 动作目标来源尚未补齐。AgentKit demo 只接来源/元数据桥接，仍保留显式按钮发送；多行 Composer 和键盘提交随受控草稿/视口接入，不能把包装器测试当作该产品行为完成。

## 分层证据

| 层 | 当前证据 |
|---|---|
| 公共 API / 确定性 | 共享控件/文本359；独立 consumer74；其中15项新增编辑契约测试；平台未改动，独立包合计395同时重跑平台36 |
| 新分支 token | 原8探针4500场景输入哈希全部匹配；另72个真实单/多行和换行/Tab/提交策略场景，4主题×3既有品牌；complete=false，未声称完整数据流提取 |
| 真实消费 | AgentKit UI workspace15、headless76、严格Clippy和Rust边界；原AgentText真实key_event转发测试，显式源码清单eb2f5bc0dae0ce051c56b5aca0d2bb31d600f4e7be4be08d48d82bc0076effb0 |
| 原生/GPU/Gold7 | Gallery真实AppKit→公共ContentHost→TextFieldContent→Metal绘制；同样7条按键链：一处普通换行、一处Shift换行、一个Tab、一个提交意图；最新y和光标可见。单行默认Tab/Backtab为8604→8601→8604，Enter/CommandEnter不提交。裁剪与光标滚动前后PNG、逐步JSON、签名后二进制和运行时源码指纹分开保留 |
| 门禁 | 45组件不变；公共类型新增6个支持类型；能力38项/组合6项、默认Gallery构建、strictClippy、参考对齐及纯Rust通过；Gold7脚本只证明机械门禁 |
| 未验收 | 物理中文拼音、VoiceOver、AgentKit原生窗口、多主题最终体验；本批原生Gold7仅Light最小场景。性能按要求不测 |

## 补丁与依赖

`design/reviews/editor-keys-20261018/patchset` 包含16路径公共补丁和5路径AgentKit桥接补丁；明确隔离既有 WIP 前置依赖。独立目录实际运行395项共享/平台回归、74项consumer、16项feature编辑测试；应用补丁后245文件逐字节匹配被测快照。不是完整#15–#21交付，也不是公开不可变commit/tag。

AgentKit使用 canonical-runtime-source.tar.gz：其原始Cargo.toml及逐文件内容与公开清单匹配。独立验证包则使用显式子工作区和子集lock，两种用途分别标明。原始8探针的中间采样保留为开发记录，当前有效证据只取 test-summary.json 中列出的8条路径及新增72场景。

下一步在完整已验证批次上推进#18；#17剩余命令/物理验收继续挂原Issue，#20最小高度和reveal已修复但完整视口契约未完成。完整#15–#21可复现交付就绪后再移交AgentKit会话，之后回到其余整改。
