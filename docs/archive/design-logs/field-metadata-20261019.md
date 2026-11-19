# Field 标签及必填标记的共享整改

根因属于公共组件布局契约。FieldView 先用整行宽度测量标签，再把剩余宽度给必填星号；
换行标签可能占满整行，使星号宽度变成0。另一个入口用 label_role 测量，却用正文 role 绘制星号，
导致强调标签和星号字体不同。两项公共 API 红例均已真实复现。

共享修复位于既有 FieldView::render_with_presentation：先为同字体的星号预留宽度，再测量标签，
按新的标签高度向下安排原输入槽和消息；星号与标签共用 label_role。复用原 TextRenderer、Rect、
FieldPresentation 和原有颜色角色，没有新增 token、状态、布局协议或抽象层。

适用范围为全部 FieldView 调用者、普通/强调表单，以及实际使用该 View 的 TextFieldContent。
独立消费者仅组合三个原 TextFieldContent，分别验证正常帮助、错误消息和整体禁用输入；
没有复制元数据视图、输入状态机或事件路由。整体禁用的交互/底色由原 TextSurface 负责，
Field 自身没有新增 disabled 属性。内容呈现 read_only_content 刻意没有表单元数据，不受本次影响。
必填星号附在首行右侧的预留空间；它不是新增图标组件，也不把 label 变成按钮。

公共 API：两项红例转绿；追加48个四主题/三品牌、正常/错误、普通/强调案例。
真实消费者：四主题×两宽度×三状态，检查实际 FieldView 槽位与 TextSurface.outer 一致、
内容不变、错误描述及禁用焦点隔离。测试最初缺少原 TextSurfaceHost，已改用既有 HostText；
该夹具失败保留日志，不计为产品红例。旧的“星号必须在整个标签矩形外”断言也已改成与实际文字宽度核对，
标签矩形内未使用空间允许紧邻短标签放置星号。

实际 CoreText/Metal GPU：前后各96图，覆盖四主题、@1x/@2x、140/344pt、100%/150%，
普通/强调元数据及上述真实消费者。旧源码丢失24个星号布局；修复后全部保留。
后续增加首行基线断言的96图也通过。已抽查窄宽大字号前后和深色IC真实表单。

当前状态：公共 API/真实接入/实际GPU及12个活动外接屏原生检查点通过，有界Gold7已单列留证。
Tab/Shift+Tab跳过禁用输入；AX更新后原生Backspace保留正确值和UTF-16光标，实际指针不让禁用输入取焦点或改值，
错误编辑器的独立焦点和错误边界在360pt窄窗保持可见。窗口已关闭，标准轮次accepted/cleaned，性能/VoiceOver/物理IME未测。
本轮源码以已验NumberField导出为底，包含全部本轮Field增量；并行Scene3D WIP保留但不扩入本次验收。
源码/证据及此前AgentKit冻结包不覆盖；最终源码身份、探针与原生收据见
`design/reviews/field-metadata-20261019/`。性能不测，不提交、推送或关闭Issue。

## 2026-10-20 状态职责与真实 token 补证

本轮未修改运行时绘制。FieldView 只负责标签、必填标记、帮助及错误信息，模型没有独立 disabled 字段；可用性由 TextSurface 子输入负责。清单将 metadata 的 disabled 明确为职责委托，真实表单的禁用验收仍保留，不能因委托而删掉。

复用既有 content_state/state_asserted 收据契约，将真实 Field API 的48个样例分别绑定为24正常、24错误。测试新增实际 draw ink 核对，验证标签／必填标记 primary-text、帮助 secondary-text、错误 status-failed，以及错误覆盖帮助的优先级；覆盖四主题、三品牌及两种标签字号角色。无新 token、组件或交互抽象。普通表单与 TextFieldContent 均适用；无装饰只读内容合理省略 metadata。

3项公共API测试、token-audit严格Clippy、真实组合消费者8组主题／宽度的正常／错误／禁用输入检查通过，42项能力治理回归通过。当前 context 精确反映 metadata 正常／错误已验证及禁用委托；仍保持 experimental 和完整 token 依赖未验边界。两项测试可执行文件已清理。

本轮相关8个运行时文件与原203d38ff逐字节一致。旧96帧GPU、12项外接屏原生及 scoped Gold7证据继续保留其原身份，不能称为本轮重新执行或当前整个WIP已验收。证据与源码哈希见 design/reviews/field-state-contract-20261020/acceptance.json。不测性能，不提交、推送或关闭Issue。
