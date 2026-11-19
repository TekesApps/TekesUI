# Issue #5 / #14 — 过滤菜单原始索引公共契约与真实消费者

基于 HEAD80d6b05 加全部 WIP；重新读取 GitHub，仍为21个开放 Issue、0个开放 PR。历史封存证据保持原样，本轮不修改 AgentKit 冻结候选，不自动提交或发布。

根因属于公共组件契约和宿主适配。FilteredMenuView 已保留过滤行到原始索引映射，并复用外部 MenuRuntime，却缺少以原始索引执行语义聚焦和确认的入口。Gallery 的候选激活捷径直接写 selected/open，与原指针和键盘路径分叉；CommandPalette 查询框已绘制，却未登记现有文本逻辑焦点，因此未被导出为实际可编辑节点。

共享修复在 layout_view.rs：FilteredMenuView::is_current 统一校验 open、当前布局、过滤映射及实际候选标题；focus_item/activate_item 接受原始 source index，反查过滤行后使用现有 MenuView 外部模型接口和 MenuRuntime 守卫。Focus 只更新 active，不提交；确认复用 ComboBox::key(Enter)。ComboBoxView、CommandPaletteView 的 focus_option/activate_option 只是这一能力的适配；现有指针确认也统一使用原模型 Enter。渲染参数 active 仍为过滤行位置，mapping/original_index 明确为原始索引，避免改写既有 API 含义。没有新增 token、状态机、样式层或框架。

适用范围为共享过滤菜单及现有 ComboBox、CommandPalette 消费者。合理例外：键盘在新查询到下一帧之间仍可按原模型处理最新候选，不要求使用上一帧映射；语义/指针操作必须匹配实际已呈现快照。语义 key 的原始索引在当前源列表内稳定，不是跨列表重排的持久 ID；宿主仍需既有挂载代际/原生旧对象围栏。过滤后保留同一候选身份且标题未变，快照仍有效。

Gallery 使用实际 TextDemo、候选布局和源索引导出 Menu → MenuItem；只发布可见且当前的候选，part39 为容器，part40+original 为候选。查询框仍是实际 IME 所有者，活动候选和提交值分别使用 focused/selected。CommandPalette 增补原查询框逻辑焦点登记和真实触发器节点；确认后复用 close_command_palette 恢复调用者焦点。外围搜索/侧栏接管时复用现有 cancel/blur，移除两个过滤弹层。原始候选确认回到既有事件日志与输入值同步，不产生第二份选择状态。

公共 API 独立测试3项覆盖非零起始过滤映射、Focus 不提交、一次确认、重复旧动作拒绝、查询/标题变动、空结果、禁用及窗口失活。真实 Gallery 测试3项覆盖两个消费者实际节点/查询、确认回调一次、调用者恢复、旧节点和空候选拒绝、TextPreedit 隔离及外围焦点接管。TextPreedit 是合成协议证据，不是物理输入法验收。

外接屏原生证据位于本批 native：U2790B、builtin=false、scale2，实际 Gallery GPU 绘制；整改前 ComboBox 候选可见但语义树没有候选，整改后 Com 显示 Compact/Comfortable，辅助确认 Comfortable 回原始索引2；CommandPalette 的 Search files 单个结果仍为原始索引2。实际键盘 Down/Escape、空结果、关闭后移除节点、外围焦点接管及命令触发器恢复分别留证。最终 accepted 构建的实际 cargo depfile 逐输入 SHA256 绑定当前代码；current 目录记录此前等价逻辑的构建过程，不能代替最终源码身份。

Gold7 分层：公共契约由独立消费者测试覆盖，真实接入由 Gallery 回归及原生动作覆盖；体验仅对本轮 Light/U2790B 场景作有界核对。阅读背景、输入边界、菜单层次、焦点几何和空反馈沿用共享定义；无页面局部样式补丁。G5 预编辑隔离有自动协议证据，物理 IME 和完整 VoiceOver、全部主题/品牌/Reduced Motion 原生矩阵仍未验证。G1–G3 可见外观核对和 G7 真组件接入不能推导整个 Gold7 完成；性能未测。

8个 token 探针重新执行，4500个有界案例，complete=false。全部验证计数和 WIP 保护、独立重放结果以本批 receipt 为准。#5/#12/#13/#14 继续按剩余原验收条件推进，不关闭 Issue；后续优先核对其余现有语义生产者及跨窗口/挂载生命周期，完整物理 IME/VoiceOver 保留人工边界。
