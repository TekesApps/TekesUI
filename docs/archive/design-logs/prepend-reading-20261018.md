# #16 · 版本化历史前插与阅读锚点

基于当前全部 WIP，复用原 ReadingController / TextSurface / ScrollArea。未提交、推送、打 tag 或关闭 Issue，不测性能。#13 总览、#14 公共组合契约、#12 统一验收仍是原入口。

## 根因、共享位置与范围

此前的 ReadingController 只在新文本以旧 shaped 文本为前缀时保护锚点；历史前插不满足此条件，走普通 offset clamp。直接用前插数据“页数正确”或滚动像素补偿都不能证明同一内容保持位置。用本轮开始时冻结的完整 WIP 旧 reading.rs、当前真实 CoreText 重放，前插位置误差为 **204pt**，见 legacy-red.log；不是从旧 bc3de79 报告推断的新缺陷。

共享修复在 controls/src/reading.rs：

- set_document 显式绑定调用者 key/revision，返回不可伪造的 ReadingDocumentVersion；每次绑定有全局唯一 attachment。A→B→A 和另一个 controller 即使 key/revision 一样，旧 token 也不能应用。新绑定按新文档清空选区/undo/scroll，保留 flags、文本角色与显式 follow 意图。
- update_document 接受请求发起时的 expected token、新 revision、完整文本及 TextRangeMapping 旧/新 UTF-16 保留区间。版本必须前进；无效身份、旧版本、绕过版本接口改写的 text、非法映射均在任何文档修改前拒绝。异步回调不能临时读取当前 token 来代替请求原 token。
- 映射区间必须按顺序、无重叠、两端均为合法字素边界，且保留内容逐字相同。纯文本消费者据稳定 item 范围或 prepend delta 提供区间，无需计算像素补偿，不需要 Session ID 类型。
- 方法在到达时捕获当前真实布局的 UTF-16 阅读位置及其视口偏移，避免恢复请求发起后已过时的用户阅读位置；pending logical anchor 支持同一布局前连续更新，复用原 layout/caret/scroll clamp。Mapped 表示逻辑映射成功，像素恢复发生在下一次布局。用户显式改变阅读意图可取消 pending；不自动 opt-in follow-latest。
- 锚点删除/初次无布局返回 FallbackStart，下一布局回到起点。选区必须有连续、同序的完整保留区间覆盖，保留正反方向和起点右/终点左亲和性；不能因文字碰巧相同而把未映射内容算作保留。失效明确返回 Invalidated，折叠到仍可映射的 caret，否则 0。边界使用原 ScrollArea 的合法 clamp。

发现并修复了新实现中的选区细节：仅比较端点与字符串会把“中间未映射、但文字相同”的选区误报 Preserved。新增编译后失败回归 selection-map-red.log，改为完整区间覆盖后通过。此缺口属于原 #16 选区验收，不新建重复 Issue。

适用于所有采用可选 ReadingController 的只读 Text/ScrollArea，以及 TextFieldContent.reading 的只读内容。真实 TextFieldContent 仍使用原表单 slot，本轮没有为视口尺寸临时改 arrange；通用尺寸契约仍由 #20 补齐。无新组件/滚动器/渲染器/动效/token/业务协议；6 个新增公开数据类型仅表达身份、映射、结果和错误，不成为另一层 UI。

合理例外：本入口是只读历史文档事务。可编辑输入返回 EditableSurface，marked text 返回 CompositionActive，绝不把它当 #18 的通用受控编辑同步；原 legacy layout/append API 保留。历史到达不是用户编辑 undo，清除该只读 surface 潜在的旧 undo/redo；旁边 Composer 的实例、草稿、选区、preedit 和 undo 不被访问。更新取消旧阅读 surface 的拖动手势，保留映射后已建立的选区；不 blur/focus 任一控件。DraftJournal 的投递/持久化版本不是阅读文档绑定，不把业务草稿协议搬进来。

## 公共消费方式

