# Slider 原生动作与宿主失活输入整改

根因分类：公共宿主的输入作用域接线。共享组件已有值、禁用、挂载和语义契约；公共 native 示例在 Focus(false) 时取消当前交互，却未拒绝迟到输入。真实原生重放显示窗口仍失活时，Tab 重新建立焦点，Right 把值从 5 改为 10。该失败证据完整保留在 attempt1-inactive-input，未计为通过。

实际修复位于 examples/composition-consumer/examples/native.rs 的公共 WindowHandler，复用 Window.is_active、ContentHost Cancel、清空焦点和发布语义后提前返回，拒绝失活键鼠、滚动、文本服务、AX 与命令回调。由同一处理器承载的 List、Rich Dialog、Slider、Command、Reading、Plain、NumberField、Field 均适用；未新增 token、状态机或抽象层。窗口生命周期、重绘、系统外观／动态设置、关闭仍照常处理。当前原生复验是 Slider 与真实相邻 TextField，不能扩写为八个场景均原生通过。

测试宿主通过既有 objc2/AppKit 绑定访问自身已发布的 NSAccessibility 对象，调用真实 Increment/Decrement selector，无需修改生产桥接，也没有引入其他 UI 框架。最终同一签名宿主的十次 selector 调用验证正常增减、禁用拒绝、移除后旧对象拒绝、重挂载后旧对象仍拒绝及新对象正常工作。另八个真实重新激活检查点验证旧焦点不会自动复活、新 Tab 后步进、AX 增减、相邻编辑器方向键隔离，以及一次 selector Shift+Tab 返回。

最小化后、恢复可见但系统仍报告失活时，真实收到的 Tab、Right、AX 写入均维持空焦点和值 60。用户通过系统 Command+Tab 激活后，原生 Focus(true)、活动状态和后续事件逐项核实通过；工具 Raise 和窗口菜单未建立活动状态的过程未计为恢复通过。

公共消费者 149 项测试、native 示例及最终宿主严格 Clippy、42 项治理和 8 项组合审计测试及 45 组件门禁通过。生产绘制／布局／token 未变化，复用上一轮 432 组、864 帧 GPU 证据，未声称本轮重跑 GPU。Gold7 范围仅为当前的焦点、禁用、旧对象和恢复体验。VoiceOver 保持关闭；未测性能，不提升整组件成熟度。

验收后关闭窗口并核实进程退出／未使用，再清理本轮运行目录、精确测试可执行文件与宿主包，保留代码、日志、图像和有界源码快照。未提交、推送或关闭 Issue。尚余延期听读、其他组件族及同一最终版本的整页验收。

证据入口：design/reviews/slider-native-actions-20261020/acceptance.json、native/verified-result.json、native/evidence-manifest.json、protected-before.json、protection.json、cleanup.json、final-verification.json。源码快照是有界审计输入，不冒充完整可发布闭包。
