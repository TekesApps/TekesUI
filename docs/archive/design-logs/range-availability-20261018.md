# Selection availability / Slider state review · 2026-10-18

基于全部 WIP，原 #3/#8/#12/#14 继续保留。纯 Rust、JSON DS 唯一真源；不提交/推送/关闭 Issue，不测性能。

## 根因和共享修复

这是基础交互原语中的可用性观察/取消缺口，不是新增视觉状态。原 SliderView 的 pointer_down/key，以及 SwitchView 的部分 pointer_down/up 遇到 disabled 会提前返回；RadioGroupView 在模型行被禁用后，pointer_down/key 未必释放旧 pressed 行。禁用与再启用之间不经过 render，也可能经事件入口观察到禁用；若只拒绝这一次输入，旧捕获/Space arm 会在再启用后复活。

crates/tekes-ui-controls/src/selection.rs 复用 SliderView.blur、SwitchView.cancel、RadioGroupView.cancel，按各自既有焦点约定释放旧交互。RadioGroup 取消的是被禁用/移除的捕获行，不扩大成禁用整个组；Switch 取消指针和 Space 两个互斥激活主人。无新 token、状态、协议或共享层。三项编译成功的公共 red/green 回归覆盖 Slider六个、Switch六个和Radio四个模型事件边界路径。

适用：所有使用这些原 View 的消费者，包括 Gallery 与组合宿主。合理例外：Checkbox 委托已有 Button 取消逻辑；不带当前模型参数的 pointer_move 无法观察模型的新禁用值；两次调用之间完全未被观察的禁用/启用不能凭空推断。宿主仍须在模型变更后绘制/分发当前模型。缺失的原生 Slider Component range role/actions 适配继续记录为缺失，本批不借状态验证声称完成架构迁移。

## 状态、真实组件与 token

独立 workspace 的 examples/composition-consumer/tests/slider_states.rs 直接使用原 SliderView，验证 rest/hover/pressed/focus/disabled/dragging，值0/50/100、四主题三品牌，共216条。通过真实 pointer/key/drag/release 改变状态；断言渲染不改值、轨道几何稳定、thumb位置符合值、焦点环独立、松手后不继续拖动，以及实际track/fill/thumb边界token。Home在下界不改变值是正常契约，不把changed=false当作未处理缺陷。

Gallery原Slider矩阵补入真实pointer_down+pointer_move产生的Dragging样例；原Workbench的完整thumb命中/保留track/隐藏取消，以及Radio/Switch真实命中模型测试均回归。没有以静态ControlState直接伪造拖动，也未增加组件范围。

公共门禁增加仅用于证据的range_state精确匹配及布尔state_asserted检查，不能由pressed或dragging+focus推定dragging通过。能力登记131项public/125项fixture缺口，required269。所有体验仍单独标记。

共享源码变更后重采全部8组观测：slider216、selection2016、input612、popup192、action576、navigation-content408、query-async264、command-async168，合计4452条，均绑定当前源码；complete=false，不是全组合/全token依赖穷尽。

## 验证与边界

公共 API：三项编译红绿、共享包350项测试；独立消费者43项测试；root/consumer strict Clippy。真实接入：Slider实际Workbench与矩阵大字布局、Radio/Switch实际命中模型测试。能力登记38项和架构6项测试、引用对齐/Gold7机械/纯Rust/格式/default与composition-review构建分别留证。

GPU/Gold7：Slider、RadioGroup、Switch三组件432页，四主题三品牌1x/2x，宽度/大字/滚动矩阵；历史同组件参考单独记录，不伪称立即before重跑。原生Gallery检查本次构建，Slider真实拖动和键盘、状态矩阵完整/420pt/150%预览分别留图；收据 design/reviews/range-availability-20261018/receipt.json。

自动事件测试不替代真实GPU，GPU不替代原生交互，窗口/菜单AX也不等于控件AX/VoiceOver。原生禁用期间丢失松手的完整系统事件序列、完整辅助技术、物理IME、实时异步和全组件统一验收仍未完成；不扩大迁移、不测性能。

## 原生发现：普通移动与按键拖动不能混作同一个测试输入

原生Gallery首次拖动没有改变值（50，之后键盘55），轨道点击可变到90。公开运行时无问题，定位到Workbench将普通Moved/Entered送给Views，却遗漏AppKit真实发出的左键Dragged。将既有SliderWorkbench回归的拖动输入从Moved改为Dragged后，编译测试复现55≠75。修复在原公共Workbench事件分发，五个既有移动分支（Dialog、Tabs、Menu、Select、普通Workbench）一致接纳左键Dragged，不新增交互状态机；SplitView继续使用前置捕获/丢失松手策略。非左键不自动纳入。

保留首次原生失败截图与旧构建身份；重新构建新的v2签名Gallery，重做真实拖动和键盘，以及最终三组件GPU矩阵。只在观察到值变化后标记原生拖动通过。所有组件的完整原生拖动/取消/AX仍须分项验收。
