# List／Tree／DataTable：当前模型与捕获取消

根因在既有 `collections_view.rs` 的输入契约，而非单页样式。指针使用上次布局命中的行，却没有像键盘路径一样核对当前模型可用性；Tree 父节点在绘制之前已折叠时，旧几何仍可能选中或展开隐藏子项。三个集合的 Escape 只当普通已处理按键，没有释放持有的行捕获；模型随后恢复时，迟到的鼠标释放可能重新选择或展开。

修复复用私有 `CollectionInteraction`：按下／释放先检查现有模型的行数、列数、节点可见性与 disabled；Escape key-down 先取消持有的捕获，再检查旧焦点是否仍可用。已消费的取消返回 Handled；没有捕获则 Ignored，让父作用域继续处理。Escape key-up 不取消新手势，不回滚此前有意完成的行选择。原有 window inactive、disabled、显式 cancel 和卸载边界保持有效。

适用的是使用这三个实际 View 的所有调用者，包括既有 Gallery 和独立消费者，不限于复现 Tree。DataTable 按 source row id 工作，排序展示的位置不变成新的行身份。已有有限且已挂载但在祖先 viewport 外的键盘目标继续保留，不能把 paint clip 当作模型可用性。合法数据重排与尺寸变化不一概取消输入。

Gallery 的 Tree 转发还有同一类问题：它在收到 Escape 时先过滤隐藏焦点，挡住了公共 View 的取消。此处仅让 Escape 到达现有 View，其余键的 live-focus 检查保留。没有另写选择／展开／捕获状态机，也没有新增 token、公共类型、组件或框架。

合理边界：List／DataTable 的旧模型使用位置型 source id，不能因此宣称提供跨替换文档的业务稳定 key 或虚拟化重建；异构 Content 与 #22 文档事务继续使用各自已经公开的稳定身份。表头既有按下即排序不改为延迟释放，取消也不撤销已经完成的排序。Chart 未使用这个集合行捕获契约，不因名字相邻而扩入此次迁移。

公共失败／通过回归在 `controls/tests/collection_live_input.rs`；四项覆盖三个组件、隐藏／删除源、持有期间 Escape 和恢复后的新手势。原来四项全部失败，修复后通过。原生精确导出包 #22 不包含本轮后续修改，也没有被覆盖；不得用该旧窗口认证这些新集合行为。

真实接入验证使用独立小宿主 `design/reviews/collection-live-input-20261020/host`，直接调用公开生产 View、真实 CoreText 和 GPU。144 组／432 帧核对取消后的画面回到按下前同一状态，以及后续新手势可用。这不是 ClientKit 视觉一致性或完整 Gold7 证明。

这组 GPU 为了隔离捕获，基线特意预选相同行；144 例中 held press 与已选基线没有像素差异。它不证明 selected＋pressed 有独立视觉反馈，也不能据此判该组合不符合设计。必需 pressed 状态和全组合的 token／体验覆盖继续归 #1／#12 核验，不从取消成功升级为状态视觉通过。拥有式 Content 使用既有核心取消，不由此次旧集合 View 修复冒认覆盖。

为避免完整 Gallery 应用构建，宿主 build.rs 从生产 `collections_adapter.rs` 原样取出 InputResult 和四个键路由相关定义，编译到仅含所需公共 model/View 字段的测试上下文。它验证实际生产转发函数，不是完整 Gallery 窗口／主事件循环；失败回归明确出现隐藏 branch 被 Toggled，修复后旧释放 Ignored。未改写这些函数以便通过，源码身份与生成边界留证。

日志、源码记录、token 探针及验收边界见本轮 review 目录。新控件 native held-pointer＋Escape、最终整页与 ClientKit 对照仍需原生复验，VoiceOver 不由这里认证。性能不测；本轮不提交、推送或关闭 Issue。
