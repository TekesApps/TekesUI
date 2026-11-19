# Select 空选项与验证错误

基于bc3de79加全部WIP，GitHub仍为14个开放Issue、0个全状态PR，更新时间与账本一致；继续原 #1／#3／#8／#12／#14，不关闭Issue。

## 根因、共享修复、适用范围

- 公共组件契约：空选项仍能键盘打开固定选项菜单，选项在按压期间清空也可能遗留旧按压。SelectView现在在空labels时复用既有cancel与不可用触发器呈现，指针/键盘按当前模型count提前取消，wheel在空集时忽略。所有SelectView消费者受益；恢复数据后只有新输入才能打开。不清空选中索引，不伪造占位选项；空集时该索引不代表有效选择。
- 公共呈现输入：共享Button内部已有JSON定义的验证错误边界，SelectView没有公共投影。新增set_error(bool)直接投影已有状态，不新增token、样式、运行时类型或协议，不禁用修正或重置值。验证结果仍归调用方管理。
- 真实调用方：Gallery复用原属性组增加Error/Empty options输入，空集触发器退出逻辑Tab顺序，反馈用既有测量和文字绘制。Gold7原生复查发现本批新增反馈在popup之后绘制，覆盖菜单文字；已把调用方基础反馈移到原View之前，菜单按既有绘制顺序覆盖基础内容，并新增真正失败后通过的Scene顺序回归。没有新overlay层或回调协议。

合理差异：普通Menu保留其原空容器语义；Select的空集没有可选择的操作。ComboBox/CommandPalette允许空查询结果并保留可编辑TextSurface，其验证错误已能由TextSurface.error表达，不给它们套Button皮肤。本批error是输入验证，调用方异步内容加载失败仍未计为完成；加载状态继续由后续消费者场景核验。

## 分层验证与边界

公共API：空Select回归修复前已编译运行并失败，修复后choice_contract共14项通过，覆盖空集不打开、count在下一帧前变空取消按压、数据恢复不复活旧arm、已打开菜单清空后无popup、恢复后新输入有效。精确状态探针从180扩至204条：Select9状态108条＋ComboBox8状态96条，四主题×三品牌；新增空集与验证错误的实际图元/值/菜单断言。预期token读取在捕获窗口之外。

独立消费者：CoreText＋原SelectView验证错误边界和focus ring独立、错误期间可选择修正、清除错误保留值、空集无菜单且不生成值、恢复数据仍保留原索引。首次测试在打开后未布局就直接确认，被既有可见候选门禁拒绝；补上真实呈现帧后通过，不把测试流程错误当产品缺陷。消费者共34项通过；控件/token/design-export共343项通过。字形后端为空的公共测试不称像素验收。

真实接入：4项Select真实回归、完整Gallery36/76确认与焦点回归、窄宽正常/大字号矩阵通过；原Popup几何探针中的空Select预期改为无popup而保留触发器，普通Menu预期未改。根及消费者all-targets严格Clippy通过；能力37项、组合6项、reference、Gold7机械检查、pureRust、fmt通过。

当前token绑定7组重新执行：204/120/192/576/612/2016/408条案例。仍complete=false、有界提取；Select验证错误使用共享error映射，空触发器复用不可用外观，没有JSON之外颜色。

GPU：四主题×三品牌×@1x/@2x，共144张最终Select状态整页；与choice-focus冻结的同组件较早144张逐张对照，新增两行和真实属性使画面变化，此参考不称本批即时基线重跑。代表性Light窄宽150%与Dark IC品牌2@2x核对错误边界、空反馈和裁剪。证据刷新与反馈顺序修复后重新渲染，实际状态样例与刷新前一致；Playground展开时的绘制顺序由独立Scene回归和最终原生图证明。

原生：只操作Gallery，不改系统设置。签名后记录应用哈希和源输入；中间版本原图及身份独立保留，最终版本重新验证错误与焦点共存、可选择修正、菜单不被反馈覆盖、空集点击/Enter不打开、恢复选项保留值且新点击可打开。中间帧呈现过反馈覆盖缺陷，保留为前对照，不能算最终通过。AX仍只见窗口/菜单，控件AX、VoiceOver、物理IME未验收；验收应用退出后检查清单为空。

统计为120项精确公共状态、136项fixture缺口、269项required。选择profile剩余5项：Select loading，ComboBox loading/error，CommandPalette loading/error；此外Select异步内容失败组合、完整体验和AX仍在原账本中，不因验证错误通过而缩减范围。下一步复用Progress/Banner/EmptyState与既有公共组合能力处理调用方异步场景。

完整输入哈希、源差异、红绿日志、GPU、原生和WIP保护见design/reviews/select-availability-20261018/receipt.json。历史冻结证据不变，未测性能、提交、推送、打tag、关闭Issue或发布。
