# Issue #5 / #14 — Menu 与 Select 公共语义及真实接入

基线是当前 HEAD80d6b05 加全部 WIP。远端仍有21个开放 Issue、0个开放 PR。原 List、图标并行变更兼容复核证据保持封存；本轮不改图标文件、不发布、不扩大45组件范围。

根因分为公共组件契约和宿主适配。MenuView 已提供内部模型的语义 focus_item/activate_item，但外部 Menu 模型没有对应方法。默认 Gallery 的正文 Menu 只有通用 Button 语义，Select 的真实触发器不在通用 Button 存储中，两者的弹出项都未导出。

共享位置：MenuView 的内部/外部语义方法现在复用 MenuRuntime 的同一组当前布局、模型 count、open、disabled、window-active 守卫和原激活/关闭路径。新增 focus_item_with_model 与 activate_item_with_model；SelectView 的 focus_option/activate_option 通过这一能力操作其原外部 Menu，并复用原 choose 和触发器恢复。Focus 不提交选择。既有 OverlayPresence 保留退出绘制，同时关闭后的语义操作被拒绝，Reduced Motion 直接结束退出。没有新增状态机、token、样式层或 UI 框架。

公共平台描述补上真实 Menu 角色并映射 NSAccessibilityMenuRole。AccessibilityRole 增加变体意味着外部 exhaustive match 需要迁移；未声称已发布兼容版本。默认 Gallery 复用实际绘制标题、popup/item 的可见布局及原模型；有可见调用者时形成 MenuButton → Menu → MenuItem，隐藏调用者时由现有页面容器承接 popup，避免悬空父节点。Menu part49/items50–52，Select part59/items60–62。动作菜单的活动项用 focused，Select 的提交值与活动焦点分别导出；关闭动画不导出仍可操作的旧项。

真实验证发现并修正两个宿主缺陷，均保留编译后的 red/green：

- Gallery 的 Select 旧激活捷径调用 Menu::show，重新展开时总把活动项重置为0。统一改用原 SelectView 激活路径，从当前提交值展开；不额外同步一份选择状态。原生旧构建也留有“Comfortable 已选中、Automatic 却获得焦点”的失败快照。
- 外围辅助焦点转移没有完整退出正文弹层。Workbench.clear_focus 复用 MenuView.deactivate，搜索与侧栏统一走共享焦点转移；原弹层退出时不把焦点抢回旧调用者。正文非弹层目标同样复用原 Escape 路径；展开触发器再次 Press 仅关闭一次。

适用范围是 MenuView 所有内部/外部模型消费者与 SelectView，以及 Gallery 的共同焦点/激活适配。真实消费者首批覆盖 Menu、Select。FilteredMenuView/ComboBox/CommandPalette 还需核验过滤索引到原始索引的适配；通用组合 SemanticPart 的完整嵌套树、其余生产者、交互 Inspector、菜单专用通知/取消动作及完整 VoiceOver 验收继续作为未完成项。非模态菜单没有被当作整个窗口的 Dialog；复用既有焦点和关闭能力。

公共 API 独立消费者4项新增回归覆盖内部状态隔离、外部模型、过期 count、禁用/失活、Focus 不提交、退出绘制及 Reduced Motion。Gallery 新增4项回归覆盖 Menu/Select 真正节点与动作、重复触发器关闭、外围焦点转移、禁用/空选项、重新展开当前选择；当前全部可访问性回归17项通过。

8个 token 探针重新执行，4500个有界案例，complete=false；后续仅 Gallery 适配修复，探针的公共源输入保持一致。最终全 workspace、消费者、独立重放、原生/Gold7 与源码身份以本批 receipt 为准。原型与最终原生目录分开，不能用原型失败场景冒充最终验收。不测性能；VoiceOver、物理 IME、完整主题/品牌原生矩阵不由自动/GPU/AX 快照替代。

收到的跨会话气泡批注图标请求单独登记；对方已选择复用现有 tekes-side-chat 资源，命名别名可后续安排，不改变本会话不提交/推送/发布的授权边界。AgentKit 冻结候选未重采集。
