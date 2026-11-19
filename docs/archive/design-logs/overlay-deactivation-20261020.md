# 浮层窗口失活与普通退出整改

本轮基于全部当前 WIP，针对 #9 的窗口失活过渡及普通关闭契约，保持 #3/#4/#5/#14 的证据边界。未宣称原 Issue 或第六批完成。

根因在共享呈现生命周期：Menu/Popover 及 Dialog 取消输入后仍可能保留待完成过渡。明确的窗口失活需要停止帧请求，普通退出则仍应保留绘制；不能把所有 Cancel/blur/deactivate 一律等同于失活。

共享修复复用 control_motion.rs 的 OverlayPresence/ProgressTransition：已有呈现对象增加私有 settle(open)，使用原有 snap，逻辑打开的模态保持静态画面，已关闭的瞬态清除呈现。composite.rs 在 MenuView 的 owned/bound Window(false) 路径清理，DialogView 在窗口失活和失活绘制时收敛逻辑终态；feedback_view.rs 的 PopoverView 失活关闭并清理。layout_view.rs 的 FilteredMenuView::dismiss 复用既有 deactivate_with_model，清掉对外命中布局并保留退出绘制，cancel 仍清除呈现。

适用范围包括 Menu、绑定菜单模型、ComboBox/CommandPalette 的过滤菜单、Popover、Dialog。Rich Dialog Content 复用原生宿主已有 motion context，按系统偏好或当前窗口失活计算有效无动画输入，未修改系统设置。普通 show_dialog 也会 Cancel 手势，因此曾试验的“任何 Cancel 都结束 Dialog 动画”已完整撤销，原适配器字节恢复，正常进入回归继续通过。模态失活保持打开是合理例外；瞬态 Menu/Popover 关闭。没有新增公共协议、token、组件或 UI 框架。

五项新增控件回归与完整 355 项 controls 测试通过；150 项公开消费者测试包括真实 CoreText Rich Dialog 的有效时钟输入、正常进入及恢复不重播；controls、native 示例和测试宿主严格 Clippy 通过。九个真实 token 探针共 4416 个有界场景已重新执行，complete 仍为 false。四主题、三品牌、窄宽、@1/@2、进入/退出中的失活共 144 组、576 帧真实 GPU 核验通过。GPU 层依赖未因原生宿主的测试入口变化而改变，依赖身份单独留证，未冒用其他层输入。

最终外接屏 U2790B@2 签名小宿主通过 17 个普通/退出中重开/Reduced Motion 命令检查点，保存实际画面。另三个步骤从真实浮层首帧开始，立即调用自身 AppKit 窗口最小化并恢复；真实 Focus(false)/Focus(true) 和 is_active 证明待完成过渡立即停止，Menu/Popover 恢复后保持关闭，Dialog 保持逻辑打开但不重播。恢复入口仅存在测试宿主，复用平台已用的 AppKit 激活方式，不能手工赋值活动状态。此为工具驱动原生生命周期证据，不冒充物理键盘或 VoiceOver 听读；不测性能。

Gold7 核对限于上述真实连续反馈、中断、重开与逻辑输入边界；该 motion 宿主不验证完整 AX 树、物理 IME 或实际调用者按钮的焦点恢复。其余内容槽、调用者恢复、其他消费者和整页最终版本仍须继续验收。Window Toolbar 仍为后续架构规划。新开放 #25 已读取并记为独立阅读依赖，不自动扩大本轮实现范围。

并发检查发现另一条 3D 工作线修改 README、构建脚本和 viewer；均未触碰或验收。全局能力投影在并发变化期间的一次 stale 检查保留，随后机械重新生成/检查通过；3D 不在本轮 API/GPU/native 源码依赖内。不得把投影刷新当作 3D 体验通过。

验收后核实窗口/进程退出、签名和源码身份，再清理本轮运行目录、精确测试可执行文件和宿主包，共享依赖缓存保留。代码、日志、图像及有界源码快照保留，快照不是完整可发布闭包。未提交、推送或关闭 Issue。

证据目录：design/reviews/overlay-deactivation-20261020/。入口为 acceptance.json、native/result.json、native/evidence-manifest.json、gpu-layer-verification.json、fresh-probe-summary.json、protection.json、cleanup.json。中间失败/被淘汰版本单独保留，不计作最终版本通过。

封存后的最终边界：3D 工作线继续修改 scene3d camera/GPU 与 viewer，全局能力投影再次过期，最后一次 --check 未通过；明确不宣称全局最终门禁全绿。组合审计及浮层 API/GPU/native/token 输入身份仍通过且无漂移，3D 不是本小宿主依赖。详见 global-gate-boundary-final.json，后续在该工作线稳定后刷新机械投影。
