# 原 #5：默认 Gallery 真实辅助技术接线

2026-10-18，基于全部 WIP。状态为部分完成；没有提交、推送、tag、关闭 Issue 或发布。性能未测，未新增注册组件或 UI 框架，JSON DS 继续是样式真源。

## 根因与共享修复

平台已有原生 AX 桥，公开 composition consumer 已能发布真实节点；默认 Gallery Host 却没有发布和动作转交。不是再造语义树或示例状态机的理由。本轮从原绘制参数、实际控件状态与已安排/裁剪后的布局投影节点，并通过原业务激活、焦点和 TextInput 入口处理辅助操作。

- `gallery/accessibility.rs` 与 `workbench/accessibility.rs` 只承担宿主投影/转交。按钮名称在原 action 绘制入口记录，同一个 title 同时用于绘制和节点；禁用/loading、selected、可见布局来自实际 Button。AX press 使用原 Button 的激活再进入原 StoryState.activate。Tab 与 AX focus 复用抽出的原 focus_control；查询修改复用原 input_changed。
- TextDemo 的 AX 入口复用原 TextInput 的合法选区、插入事务和 TextSurface.reveal；marked/disabled 拒绝，readonly 只允许选择。输入状态仍来自同一个 surface_state，不改变业务发送或草稿归属。
- 现有 DialogView 缺直接语义动作入口。补 `focus_button(index)`、`activate_button(index)` 和 `button_label(index)`，复用保留的 Button、原 finish/恢复调用者路径和绘制标签。不合成 Tab/Enter，不增加焦点系统。隐藏、失活、disabled、非法索引和已关闭动作均拒绝；不会二次关闭。
- AccessibilityRole 新增 TextArea，原选择/文字协议及所有原生文本 selector 同时覆盖单行/多行。默认 Gallery 按实际 multiline 标志投影；仓库中已有两个原生 projection 同样迁移。公开枚举的穷尽匹配消费者需增加分支；这不属于已移交的旧 e4c6cb6f 开发候选，AgentKit 冻结验收不被自动更新。
- Window.accessibility_nodes 返回验证后已发布树的 owned readback，关闭后返回空。native 元素/子节点查询同样拒绝 closed Shared，旧对象不能继续暴露关闭窗口的数据。没有额外事件总线或轮询。

## 真实范围和合理边界

当前覆盖默认 Gallery 外围导航/搜索、实际通用 Button 入口、TextField/TextArea/SearchField，以及现有 Dialog 及其两个动作。Readonly TextDemo 选择/拒绝编辑有自动回归。非交互状态标本不冒充真实可操作控件；没有为了补数量暴露缓存按钮或从命中框猜角色。

Gallery 外围导航和组件预览是两个既有宿主范围。最终实现只隔离 Dialog 所在预览的背景节点，保留外围导航；导航/文本焦点优先级沿用原 Gallery.handle_input/text_input_state。story 容器的 Group 身份持续存在，Dialog 用独立 part69，两个实际按钮为70/71。没有将既有 story 容器改成另一种角色，也没有把预览模态扩大为整窗模态。中间版本的整窗隔离记录保留为历史，不作为最终范围结论。

默认 Gallery 的 IconButton/选择控件/列表与 Menu 等 producer 仍待接线；此处目录中的对应按钮只是实际导航按钮，不是这些组件本体的验收。已有独立 composition consumer 的这些族证据仍分开保留。完整静态文本、富文本辅助协议、交互式语义树 Inspector、完整通知/VoiceOver 与物理输入矩阵没有冒充完成。当前内建 opt-in `TEKESUI_GALLERY_AX_OUT` 保存来自 Window 已接受树的摘要，可与系统 AX 查询对照，尚不是完整交互 Inspector。

## 分层验证

公共 API：独立 consumer 直接调用 DialogView 的可见/隐藏、非法索引、disabled、失活、Cancel/Confirm、恢复一次及关闭后拒绝路径；普通2项、token feature3项通过。新增24个实际 focus/activate/原画笔查找案例（4主题×3品牌×2动作），complete=false。其余8组既有探针4500案例重跑并绑定当前共享源码，不宣称依赖全集。

最终完整 workspace1015通过/1项既有图标目录测试忽略，全workspace严格Clippy及完整Python/设计门禁通过。共享395、独立消费者116、平台36通过；新增7项接线/模态/族回归与1项readonly回归包含在最终完整命令中。独立源码副本的controls/platform352、Dialog feature3及公开绑定3通过。相应严格Clippy、格式、能力/组合元数据及设计门禁以本批 receipt 和原始日志为准，不把旧全库结果冒充新完整命令。

默认 Gallery 原生：before 签名二进制的系统树只有窗口/菜单；最终 accepted 二进制导出实际节点。真实 AX press触发原事件日志、禁用后不再激活；TextField/SearchField 中文、emoji、组合字符值和UTF16选区3..5；TextArea 识别为系统 text entry area 并可编辑多行；Dialog 保留外围宿主，只隐藏预览背景，Cancel/Confirm 恢复 Open dialog；外围搜索获得焦点时没有第二个 focused owner；关闭后的公共树为空。

所有原生窗口均按外接屏规则启动，accepted readback确认 U2790B/builtin=false/scale2/max_fps30。未测帧率，未改变系统设置。before/after、Metal PNG、系统 AX 树、动作摘要、最终302个源码输入和签名二进制哈希分别保留。Gold7核对原共享边界、文字、选区和焦点反馈；没有页面局部样式补丁。Light代表场景及机械门禁不能替代完整主题/VoiceOver验收。

最终作用域与系统操作证据以 native/accepted 为准；最后补齐输入 invalid 状态和完整 parent/description/expanded/invalid 读回字段，补证位于 native/flags-final（系统随时间切到 Dark，未修改系统设置）。invalid 的公开读回不等于 VoiceOver 已朗读错误。此前 current/final/scope-final 属于逐步实现历史，不用于宣称最终模态范围；共享代码的命令链、IME和pointer原证据不重写。物理中文IME/VoiceOver未测，不把Unicode AX操作称为物理输入法。

## 接续

AgentKit 已独立确认冻结 e4c6cb6f 候选 UI27/headless76、实际 Earlier两页0pt和真实Native SEP到GPU通过。原生窗口授权仍待其会话确认；本会话没有操作AgentKit窗口或改其源码。

#5/#12/#14 保留未满足项：补完现有 producer、真实树 Inspector与完整通知/无障碍矩阵；不自动扩大组件范围。原 #4 其余浮层生命周期仍按原 Issue继续。证据位于 `design/reviews/gallery-accessibility-20261018/`，未满足条件不关闭 Issue。

中断恢复：首次完整命令因系统ENFILE终止，原进程确认结束、资源恢复后以--offline/-j2重跑；失败日志保留，不计作代码断言失败或通过。
