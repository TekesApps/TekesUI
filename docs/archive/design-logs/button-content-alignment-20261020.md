# 普通组合按钮复用 Design System 默认对齐

本轮基于当前全部 WIP。没有创建 Toggle 或其他组件，没有扩大内容槽迁移；没有提交、推送、测性能或写 GitHub Issue。

根因属于公共组件的呈现契约：ButtonTextAlign::default() 已投影 JSON Design System 的 BUTTON_CONTENT.text_alignment，但 ButtonContent.paint 仍传 Leading，绕过该定义。修改共享 composition.rs 的这一处调用，全部普通 ButtonContent 消费者自动复用默认居中。不新增 token、状态或层。

合理例外：CompoundButton 的装饰子树由消费者和容器组织，不强迫复合子内容居中；TextContent、Link 等保持自身正文/导航语义；直接调用 Button.render 并显式指定对齐的消费者保留覆盖能力。当前 Registry 没有独立 Toggle，本轮不新建。

公共 API：新增2项测试，144个已有状态/主题/品牌/宽度组合及正文左对齐例外。先得到 Leading != Center 的真实失败，再通过修复。早先夹具把 Theme 当 ContentTheme 的编译失败另存，不算红绿证据。控件包343项通过。

真实消费者：现有 composition-consumer 用真实 CoreText 度量，在72个普通/禁用/加载、四主题/三品牌/两宽度场景中检查默认对齐、内容槽中心、可访问名称和单次 Space 动作/不可用隔离。该 fixture 的文字 draw 只记录调用，不冒充 GPU。消费者34项通过，两个包的严格 Clippy 通过。

真实 GPU：独立 Button 小宿主复用原有文字后端与平台适配，288个场景共864帧。实际 ContentHost/ButtonContent 与直接公共 Button 的 DS 默认对齐逐像素一致；均能区别显式 Leading 参考。四主题、三品牌、280pt/1.5和640pt/1.0配对、@1x/@2x、六状态；576张PNG及哈希保留。显式 Leading 是复现旧调用方式的参考，不是另编旧源码快照。抽查浅色宽按钮前后和深色IC/品牌2/窄大字禁用图。Reduced Motion 为true，未测性能。

外接屏原生：U2790B@2x、640pt、Light/默认品牌，实际系统 Reduced Motion=false。事件29为鼠标抬起激活，60为Space抬起，63为Enter按下，65为AX Press，100为恢复可用后的新Space；各计数+1。禁用和加载状态各11个活动窗口快照均维持计数4、无焦点/按下，Tab、Space、Enter和真实鼠标下/抬均不激活。verify-native.py 可仅从原生事件及绘制后状态复核。没有把该结果当作物理键盘、按住取消时序、VoiceOver或完整动效/原生品牌矩阵通过。

Gold7：本轮有界核对共享默认规则、必要焦点、不可用状态、真实动作和可访问名称。检查了活动窗口居中和最终焦点外观。九组token探针重新执行4416个案例，complete=false；42项能力治理、8项组合审计及45组件矩阵通过。没有晋级整族或整页成熟度。

清理：28个精确且无占用的测试可执行文件已清理；小宿主自身包产物在签名复制后清理；本轮窗口关闭、PID51848消失、签名哈希核对及新鲜无占用检查后移除复制的App。公共缓存最新依赖保留；全部源码、576张GPU图、原生日志和清单保留。560项既有受保护输入不变，已有文件仅 composition.rs 和消费者 scenarios.rs 有意修改，另新增公共回归测试。

证据入口：design/reviews/button-content-alignment-20261020/acceptance.json；GPU/原生：design/reviews/button-content-native-20261020/acceptance.json。文本族必填/可选 VoiceOver 仍按用户要求延期，未重新开启。批次5及其余批次仍未完成；旧 AgentKit 冻结包及公开候选不改。