```rust,ignore
let expected = reading.set_document(&mut transcript, "caller-document", 1, &old)?;
// Retain expected with the page request. Apply its response on the UI thread.
let delta = prefix.encode_utf16().count();
let len = old.encode_utf16().count();
let update = reading.update_document(
    &mut transcript, &expected, 2, &format!("{prefix}{old}{tail}"),
    &[TextRangeMapping { old: 0..len, new: delta..delta + len }],
)?;
// Persist update.version for subsequent requests. Run the ordinary layout next.
```

独立 workspace 的 reading_prepend fixture 只使用公开 API：原 TextSurface 显示历史，旁边 ContentHost/Stack/ButtonContent/TextFieldContent 提供分页、尾追加、回到最新和独立草稿。它沿用当前 activations 计数出口，限制由 #21 跟踪，不另造动作协议。

Gallery 可运行预览：开启 composition-review 并设置 TEKESUI_READING_PREPEND_AUDIT_OUT；可选 TEKESUI_READING_PREPEND_BACKGROUND=1 以一次有限工作线程请求复用 #15 WindowWake。默认 Gallery 不变，无新登记路由，组件仍 45 个。

## 分层验证

- 公共 API：reading_contract 共 10 项真实 CoreText 测试，覆盖旧尾追加/跟随、固定宽度前插、连续两次前插（同一 layout 前）、前插+尾追加、宽度/字号重排、A→B→A/跨 controller/旧版本/无效映射、删除/截断/clamp、字素、选区身份/亲和性、组字拒绝。共享相关包合计 357 项通过。
- 真实消费者：独立 consumer 共 50 项通过。新增 3 项验证 direct TextSurface 阅读与 Composer 焦点/草稿/undo 隔离、原 Button 激活驱动分页，以及原 TextFieldContent.reading 通过其既有真实 slot 保留锚点、不抢 sibling editor 焦点；CoreText 真布局，glyph painter 为 no-op，不能替代 GPU。
- GPU：4 主题 × 2 宽度/字号代表组合（360/150%、800/100%）× 1x/2x × 初始/两次前插 = 48 张原 Renderer/真实字形截图。32 个前插几何检查误差均 0pt。保留逐图与 geometry.json；Light 窄大字、Dark IC 2x 已实看。没有将这些代表组合说成全状态穷尽。
- Token：共享 Rust 源码变化后重新执行全部 8 组探针，4500 条实际查找均绑定当前源哈希，complete=false。无 token/设计源变化。门禁正确拒绝了只包含 header 的 288 条导航证据；改用原 content-status 组合探针补全 408 条，不降级原验收范围。
- 原生 Gallery：先将真实草稿控件焦点置于 8604，再由后台一次 WindowWake 前插第一页，焦点/草稿保持；真实点击加载第二页，记录两次误差均 0pt，显示同一行。尾追加不自动跟随，显式回到最新才显示末尾新增内容。截图和原生事件 JSON 单列，不把控件内部焦点 ID 说成完整 VoiceOver 验收。
- 可访问性：此预览当前只导出窗口/菜单 AX，未验收新正文/按钮语义树；实际数值 AX、VoiceOver 仍归 #5/#12/#14。物理中文 IME 未测；组字拒绝是公共 API 验证，不是现场输入法结果。性能按用户要求暂不测。

Gold7：G1–G3 继续使用原 JSON 样式与字体，四主题 GPU/代表截图检视阅读与边界；G4 原共享 Button 物理保留，未新增动效，不能由截图判定完整动效验收；G5/G6 用前后位置、选区/焦点隔离、跟随/回到最新与失效回退验证；G7 公共 API、独立消费者、真实可运行 Gallery 和文档分别交付。物理 IME、全 AX/VoiceOver 与消费侧最终体验仍未测。

证据：design/reviews/prepend-reading-20261018/receipt.json。尚未完成：AgentKit 去除像素补偿/接入本接口后的 workspace/demo/第二 consumer 联调；正式不可变 pin（受用户禁止提交/推送/tag 约束）；物理 IME、VoiceOver，以及 #17–#20/#21 的剩余共享能力。全局整改仍进行中，下一批先统一 #19/#17 的公共事件透传与文本服务消费语义。
