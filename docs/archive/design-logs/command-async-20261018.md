# CommandPalette independent async caller · 2026-10-18

基于全部当前 WIP；保留原 #3/#8/#12/#14，纯 Rust、JSON DS 唯一真源。不提交、推送、关闭 Issue，不测性能。

## 根因、共享位置与合理例外

本项属于公共组件契约的真实消费者接入缺口；现有 CommandPaletteView 已提供触发器、查询候选菜单、重复确认、焦点转移和取消能力，现有 TextFieldContent 提供真实 TextSurface/IME/选区。现有 ContentHost 提供合成编辑器和指针捕获的优先路由。检查未发现需要新增共享原语或 token，因此不修改库、不新增异步协议或抽象层。

examples/composition-consumer/src/command_status.rs 用同一个 Commands 组件保留原 CommandPaletteView 与 TextFieldContent，复用现有 Status/Feedback/Progress/Banner/Stack。触发器与查询编辑器是同一命令控件的互斥焦点目标：打开时原触发器 blur、原编辑器 focus；关闭时编辑器 blur、原触发器恢复焦点，语义节点由 Button 切到 TextInput。原触发器仍按公共 View 绘制，不复制按钮样式或键盘状态机。原生语义角色切换/AX 对应关系仍须原生验收，不能由公共语义动作测试推定。

后台状态切换保留查询、IME marked、选区、候选和焦点；合成期间拒绝数据替换，控制键仍交给原编辑器，Escape 先取消预编辑、再次 Escape 关闭命令查询。数据响应替换取消旧候选捕获，按唯一命令标签重映射仍存在的候选；重复标签被拒绝。最后执行记录是消费者历史，命令从响应中移除不抹掉历史，不是通用稳定 ID 协议。初始空数据允许编辑/加载反馈，数据到达不偷走焦点。确认同一命令仍发出执行事件。

适用：调用现有 CommandPaletteView 的命令查询消费者；同类文本状态使用既有 TextFieldContent/TextSurface 和共享 ContentHost，不迁移其他组件。合理例外：异步状态由消费者拥有，不给每个控件强加 loading/error 布尔标记；单独使用 View 的宿主继续负责窗口焦点/事件整合；fixture 使用 Reduced Motion 取得确定性结果，不代表完整动效验收。

反馈位于命令控件前方，用现有 Stack 测量布局，避免候选弹层遮住失败信息；候选在公共 overlay pass 末尾绘制。Gallery States 加入缓存加载、后台失败和初始空加载三个真实消费者样例，按公共 MenuView 的测量为展开菜单保留样例间距；仅 composition-review/test 接入，默认构建不扩迁移。

## 分层证据

公共 API / 真实接入：独立 workspace 的 command_status 三项测试覆盖原触发器 Enter/Space/指针、打开/关闭焦点、真实 CoreText/TextSurface、后台状态、预编辑/选区、捕获释放、数据替换、原语义动作及重复执行。四主题三品牌 × 两种数据源 × loading/error 共 48 条实际 token 路径，断言 query 保留、输入边界/填充和错误反馈语义，不套按钮状态。

全部独立消费者回归、两侧 strict Clippy、窄宽/大字 Gallery fixture、默认/review 构建、能力登记/架构/Gold7 机械/纯 Rust/格式门禁分别留日志。公共测试 glyph painter 为 no-op，不替代 GPU 或原生体验。

真实 token：本批重采 command-async168、query-async264、navigation-content408；input612、popup192、action576、selection2016 的源码绑定未变，复用上一冻结批次且再次核对全部哈希。合计 4236 条有界观测，complete=false，不宣称穷尽 token 依赖或状态组合。

GPU/Gold7：CommandPalette States 144 页，四主题、三品牌、1x/2x，窄宽/大字/滚动位置；对照历史同组件冻结参考。原生 Gallery 使用本次 composition-review 签名后二进制，检查完整和 420pt/150% 应用预览，留图、源码身份和退出记录。统一收据：design/reviews/command-async-20261018/receipt.json。

边界：静态 review 样例不证明实时原生异步切换；physical IME、完整原生控件 AX/VoiceOver、完整动效和全组件统一体验仍未完成。125 项 public 状态、131 项 fixture 缺口、required269；choice profile 的公共 fixture 缺口本批补齐，不代表原 Issue 已可关闭。
