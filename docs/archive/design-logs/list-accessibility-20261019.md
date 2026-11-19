# Issue #5 — 真实 List 语义与集合激活边界

以当前活跃 `gallery-design-baseline` 的全部 WIP 为基线。2026-10-19 刷新远端仍有 21 个开放 Issue、0 个开放 PR，各 Issue 更新时间与上批一致。主 checkout 的 main/a3bdf04 是 10 月 15 日的既有文档提交；未切换、覆盖或合入，整改继续基于当前 bc3de79 + ALL WIP。

根因与共享范围：

- 基础交互边界：ListView 只检查焦点索引是否属于模型，TreeView 只检查当前模型可见性，未确认该行是否进入上一轮实际布局。独立消费者分别复现了未绘制 List 行被 Activated(4)、刚展开但未布局的 Tree 子节点被 Activated(1)。
- 公共实现修复：`collections_view.rs` 的 ListView/TreeView key 守卫改为确认当前 layout.items 含目标 ID。复用既有布局与 CollectionEvent；保留已布局但被祖先裁剪的键盘目标。DataTable 的 Enter 已检查实际 layout.rows，保持原实现；Chart 的图表导航是另一契约，不套行协议。
- 宿主动作适配：集合适配已有 InputResult { consumed, changed }，但 ExtendedStory 的 activate 只取 changed，导致已选中行的合法 Activated 被当作失败。改为复用 consumed；List、Tree、DataTable 均以真实布局和原视图验证了重复激活。未新增事件总线或动作状态机。
- 真实消费者接入：默认 Gallery 之前只有导航侧栏 List，未导出正文的 ListView。现在在原绘制处登记实际行标题，沿用上一批的分组/可见几何投影，导出 part29 List 与 part30–34 ListItem；父子关系、selected/focused、enabled 和 bounds 均取原对象。AX Focus/Press 复用原 focus_control、ListView 和集合适配。导航 List 不能作为正文接入证据。

适用组件是公共 ListView/TreeView 的布局成员边界，以及现有集合激活适配的消费结果；本轮真实原生生产者是 List。Tree、DataTable 的原生 role/层级/排序等尚未接入，不因共享测试通过而升格。List Playground 沿用既有配置，不新增 disabled 开关；禁用边界通过公共 API 与真实 Gallery 状态测试验证。静态状态矩阵样本不伪装为可操作节点。

验证证据分层保存在 `design/reviews/list-accessibility-20261019`：公共消费者 collection_mount 的编译后 red/green；原视图/共享激活回归；真实 Gallery List 的标签、互斥选中、Focus 与 Press 分离、重复激活日志、裁剪/隐藏和禁用拒绝；原生 AX 及实际 GPU 前后画面。最终命令结果和源码身份见 receipt，不将编译或 JSON 导出替代实际体验。

共享核心源码变化后，8 个 token 探针已全部重新执行，4500 个有界案例；complete=false。未新增 token、样式或组件。能力清单和组合审计绑定更新为当前源，保留 native_acceptance=未验证 的静态审计边界。

当前 #5 仍为部分完成。后续是剩余选择/集合/菜单真实生产者、可交互语义树 Inspector、完整通知与 VoiceOver/物理 IME 验收。AgentKit e4 冻结快照保持不变。无提交、推送、tag、关闭 Issue、发布、性能测试或系统设置改动。

本轮已完成验证：全 workspace 1021 项通过、1 项既有 ignored；共享 controls/platform 352 项、独立消费者 118 项通过；Gallery 可访问性 13 项通过，其中本轮新增 3 项（包含 List/Tree/DataTable 的真实公共激活消费结果）。严格 Clippy、fmt、完整 Python/设计门禁通过。

原生验收为 U2790B、builtin=false、scale2、Light。修复前系统只有导航 List，正文五行可见但没有 row 节点；修复后正文 List/五个 row 的角色与父子关系由系统读回。AX 连续两次激活 Images 均进入原事件日志，原生 Down 选中 Archive，Docs 切换移除正文节点，返回保留原选择，窗口关闭后 accepted_nodes_after_close=0。所有本轮 Gallery 已退出。前后 PNG、完整系统树、动作日志与每次执行的源码/二进制摘要分别留存。

Gold7 核对限于上述 Light 原生画面的名称、选中、焦点、原事件日志和前后规则一致性；未改绘制或 token。四主题/多品牌的既有 GPU/契约门禁通过不替代所有原生主题体验，VoiceOver 朗读仍不声称完成。
