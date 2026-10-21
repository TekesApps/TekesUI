# Issue #5 — IconButton 与选择控件原生语义接入

基线为 `gallery-design-baseline` 的全部 WIP，HEAD bc3de79；2026-10-19 刷新远端仍为 21 个开放 Issue、0 个开放 PR。此次是原 #5 的后续，不新增 Issue，不升级组件成熟度。

根因分为公共语义表达和真实消费者适配两部分：已有控件模型与保留视图能够激活、选择、禁用及聚焦，默认 Gallery 之前只遍历通用 Button 存储，遗漏 ExtendedStory 的真实控件；Checkbox 还未登记实际绘制标题。原 Switch 的二态原生映射不能准确表达三态 Checkbox 或互斥 Radio。

共享修复位置：`tekes-ui-platform-macos/src/accessibility.rs` 增加 Checkbox、RadioButton、RadioGroup 角色，接入现有 role/value/Press/Focus 路由。Checkbox 复用节点 `selected` 和 `value`；`value == "mixed"` 映射 AppKit 数值 -1，普通状态仍为 0/1。新增 enum 变体需要外部 exhaustive match 迁移，不声称已发布兼容版本。AgentKit e4 冻结快照保持不变。

真实接入位置：Workbench 公共可访问性投影复用 mounted/eligible 焦点目标、真实可见布局和控件模型。IconButton 名称直接来自 required accessible_name；Checkbox、Radio、Switch、Segmented 文案在实际绘制处记录，避免另写一套文案。互斥组保留稳定 part29 容器、part30–32 子节点及父子关系。Press 复用原模型/视图动作和事件日志；互斥选择沿用原 roving 焦点，已选中项目仍能获得焦点而不伪造选择变更日志。通用 Button/Checkbox 的 Focus 与 Press 保持原有独立语义，Press 不强制抢焦点；IconButton 原 activation 入口改为调用现有 IconButtonView::activate。未新增状态机、样式覆盖或 token。

适用范围是当前真实 Gallery 的 IconButton、Checkbox、RadioGroup、Switch、SegmentedControl。Segmented 是互斥选项，导出 RadioGroup/RadioButton；Tab 页面导航不因此自动归为相同角色。Slider 的 range 与 Select/Combo 弹层、Tabs、列表和菜单需要各自已有契约接入，不能由本轮五项替代。状态矩阵中的静态展示样本未伪装为可操作控件。

验证分层：

- 公共 API：controls/platform 352 项通过；新增角色通过实际 AppKit 编译和原生角色/value 读回验证。
- 独立公共消费者：composition-consumer 116 项通过；这是现有消费者兼容回归，不声称该消费者新增了全部五种语义导出。
- 真实 Gallery：10 项可访问性测试通过，其中新增 3 项覆盖真实动作、禁用拒绝、互斥选中/焦点/父子关系、mixed 转换。原 Dialog/文本/隐藏路由/preedit 隔离保持通过。
- 原生与 GPU：5 页均在 U2790B、builtin=false、scale2 验证；max_fps30 是设备属性，不是性能测试。原生 AX Press → 原事件日志和实际绘制已留证。Radio/Segmented 值只在目标为1，Switch 0→1且实际标题同步，Checkbox -1→1，IconButton 真实名称及禁用状态可读。
- 前后对照：使用上轮已留源/二进制身份的修复前 Gallery，同一 Radio 页可见控件但缺少语义子节点；本轮导出真实角色和值。该旧二进制仅作对照，不冒充当前源码。
- Gold7：逐页检查本轮 Dark 外接屏画面中的名称、选中与焦点反馈，复用现有视觉语义；没有改动 token 或绘制规则。本证据不替代完整主题/品牌矩阵和 VoiceOver。

严格 workspace Clippy、完整 Python/设计门禁通过。全 workspace 1018 项通过、1 项既有 ignored；独立重放 controls/platform 352 项通过，321 文件精确字节重放。所有原生身份文件保留执行时源码摘要；随后能力清单更新仅更新源码绑定信息，未重写既有原生证据。

#5 仍为部分完成。下一步：列表与菜单真实生产者、交互式语义树 Inspector、完整通知矩阵；VoiceOver 朗读/导航与物理 IME 仍待独立条件和实测。无提交、推送、tag、关闭 Issue、发布或系统设置改动。
