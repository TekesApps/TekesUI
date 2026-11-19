# CommandPalette 结果状态与重复确认事件

基于 `bc3de79` 加全部 WIP。当前GitHub仍为14个开放Issue、0个全状态PR，更新时间与原账本一致；继续原 #3／#8／#12／#14，不关闭原Issue。

## 根因、共享实现与影响范围

ComboBoxView.key() 与 CommandPaletteView.key() 把选中索引是否变化当作确认是否发生。第二次确认同一选项/执行同一命令时，模型已成功关闭并确认，但便捷接口返回None，消费者漏掉回调。两项已编译执行的修复前失败案例留证。

共享修复位于 `crates/tekes-ui-controls/src/layout_view.rs` 的上述两个key方法：复用原key_event结果，只在Enter按下成功确认、菜单已关闭时返回选中索引。所有使用这两个公共便捷接口的消费者受益；不增加事件类型、状态、token或抽象层。取消、导航、key-up、禁用、空结果仍不产生选择事件；按相同值确认与值变化保持不同语义。

Gallery本来使用key_event，重复命令并未被该缺陷影响。其真实交互回归和原生操作证明行为一致，不声称修复前Gallery重复执行失败。指针确认原来已返回每次激活，未重复改写。

CommandPalette的selected表达最后执行的真实命令，不是持久模式切换，也不把触发器画成Button.selected。新增状态样例使用原View的Down/Enter完成选择，并复用现有测量、标题、caption和裁剪；打开和空结果保留调用方TextSurface及过滤菜单所有权，不新增状态层。加载/错误与Select空内容仍单独处理。

## 分层验证

- 公共API：choice_contract共13项通过，其中2项红绿回归分别验证重复ComboBox确认和CommandPalette执行；空结果、取消、key-up不激活旧值。新增36条精确案例覆盖四主题、三品牌、selected/open/empty，验证原View、查询表面、真实选择、重复激活、关闭状态、空结果无幽灵菜单、输入底色与必要边界。预期token读取在观测窗口之外；CoreText排版加无字形绘制不代表像素验收。
- 独立消费者：新增公共API场景分别接收两次相同ComboBox选择、两次相同命令激活，再用原TextSurface验证无匹配时查询保留且不执行旧命令。消费者共33项通过。
- 真实Gallery：CommandPalette相关8项回归通过，覆盖实际重复命令日志、空查询不变、输入焦点、IME preedit取消、关闭恢复、禁用/卸载调用方和测量；状态矩阵在正常/大字号窄布局通过。测试调试时曾把保留最近6条的日志当无限历史计数，已改为在空结果Enter前后比较当前日志快照；不把该测试错误当产品缺陷。
- 控件/token/design-export共342项通过；根与消费者all-targets严格Clippy通过。能力门禁37项、组合门禁6项及reference/Gold7机械检查、pureRust、fmt通过。
- 真实token：共享源改变后，7组当前绑定证据全部重新执行。CommandPalette观测合并既有84条触发器与36条结果案例，共120条；其他当前观测为180/192/576/612/2016/408。仍是有界观测，complete=false，不声称完整依赖提取。
- GPU：四主题×三品牌×两种比例，生成144张当前整页。与choice-hover冻结的同组件144张对照均有变化，原因是增加真实执行后样例导致后续内容重排；这是较早参考，不是本批即时基线重跑。代表性Light窄宽150%及Dark IC品牌2@2x帧核对执行结果、触发器/输入/候选层级和裁剪，未宣称所有体验组合完成。
- 原生：只操作本批签名后留哈希的Gallery。Light鼠标打开、Enter执行、再次打开执行相同命令、查询无结果后Enter保持输入与反馈、Escape恢复调用方，均留图；状态矩阵中执行后caption可见且不假用Button.selected外观。事件日志仅显示最近3条，公共与Gallery测试另外证明重复次数；不凭单张日志图声称无限历史。AX只见窗口/菜单，未完成控件AX/VoiceOver/物理IME验收；Gallery已退出。未改系统设置、未测性能。

公共状态从115增至118，fixture缺口从141降至138，required269保持不变。选择族剩余7项：Select empty/loading/error、ComboBox loading/error、CommandPalette loading/error。后续区分调用方异步内容失败与输入验证错误，优先复用TextSurface、Progress/Banner/EmptyState和既有消费者组合；不因便利层缺少参数就直接增加新层或把状态判为不适用。

当前账本和源行锚点已同步。完整源差异、红绿日志、当前token/GPU/原生证据和WIP保护见 `design/reviews/command-results-20261018/receipt.json`。全部历史冻结证据保留，不提交、推送、打tag、关闭Issue或发布。
