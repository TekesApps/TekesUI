# #18 受控文本更新与真实消费者（2026-10-18）

本批基于全部 WIP。根因是公共编辑器没有可验证的外部更新事务，消费者直接替换 TextInput 会丢失选区、组字和撤销，并可能接受迟到结果。复用 DraftRevision、TextInput、TextSurface 和 ContentHost，不新增业务状态层或组件。

## 公共契约

`TextInput::external_baseline()` 返回绑定当前实例的不可变预期基线。请求开始时捕获，结果到达时使用原基线，不能用到达时的新基线伪装旧请求。调用 `TextSurface::apply_external_value` 同步原有布局、滚动与交互缓存；仅直接使用 TextInput 的消费者可调用其同名方法。

`DraftRevision` 的 identity/version/payload 由调用方的权威数据源提供。identity 非空，version 非零；同文档旧版本或同版本不同 payload 拒绝。同 key/version/原始 payload 的重复确认返回 Unchanged，即使已有新本地输入或 preedit，也不覆盖它们。原始 payload 身份比较与规范化后的显示值分别保留；CRLF、控制字符和单行换行按现有 insert 规则规范化。

所有检查在修改前完成。普通更新在 marked/composition 期间返回 Composition，不自动提交、取消、排队或重试。调用方决定是否等待，并必须在重试前核对新的权威意图。旧基线、不同编辑器实例、编辑后撤销回原值、非法选区分别有明确冲突。原有公开字段的当前值改变也会检出；绕过方法直接修改字段又恢复相同值的 ABA 不属于受控协议。read-only/disabled 是用户交互状态，不阻止经过验证的权威值更新。

选区策略是 Preserve、Set 或 Reset。Preserve 按完整 grapheme 的最长共同前后缀映射两个有方向端点，替换内部收敛到新末端，插入边界向右偏移；不是任意多段编辑映射协议。Set 使用规范化文本的 UTF-16 范围并对齐 grapheme，非法范围原子拒绝。历史策略是 Preserve（保留双栈）、Transaction（一次可撤销修改）或 Reset（清空双栈）。切换文档必须 Reset；发送后的清空使用 Transaction，因此撤销可恢复草稿，这是明确策略而非重复回写。

ContentHost 原有按键所有权同时记录编辑器实例与受控绑定代次。外部更新或更换编辑器后，旧按键的剩余回调/提交回退失效；已实际交付的本地编辑不会回滚。不增加第二套焦点或事件路由。

适用于共享 TextInput/TextSurface 的输入族和消费方包装器；本批真实回归是独立 TextFieldContent 以及 AgentKit AgentText。其他组件并未自动迁移，未以单个页面改善宣称整个输入系统完成。

## AgentKit 接入

PresentationStore 在发布 RunSnapshot 时生成单调 revision，正常刷新和降级路径一致；UI 不使用结果到达顺序伪造源版本。UI 属于一个 controller/store；新 controller 使用新的 surface。反序列化的旧 snapshot revision=0 不作为受控权威数据接入。

RunSurface 拒绝迟到的源快照；保留各文档原编辑器实例。重复源值不覆盖未确认的本地输入或 preedit；正常确认更新受控版本，发送按钮捕获原编辑基线，复用既有同步 prepare 后立即应用权威清空。冲突保留当前草稿并阻止用不一致模型发送，不自动重试。仍是显式按钮发送，键盘业务发送及完整多行视口属于后续 #20/#21。

## 分层验证与边界

公共 API：13 项受控事务测试，另 3 项独立 fixture 测试，覆盖重复/迟到/ABA、UTF-16、组字拒绝、撤销与派生状态原子性。真实消费者：AgentKit 新增 5 项，总 UI 20 项；headless 76 项；严格 Clippy。

Gallery 使用同一个独立 fixture 经 AppKit、公共 ContentHost、原 TextSurface 和真实 Metal 绘制。Light 场景：全选原文 0..15，外部前插后 1..16 仍选原文；清空一次撤销恢复文本与选区；捕获清空后编辑为“前x”，旧清空返回 Revision；切 B 后旧 A 结果返回 StaleBaseline，B 无 A 撤销，切回 A 保留最新值。分步 JSON、截图、签名后二进制和源码哈希分别保存。

Gold7 仅核对此 Light 最小场景的选区、焦点、文本可见性和状态一致性；不是多主题、整个组件族或无障碍最终体验验收。逻辑 semantics 单独保留，未声称 VoiceOver/原生 AX 完成。物理输入法、AgentKit 原生窗口和性能没有验收；性能按要求不测。完整门禁与独立构建结果以本批最终 test-summary 为准，当前未封存前不标记整体完成。未提交、暂存、推送、关闭 Issue 或发布。
