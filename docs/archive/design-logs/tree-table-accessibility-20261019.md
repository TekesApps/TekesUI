# Issue #5 / #14 — Tree、DataTable 真实语义和生命周期

当前基线为 HEAD80d6b05 加全部 WIP；GitHub 刷新仍为21个开放 Issue、0个开放 PR。此前过滤菜单批次已封存，继续保护原有证据与 WIP，不修改 AgentKit 冻结候选、不提交或发布。

根因分类为公共组件契约、基础挂载守卫和宿主适配。TreeView 已有原模型、布局、键盘和指针能力，但缺少“设置明确展开状态”的入口。DataTableView 的表头键盘路径只核对模型列数而未核对当前布局，旧 focus 可在未挂载时排序；行导航也需要与 List/Tree 一致地拒绝未挂载源索引。默认 Gallery 尚未导出两个真实生产者。

共享修复：TreeView::set_expanded 使用既有 disabled/window-active、模型可见性和实际布局守卫，复用 Tree::toggle，幂等请求不反向切换。收起隐藏当前焦点/选择时恢复到祖先，释放旧指针捕获；叶子不支持展开。原 Enter 激活不被改成展开。DataTableView::key 在所有表头和行键盘路径之前校验模型范围和当前布局；已挂载但被 clip 隐藏的逻辑目标仍可供键盘导航和宿主 reveal。原排序、选择和视觉顺序仍由原模型及布局处理，没有第二份状态机。

公共平台角色补上 Tree/TreeItem、Table/Column/Header/Row/Cell，索引、层级、排序信息放在角色专属数据中，复用现有 AccessibilityNode 的 parent、expanded、selected、frame，未增加通用必填字段。原生使用 Outline/Row、Table/Column/Cell 和实际可操作表头 Button；提供 rows/columns、当前可见/选中关系、disclosed rows/by row/level、单元格行列区间和排序方向。SetExpanded(bool) 复用原生 expanded/disclosed setter，叶子与失效原生对象不能调度。关系查询仍读取当前 accepted snapshot 和原 native cache，保留已有 key+对象身份围栏。角色和动作枚举新增变体需要外部 exhaustive match 迁移；当前独立 native 组合示例不挂载 Tree，明确拒绝 disclosure，不把它误当 Press。没有为此扩增通用组合协议或组件范围。

平台接口依据：[Apple NSAccessibilityOutline](https://developer.apple.com/documentation/appkit/nsaccessibilityoutline)、[NSAccessibilityProtocol](https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol)。这些接口映射不等于完整 VoiceOver 验收。

真实接入：Tree 使用实际布局深度、源节点标签、分支状态和父子关系；收起后立刻过滤不可见后代，原键失效。DataTable 使用实际绘制的行/单元格文本、列几何及排序顺序；语义 key 保持源行索引，AX 行位置使用显示序号。表头的装饰性 ↕ 不重复进入标题，排序方向单独导出。只导出真正可见槽位；禁用保留可读节点，拒绝动作。Workbench 公共 focus_control 现在拒绝窗口失活时改变焦点。Workbench 保留宿主窗口 active 状态，复用原有每个 StoryState 的窗口状态应用代码，让后创建及 Reset 后的消费者也继承失活状态，避免只更新已经存在的页面；所有使用这些入口的现有组件统一受益。

原生验证还发现并修正了共享桥接缺陷：旧 Objective-C 方法名为 accessibilityIsSelectorAllowed:，而 AppKit 协议实际调用 isAccessibilitySelectorAllowed:，原策略未生效。新增 disclosed 接口使这个问题暴露为所有按钮/文本框都显示 Expand。修正名称后按角色限制展开、表格及单元格属性，普通读取交给 superclass；保留原生失败快照和最终通过快照。树分支才提供 Expand/Collapse，叶子和普通控件不提供。既有 ComboBox 文本编辑/确认与 Select 打开/取消均以选择器修复构建实际回归；随后单元格键修复另用最终构建留证。这属于原 #5 桥接验收范围，不另建重复 Issue。

公共 API 独立回归覆盖未挂载表头/行、源索引排序、失活和恢复后无旧焦点复活；Tree 明确/幂等展开、收起恢复焦点与选择、叶子/禁用/失活拒绝。旧表头单测此前完全没有 arrange，却期待能排序，已改成真正渲染表头后验证排序；保留独立编译的 red/green，不删除断言。语义单元格键沿用实际已绘制布局的列数；模型列数在重绘前缩减时，不能用新列数重解释旧文本槽位，已补真实回归。真实 Gallery 回归覆盖 Tree 分支关系与原激活、Table 列/表头/15个实际单元格/排序后原始索引、禁用/窗口失活/页面切换后的旧目标拒绝。

分层证据与最终计数见本批 receipt。原生全部在外接 U2790B scale2 Light Gallery 验证，保留整改前后真实 GPU 截图、AX 树、动作日志及425项实际 cargo depfile 输入身份。共享 macOS AppDelegate 还补齐应用激活/失活通知，并将窗口 become-key 与应用 isActive 联合判定。跨应用失活时窗口可能仍为该应用的 key window，不能只监听窗口通知。统一复用 Event::Focus，不另建应用状态机。Focus 事件即时复用 sync_accessibility 发布当前模型状态，不依赖被遮挡窗口下一次绘制；Window::is_active 提供应用 active 与 key-window 的有效作用域，Created 后立即发布初始 Focus，避免默认把后台新窗口当作活跃。诊断输出分别记录 key-window、有效 active 与宿主 active。原生最小化后 focused=0、选择保留已经核对；当前 CUA 的 Raise 和 Gallery Window 菜单没有恢复实际最小化窗口，完整原生恢复保留人工验收，API 与真实 Gallery 的 inactive/active 恢复回归单独通过。窗口关闭通过既有 close-readback 验证 accepted_nodes_after_close=0，不能把磁盘上的最后一帧 native-tree 当成关闭后 readback。

Gold7 仅对此有界场景分别记录公共契约、真实接入与可见体验；沿用共享中性阅读背景、Quiet 行和表头、树缩进及焦点几何，无页面样式补丁或新增 token。全部主题/品牌、物理 IME、完整 VoiceOver、完整网格/大树辅助技术遍历、最小化恢复和真实跨应用失活/恢复通知体验仍单独列为未验证，不由角色存在或自动测试替代。不测性能；#5/#12/#13/#14 保留剩余原验收条件，不关闭 Issue。
