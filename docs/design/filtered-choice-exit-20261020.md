# 选择与命令消费者：过滤身份和退出绘制

基于全部当前 WIP，仅整改已有 Select、ComboBox、CommandPalette 消费者。共享实现负责人为 root，未提交、推送或关闭 Issue。

根因分为三处：公共过滤菜单把普通关闭当作硬取消；消费者用动画是否运行判断浮层是否仍需绘制，漏掉退出首帧；过滤后的语义动作使用可见行号，可能执行错误来源项。Select／ComboBox 还把失败提示放在候选浮层下方。

共享修复位于 `crates/tekes-ui-controls/src/layout_view.rs`，复用现有 `dismiss`、来源映射及 `OverlayPresence`。消费者沿用现有 `ContentHost` 和 Stack，将语义项与动作统一为原始来源索引，使用实际 presentation 状态，并将反馈放到输入／触发器之前；没有新增抽象层或 token。

适用范围是上述三种消费者及现有 FilteredMenuView 调用者。原始索引仅在过滤中保留身份，不表示任意数据替换后的永久业务 ID。数据替换、窗口失活和显式取消仍硬清理；退出期间资源或几何失效遵守既有规则。Select 改变标签宽度可能使退出几何失效；固定几何下的退出已单独验证。

验证结果：370 项 controls、157 项消费者测试及严格 Clippy；真实 GPU 144 组、720 帧；U2790B@2 外接屏真实 AppKit／AX 36 条断言；11 份实际 token 探针、5,040 个有界场景。旧消费者对照使用当前公共原语，不能当作旧整库二进制。Gold7 核对覆盖失败提示可读、过滤动作准确、查询与选择保留、关闭后事件与语义撤销、退出绘制和窗口恢复。VoiceOver 听读、物理 IME 与最终整页仍单独验收。

完整证据见 `design/reviews/filtered-choice-exit-20261020/acceptance.json`。已关闭宿主并验证无占用，清理约 324 MB 的本轮运行与测试产物；共享库缓存、源码及证据保留。性能未测，八批整改仍在进行。
