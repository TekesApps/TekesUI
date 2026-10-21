# #20 公共文本视口与真实多行消费（2026-10-18）

基于全部 WIP。根因属于公共组件契约：TextSurface 已有 ScrollArea、组字、选区、caret 和阅读重排，Stack 已有 flexible_child；TextFieldContent 仍只提供表单 slot。现在补足配置和现有 FieldPresentation 的精确 slot 高度，不增加布局框架、滚动状态、组件或 token。

## 接入与尺寸语义

```rust
transcript.surface = TextSurface::new("", true, true);
transcript.viewport = TextViewport::Fill;
transcript.reading = Some(ReadingController::default());
// 原 StackContent 指定 flexible_child: Some(transcript_id)。

composer.surface = TextSurface::new("", true, false);
composer.viewport = TextViewport::Grow { min: 56., max: 160. };
composer.tab_policy = TextTabPolicy::Insert;
// 是否提交仍使用既有 submit_policy；尺寸策略不产生业务发送。
```

TextViewport::Form 保留默认单行表单尺寸和行为。Fixed(height) 指定输入 viewport 高度；Grow 按当前宽度下真实 shaping 的内容高度增长，受 min/max 限制；两者保留原最小完整文本行/token 高度。NaN/无穷/负数归零，max 小于有效 min 时使用 min。Fill 使用分配高度减去原 Field 的 label、gap 和 help/error 占用，可以在空间不足时收敛到零；文案自身不能凭空缩小，宿主仍须提供合理最小窗口高度。

所有高度均是输入 viewport，不包含 label/help/error。元数据复用 FieldView 的真实字体测量，不复制其计算规则。FieldPresentation.slot_height 是可选精确高度，None 维持原行为；旧显式结构体字面量需补 None 或使用 Default。没有改变 JSON 字体、间距、边界、选区或滚动条角色。

多行/只读继续由现有 TextInput 配置。TextSurface.wrap 控制多行软换行，单行仍不换行；滚动轴直接配置原 surface.scroll.set_scroll_policy。关闭 wrap 需要水平滚动时选择原 Horizontal Auto/Always 策略，不增加第二套内部滚动配置。

测量使用同一 role、padding 和像素对齐 wrap width，但不修改编辑值、历史、组字、已排布 paragraph、scroll 或 reading anchor。arrange 唯一确定实际 slot；paint 使用其既有高度，命中、caret、原生候选 rect 数据和逻辑 AX frame 均来自同一 TextSurface。切换 wrap 或 backing scale 导致实际换行宽度改变，也复用原 reading reflow anchor。

适用范围：现有 TextFieldContent 及正确委托它的消费者，包括本批独立 Transcript/Composer 和 AgentKit AgentText。旧单行表单继续 Form。其他输入族和组件没有自动迁移；未以本样例宣称全库体验完成。

## 真实消费与合理例外

独立 ViewportFixture 仅组合原 TextFieldContent、Stack 和 Button，无自定义 Component::arrange。Transcript 是 Fill+原 ReadingController，Composer 是 bounded Grow。原生 Gallery 使用同一个 fixture 和现有 key/pointer 适配，复制粘贴桥接复用 AgentKit 现有宿主方式。仅在 opt-in 验收入口运行。

AgentKit AgentText 移除固定 78pt composer 和专用 transcript arrange/paint，统一委托公共 Field。Composer 采用原文本行高与 inset 的十行上限产品策略；通用库不写入 Agent 业务尺寸。仍保留显式按钮 Send，不因多行或 Enter 新增业务提交；原受控草稿事务、文档实例和真实消息 identity 继续由 AgentKit 持有。

## 分层证据

公共回归覆盖 fill/grow/scroll、Form 与原 FieldView 尺寸一致、纯测量不改变 surface、resize 保留 preedit/selection/undo、只读选择与拒绝编辑、wrap/原滚动轴，以及非零原点、祖先 clip、1×/2×、100/150/200% 文字缩放的 caret/hit/逻辑 semantic frame。AgentKit 两项新增真实消费回归与原阅读、草稿、发送测试分开运行。

外接 U2790B，真实 AppKit→原公共 Field/Stack→Metal：3行混排84pt、20行封顶160pt；窗口由760×640缩为560×520后 caret 和发送仍可见；长行软换行且内部滚动；原生撤销恢复文本和选区。Transcript 已读 y=552，Composer 从160收缩到84后仍为552，焦点保持9202。只读全选后删除/粘贴不改变原文。保存逐步 JSON、PNG、真实 screen/placement/backing-scale 和签名后二进制/源码哈希。

初版键盘 typeText 未产生中文事件，故其证据仅支持实际收到的英文/Return；最终版用原生粘贴验证中英、emoji 和组合字符，不将自动化输入当物理中文输入法。Gold7 仅核对此 Light 原生组合场景的文本可见性、焦点、滚动与约束一致性。逻辑 semantics 不代表原生 AX/VoiceOver；物理 IME、VoiceOver、AgentKit 原生窗口和完整多主题体验未验收，性能按要求不测。

统一回归与可复现补丁集以本批最终 test-summary 为准。公开不可变 pin 需另行提交/推送授权；本批不暂存、不提交、不推送、不打 tag、不关闭 Issue 或发布。完整 #15–#21 就绪前不提前移交 AgentKit 会话。
