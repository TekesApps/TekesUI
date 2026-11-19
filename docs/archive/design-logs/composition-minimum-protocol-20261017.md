# #14 更新后的最小组合协议验证

> 后续第二批：异构 List 与 Rich Dialog 已补最小适配和独立消费者；最新分层证据及剩余条件见 [第二批记录](composition-list-dialog-20261017.md)。下文保留第一批时点的状态，不作为当前阻塞结论。

本轮先读取了更新后的 #14（2026-10-17 16:15:16，Asia/Shanghai；标题“统一组件模型与组合协议：控件内容组合、语义解耦与窗口级 Toolbar”）。正文快照见 `design/reviews/composition-20261017/issue14-updated.json`。

#14 保持审计/规划入口，不创建或拆分子 Issue，不把试验代码等同于审计完成。本聊天此前授权继续实际整改；以下为全部现有 WIP 之上的未提交最小协议试验。没有立即执行全库泛化迁移，也没有在 GitHub 回填“已完成”。旧 Container 矩阵不足以替代最新要求的全组件能力矩阵，该审计交付仍未完成。

## 根因、复用与新增边界

根因属于公共组件契约和组合能力：已有控件拥有测量、绘制和自身交互，但消费者必须手工接线不同组件的布局、事件、焦点与状态。已有 `MeasuredChild`、`layout_measured_children`、`ContainerLayout`、`ScrollGeometry`、`ScrollArea` 和文本接口可以复用，不能再把这部分描述为缺失。

`tekes-ui-controls::composition` 试验只补 owned child、稳定 key、遍历/路由与语义内容边界。组件只必须实现测量；叶子可只实现测量/绘制，交互、子节点、焦点、生命周期等有空默认行为，不要求所有组件都可点击或有 children。使用动态异构节点是当前消费者场景的一个实现选择，不宣称静态组合、借用式组件或全库统一表示已定案。

- 几何与主题：继续使用 Scene、Rect/Size、JSON 派生 Palette、原文本适配器及共享测量布局；不维护第二套布局引擎、颜色表或 token。
- 状态：Content 持有原对象，重排移动对象；主题/重布局不重建。卸载返回 owned subtree，由消费者决定保留或丢弃；没有自动序列化/重建恢复/虚拟化保证。编辑中的保留与生命周期以此边界为准。
- 事件：子项先消费；独立交互能力与键盘焦点分别声明，非焦点拖动也能 capture；Scroll 显式声明滚动交互，行内 Button 未消费的 wheel 可向其冒泡；失活/卸载/完全裁出捕获项会取消。逻辑焦点与可见命中区域分别处理；IME composing 时 Tab 让路。原生 text-input query/candidate/selection 桥和生产 TextField 适配尚未接入。
- 更新：事件 changed、焦点与结构修改置 `needs_render`，成功 render 确认；动画继续走原文本适配器的 host clock/frame request。
- Slot：Stack/Grid/Scroll 与 CompoundButton 都承载同一种 Content；Button 固定一个 content 槽，槽内可用任意嵌套布局。普通 Button 内禁止独立交互后代，给出稳定 ID 诊断；子内容不形成第二个键盘焦点或系统语义操作。
- 呈现：`Button::render_content_frame` 复用原 Button 绘制、按压、取消、动画和焦点实现；原字符串入口继续调用同一实现。没有新增 ButtonVariant。组合内容的继承前景通过 Palette 派生，Badge 等自带表面的消费者内容仍使用自身语义颜色。
- 无障碍：试验导出实际 owned 树的 id/parent/bounds/role/name/value/state；复合 Button 导出一个显式名称和一个动作节点，其内部为装饰内容。此投影没有 macOS 桥或完整 action 映射，不构成 #5 或 VoiceOver 通过。

合理例外：Stack/Grid 是布局组织者，遵循现有布局原语，不额外成为 viewport；真实 ancestor/Scroll clip 约束绘制与命中，焦点环不被每个控件自身框裁掉。一个可选 flexible child 可在固定 Icon/Badge 之间按剩余宽度重新测量文字；这是现有纯 placement helper 无法表达的约束分配，没有增加视觉 token。Scroll 继续复用原来的滚动和弹性状态机。

## 更新后的实施门禁

| 最新 #14 最小场景 | 当前状态 | 边界 |
| --- | --- | --- |
| 独立消费者复合 Button：Icon、嵌套标题/说明、Badge | 已实现待验收 | 独立 workspace 1 项公共 API 检查、核心10项定向测试、真实 IconView/Button/文本/GPU 的32帧通过；物理输入、原生辅助技术与完整体验待验收 |
| 独立消费者异构 List：Text/Badge/Switch/TextField/Menu/自定义内容 | 被依赖阻塞 | 仍缺生产输入/菜单适配、集合内容槽及语义路由；不能把泛型 Stack 或测试 Editor 当成完整 List |
| 独立消费者 Rich Dialog：Grid/输入/选择/验证/Footer | 被依赖阻塞 | 仍缺 Dialog 内容槽与原生文本焦点/模态子树接入；现有固定 Dialog 不算本场景通过 |

接下来先补输入/选择/菜单的公共组合适配与节点查询边界，再复用现有 List、Dialog 模型验证上述两个消费者场景。三个场景确认后才能按组件族迁移 Tabs/Tree/DataTable/Disclosure 等内容槽；窗口级 Toolbar 继续依赖组合和语义基础。本轮没有改动 Gallery 生产路由或把内容工具栏称为窗口 Toolbar。

## 三层验证与 Gold7

1. 公共 API：`crates/tekes-ui-controls/tests/composition.rs` 覆盖混合内容、嵌套、重复 key、动态增删/重排、编辑/IME 模型保留、焦点、取消、子事件消费、单一动作与非法槽位。`examples/composition-consumer` 为单独 Cargo workspace，不引用 Gallery 类型。
2. 真实接入：Gallery 测试只负责给同一消费者 fixture 提供真实文字 backend、上传批准 Icon 的像素及 GPU；组件使用生产 IconView 和 Button。Gallery 测试不替代独立消费者，也没有完成生产路由/原生交互接入。
3. Gold7 体验：抽查普通 Light 窄宽/150%/@2x 与 Dark 画面，检查文字包行、Badge 边界、图标、共享前景和独立焦点环。32帧和几何断言不是完整体验验收；G5 实际IME、G6 原生操作及 G7 三场景/全部组件接入仍未完成。不改变既有 experience 未验证标记或组件 experimental 成熟度。

当前公共字符串 Button API 保持兼容，新增入口与动态 owned 表示均为 experimental；尚无全组件 Breaking Changes 审计结论，其他组件族不据此强制迁移。

性能按用户范围未测。验证日志、源身份及剩余项见 `design/reviews/composition-20261017/receipt.json`。保留原 WIP；不提交、推送、打 tag、关闭 Issue 或发布。
