# Gallery 瞬态浮层失活呈现清理

基于全部当前 WIP，在 AgentKit 独立验收公开 4310937 期间推进。188 公共输入及旧 e4/037 封存包均未改动；本批私有修复没有提交或推送。

根因属于 Gallery 宿主对已有公共生命周期契约的接入遗漏：窗口失活关闭 Menu/Popover 模型，但未显式清理保留的退出画面。隐藏窗口不保证绘制失活帧，立即恢复时旧画面会重新参与退出呈现。测试以“失活后不绘制，直接恢复”为触发条件，先红后绿。

修复只复用现有能力：`StoryState::set_window_active(false)` 调用 `MenuView::clear_presentation`；`ExtendedStory::set_window_active(false)` 调用既有 `clear_overlay_presentation`。没有新增 token、状态、副本或抽象层。

适用 Gallery Menu、Popover。Select 已通过现有 cancel 清理呈现，作为回归对照；Dialog 的逻辑模态状态会跨临时失活保留，属于合理例外，不强行按瞬态 Menu 清除。恢复时按钮自身的合法焦点反馈也不算旧浮层残留：初次宽泛帧请求断言误包含该反馈，最终测试改为核对浮层自己的呈现 runtime；相关过程日志完整保留。

验证：针对性回归通过；真实 Gallery 交互 87 项、AX 29 项及严格 Clippy 通过。公共 API 仍为相同 188 字节输入，未新增接口；当前能力清单已重新生成，组合审计逐项复核两处私有差异并绑定新来源，45 组件检查及 6 项负例通过，不晋级原生或体验成熟度。

新修复的原生窗口、GPU 中间帧及 Gold7 体验仍待复验，不能用自动测试或上一批的 Dialog 原生证据代替，因此 #9 继续部分完成。本批只编译所需接入测试，未启动原生应用、VoiceOver 或性能测量。两份无占用构建产物已清理，共享缓存、源码、补丁和证据保留在 `design/reviews/gallery-transient-lifecycle-20261019/`。

后续先核对公开 CI 的独立 golden 失败（原 #2），再按外接屏小宿主流程补本修复的原生及 GPU 观察。冻结消费线可以继续，不因 Gallery 后续验收相互等待。
