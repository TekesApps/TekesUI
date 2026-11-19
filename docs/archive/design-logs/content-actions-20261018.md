# #21 带目标与来源的公共动作（2026-10-18）

根因属于公共组合契约。原控件能判断激活，原 ContentHost 知道具体目标与输入来源，但 ContentResponse 只有聚合 bool，消费方通过 activations 计数发现动作，丢失顺序并可能在切换会话后重新解释目的地。

复用原 Component/ContentResponse/ContentHost：保留既有 Copy response 和计数兼容行为，增加三个纯数据类型、一个默认 Component 声明方法和原宿主内的收集/drain。没有事件总线、回调注册器、业务路由或新控件。JSON 颜色、字体、状态、边界和组件登记不变。

## 公共接入与生命周期

```rust
host.set_action_events(true); // 在输入之前显式启用；旧宿主默认不积累队列。
let response = host.event(&event);
for action in host.drain_actions() {
    // action.target: ComponentId
    // action.kind: Activate / Submit
    // action.source: Pointer / Keyboard / Accessibility / Programmatic
    // 此处捕获消费方当前展示的业务上下文，再异步处理。
}
```

AX 使用同一个 host.accessibility_action 后 drain，无需伪造鼠标移动。set_action_events(true) 重复调用不丢弃待处理记录；false 明确丢弃队列。重新启用不会扫描历史计数。drain 返回独立 Vec，空 drain 不重复通知；drop 宿主自然释放值，不调用业务回调。

默认 Component::action_kinds 在本次 response.activated 时产生一个 Activate。编辑器 TextFieldContent 显式声明 Submit，表示配置好的编辑器提交意图，仍不是业务发送；透明编辑器包装器转发该声明。动作类型不从 AX TextInput 角色推断，Combo 的选项确认仍为 Activate。此共享修正的红绿回归见后续 editor-navigation 批次，历史冻结证据保持不变。consumed/changed 不自动生成动作。其他组件可覆盖纯声明方法返回多个 kind，目标始终是实际收到输入的组件，source 由原路由绑定，不能伪造其他控件来源。

顺序是原输入实际投递顺序，再按每个组件声明的 kind 顺序。指针命中沿用反向子树命中与既有消费规则；悬停广播沿用先父后子的原顺序。一次按键链只有实际回退分发时产生动作，Started/Command/Key/Finished 不能反复发布同一次激活。一般控件通过 consumed 阻止再次处理；自定义组件显式声明两个动作就得到两个，不按 kind 盲目去重。

每个 owned Content 的私有挂载标识防止未 drain 的动作交给同 ID 新实例；remove/reinsert 即使保留原对象也更换标识，直接替换为新 Content 也不会继承记录。已经成功发生的动作不会因随后隐藏或改变焦点被重定向；隐藏不是卸载。drain 后得到的数据不引用宿主，可在处理时 redraw、focus、remove/insert 或再 dispatch。不得在 Component::event/action_kinds 内重入同一个可变宿主；声明方法是纯同步描述，业务处理在 drain 后进行。

适用范围是所有正确使用原 Component 投递边界的控件及消费方包装器。ButtonContent、ActionContent 和 TextFieldContent 默认受益；真实样例覆盖公共 Button、编辑器、嵌套树、独立消费者和 AgentKit。仅 changed 的选区/数值变化不自动升级为激活；没有自动迁移整库或发明新的选择/关闭/业务动作。取消与卸载清理不会发布用户激活动作。Source 表示路由入口，不单独证明物理输入或 VoiceOver。

## AgentKit 的真实接入

RunSurface 启用公共动作，移除所有 activations 轮询。event 和新增 accessibility_action 立即 drain 并捕获当前展示的 destination/scope。原生 demo AX 宿主直接接入，删除伪造 PointerMove 的旧适配。drain_intents 返回值可在选中 B 后仍安全处理 A 的发送、取消、分页。

低层 tree 调用者也需 drain；兼容路径在 sync 更换画面之前将尚未处理的公共动作和编辑变化冻结为消费方 owned Intent，防止用 B 的选择解释 A 的回调。该小型待交付值列表属于现有 AgentKit 消费状态，上游不认识 Session。发送清空仍使用先前公共受控事务的原基线。Reconnect 是既有全局连接操作，不伪装成会话动作。

AgentKit 产品仍是显式按钮 Send；公共编辑器 Submit 不自动升级为业务发送。AX 与原指针组字边界一致，preedit 期间对其他控件的操作不会先抢焦点并意外 unmark。发送/取消/分页的实际权限由原 presentation 和控件 disabled 状态决定。

## 分层证据与边界

公共确定性回归覆盖 pointer/Space/Enter/AX 一次激活、disabled、明确多动作顺序、空 drain、同 ID 新建与保留重挂载、隐藏后首次交付、焦点/绘制不重放、按键链一次派发、提交种类与组字屏障。AgentKit 新增三项真实 fixture 回归，包含运行中的取消/分页，在 A→B 后仍按原先 Earlier→Cancel 顺序携带 A scope；既有受控清空/IME 与完整 Run 回归继续运行。

外接 U2790B、scale2、实际 AppKit/Metal：坐标点击产生 Pointer×1，Space/Enter 产生 Keyboard×2，真实原生 AX 按钮入口产生 Accessibility×1；disabled B 不增加动作；原生粘贴不激活，CmdEnter 仅产生编辑器 Submit×1；焦点往返后总计仍是5。保存原生 AX 树、逐步 JSON、PNG、screen 信息、签名后二进制与源哈希。AX Press 入口可用不等于 VoiceOver 已验收。

Gold7 本批只核对此 Light 场景的控件可发现性、disabled/focus、输入和反馈一致性，原共享样式不调整。自动测试、真实消费、GPU/原生和可访问性入口分别留证；物理 IME、VoiceOver、AgentKit 原生窗口和完整多主题体验仍未验收。性能按要求不测。统一门禁和独立重建以最终 test-summary 为准，未完成全 #15–#21 之前不提前移交 AgentKit 会话。公开不可变 pin 需要另外的 Git 授权，本批不提交、推送、打 tag、关闭 Issue 或发布。
