# ComboBox retained query / async review · 2026-10-18

基于当前全部 WIP，不提交、不推送、不关闭 Issue，不测性能。原 #3/#8/#12/#14 继续保留；这是公共契约、真实接入和体验的分项证据，不是统一验收完成声明。

## 根因及共享修复

ContentHost 原先先把键盘及指针事件交给弹层：正在合成的编辑器可能被 Enter/方向键/Escape 抢先确认或关闭；选区拖动中打开候选菜单后，松手被新菜单吞掉，编辑器的 pressed/capture 未结束。两个问题均在原 ContentHost.dispatch 修复，复用 focused/composing/captured，不添加状态层。合成中的控制键先交回原编辑器（保留 Escape 先取消合成）；被捕获的移动/松手先交给原指针主人。无捕获的候选菜单照常使用原路由。

适用：所有 ContentHost 中的合成编辑器和指针拖动组件，包括 Rich Dialog/TextFieldContent 与组合候选菜单。合理例外：独立使用 View 的宿主仍负责自己的事件分发；系统 IME 的未处理按键继续交给原生输入法，不能自动当作菜单快捷键。公开回归使用原 MenuContent 和现有 Drag/Editor；真实消费者另验原 TextFieldContent。

## 独立消费者

examples/composition-consumer/src/combo_status.rs 的 Query 适配原 TextFieldContent、ComboBox 与 ComboBoxView；保留同一 TextInput/TextSurface，复用已有 Status、Progress、Banner、ContentHost、Stack。后台 Loading/Error/Ready 不替换草稿、选区、marked、已选值、焦点或候选；内容失败和字段 validation 分开。数据替换拒绝 marked 期间更新及重复/丢失保留选择的响应，按唯一标签的示例领域身份重映射选择并取消旧菜单捕获。唯一标签是这个消费者的领域约束，不是通用稳定 ID 协议。

候选绘制复用原 ComboBoxView 并在基础反馈之后，字段绘制复用 TextFieldContent/TextSurface。Gallery States 中加入 loading、failure、validation 三个真实消费者样例，限定 composition-review/test；默认 Gallery 构建不扩入试验消费者。

## 分层验证

公共 API：编译成功的 IME 与捕获 red/green、原 composition 回归。真实接入：ComboBox 三项独立消费者测试，覆盖真实 CoreText/TextSurface 的后台状态、预编辑/Escape、选区、候选、指针捕获/响应替换、重复确认/语义动作；36 个四主题三品牌 async/validation token 路径。原共享包及全部消费者回归、strict Clippy、格式与仓库门禁分别留日志。

真实 token：query-async 264 条与既有 command/popup/action/input/selection/navigation-content 合计 4188 条有界观测，全部绑定当前源码哈希。不是完整组合或依赖穷尽声明，不新增 token/按钮外观/框架。

Gold7：真实 GPU 的 ComboBox States 144 页（四主题、三品牌、1x/2x、宽度/大字/滚动位置），与历史同组件冻结参考对照；原生 Gallery 检查真实 review 二进制、完整预览和 420pt/150% 预览，分别留图与签名后二进制身份。详见 design/reviews/combo-async-20261018/receipt.json。公共测试的 glyph painter 为 no-op，不能代替 GPU/原生体验。

边界：静态 Gallery async 样例不能证明原生实时异步切换；physical IME、完整原生 AX/VoiceOver、完整动效仍待验收。两个 CommandPalette loading/error profile 要求继续开放；123 项 public 状态、133 项 fixture 缺口、required269，不扩大迁移。
