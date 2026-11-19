# RadioGroup 禁用期间迟到焦点整改

基于全部当前 WIP，本轮仅修复 RadioGroupView 的整体禁用契约，不提升整个组件族成熟度。

根因：`focus(model, index, visible)` 只检查子项可用性，没有拒绝整体禁用的组。迟到请求在恢复启用后遗留焦点，随后 Space 会意外选择子项。两项永久回归先在旧实现失败，再在修复后通过。

共享修复：`crates/tekes-ui-controls/src/selection.rs` 的既有 `RadioGroupView::focus` 复用 `blur/cancel`，整体禁用时拒绝请求。适用于全部该 View 消费者，包括 Gallery 与独立消费者，不增加 token、协议或抽象层。RadioGroup 保持 Space 按下时选择的既有语义；不强套按钮释放语义。Slider 的 disabled 在模型中，须另行核验，不能据此推断通过。

验证分层：350 项 controls 测试，新增 30 组禁用焦点矩阵和一项恢复隔离回归；720 组真实 CoreText 消费者场景；真实 native MouseEvent 适配回归；严格 Clippy。48 组真实 GPU、144 帧覆盖四主题、三品牌、窄宽与两种比例及错误状态，布局/选择不变，恢复后无残留焦点环。GPU 对照使用显式新焦点重建旧错误状态，并非旧二进制 GPU 截图。

外接屏 U2790B@2 的 15 个原生检查点覆盖 Tab、Home/End、绕过禁用行、禁用迟到请求、恢复后的 Space 隔离、新焦点后的选择、左右导航、AX Press、真实鼠标与禁用行隔离。原生输入事件与后状态均处于活动窗口，连续区间无失活。Gold7 核对限于上述既有导航、焦点、选择和隔离体验；未宣称 VoiceOver 人声或物理键盘验收，未测性能。

窗口关闭、进程退出、未使用检查后清理本轮运行目录、宿主包和测试可执行产物；公共依赖缓存保留。代码、输入快照和证据保留。未提交、推送或关闭 Issue。

证据：`design/reviews/radio-disabled-focus-20261020/acceptance.json`、`native/result.json`、`native/evidence-manifest.json`、`gates.json`、`protection.json`、`final-verification.json`。下一步继续第五批 Slider 最终源码状态核验。
