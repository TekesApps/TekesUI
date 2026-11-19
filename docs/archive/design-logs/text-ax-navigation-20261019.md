# #20 VoiceOver 文本进入与行导航跟进

以 gallery-design-baseline 全部最新 WIP 为基线，上一轮 `text-ax-lines-20261019` 和封存的 `agentkit-final-candidate-20261019` 保持不变。原 AgentKit 冻结快照 e4c6cb6（188文件）逐项核对无差异。本轮仅补共享文本 AX 能力和独立消费者验证，不修改 Agent 业务状态、JSON token 或 Composer 的 Tab 策略。

根因分类为公共文本语义/平台桥接缺口。上一轮真实排版行范围已存在，但缺少字素索引、位置命中、可见行、选区数组和带属性文本查询。本轮复用已有 `AccessibilityTextLayout` 与 CoreText composed-character caret boundaries，不新增编辑器、坐标体系、滚动状态或 UI 框架。

共享修复位于 `crates/tekes-ui-platform-macos/src/accessibility.rs`：`range_for_index` 返回实际排版字素范围（包括落在代理对内部的UTF-16索引）；`range_for_position` 以真实视觉caret区间命中；`visible_range` 返回垂直可见的完整排版行，保留横向被裁剪字符。原生桥接补齐对应的 AppKit 查询、AttributedString 和 SelectedTextRanges；单选区继续路由已有 SetSelection，拒绝多选区和错误 NSValue 类型。保持只读可选择、禁用不写入、旧元素失效及窗口关闭边界。

适用范围是所有发布完整布局的 TextField/TextArea 及其 Search、Composer、只读阅读组合。独立公开 consumer 和真实 AgentKit 消费候选均使用上一轮已有的按owner布局发布，无需新业务适配。旧Gallery未发布完整布局的入口仍需后续迁移，不能将两个消费者样例当作全系统验收完成。

诊断层次：旧式 NSObject AX 属性枚举与现代协议/远程AX读取表现不同，已明确不将其当作 VoiceOver 根因或通过证据。`selftest::Action::AuditAXText` 改为检查真实 AppKit 元素的现代协议值、selector可调用性、选区数组、可见行和字素几何。显式 `TEKESUI_TRACE_AX` 仅输出有界查询/动作元数据，用于关联实际 VoiceOver 请求；不启用轮询或额外动画。

自动验证：平台43项、独立消费者130项、AgentKit UI/第二消费者27项、headless76项全部通过；UI与平台测试严格Clippy、格式、Rust所有权边界、构建、GPU、设计与组合审计分别留证。新增回归覆盖代理对内部索引、RTL几何、软换行、水平裁剪、空文档、无几何拒绝，以及真实CoreText Transcript/Composer接入。

真实AppKit断言已在U2790B、scale2、Light通过：未聚焦只读Transcript和Composer各自返回正确角色、选区、可见范围、首行、字素及屏幕rect。这些断言不等于VoiceOver朗读通过。人声反馈为“仍无法进入历史文本”；日志显示该次窗口未活动，且无焦点/选区请求到达。随后用户明确表示当前不方便复验并要求关闭VoiceOver，因此活动窗口条件下的复验未执行，不把它标为解决。VoiceOver已通过Activity Monitor正常Quit并确认进程退出，临时筛选清空。最终源码仅执行不启用VoiceOver的AppKit断言。

Gold7：沿用现有JSON DS；本轮GPU PNG SHA `819c3c0d9415b7c17eacec9b47825dfd19c897e817c241e6b4a396d48ac05fc5` 与上一轮完全一致。G5/G6依实际文本导航与阅读连续性验收；G7有公共契约、独立consumer、真实宿主和可复现源码，不用静态检查代替体验。

Composer显式 `TextTabPolicy::Insert` 消费Backtab，是既有策略；不以修改它来让Shift+Tab测试通过。原Issue、公开不可变pin与消费者提升候选的交付边界保持，未提交、推送、发布或关闭Issue。性能未测。

窗口归属已单独排除：继承的Window/TopLevel关系对两个文本owner均匹配实际宿主，无需重写。最终源码公开快照 `4a02d30bd65c1117f433a874a7c421492bf76ad101e92f7260ecd470323554a1`，原生二进制 `59dd6634dc553ebf9e0ca950d7904238262e20b4d9e1605f4982f0004ee80b60`；10项复现检查及最终小宿主AppKit断言对应上述版本。状态为部分完成，缺实际VoiceOver独立进入/逐行的通过证据。

所有小宿主轮次已关闭，经fresh lsof检查清理runtime；最终测试可执行文件清理，公共库缓存保留。窗口关闭原wake及clone均Closed。VoiceOver已关闭且未在后续AppKit自动检查中重新开启。分层汇总与WIP相对补丁见证据目录，`delivery-source.tar.gz`保留316文件消费者源码，未提升主AgentKit快照。

协议参考：[Apple NSAccessibility](https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol)、[VisibleCharacterRange完整行规则](https://developer.apple.com/documentation/appkit/nsaccessibilitystatictext/accessibilityvisiblecharacterrange%28%29)。
