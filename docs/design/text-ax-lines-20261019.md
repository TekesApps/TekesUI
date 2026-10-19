# #20 文本 AX 行契约与 VoiceOver 现场跟进

基线为 gallery-design-baseline 当前全部 WIP；旧交付候选 `agentkit-final-candidate-20261019` 的636项封存产物不变。新源码和证据在 `design/reviews/text-ax-lines-20261019/`，消费者原冻结 `.deps`、公开 pin 均不改写。

根因分类为公共文本语义/平台桥接契约缺口：已有 AX 值、UTF-16选区和文本片段，不含按真实排版行查询的接口。此前人声确认 VO+A 能读取实际历史，但 VO+L 无效、VO+Down 跳到消息及按钮。缺行查询是已确认代码缺口；不能据此断言它是所有 VoiceOver 交互失败的唯一原因。

共享修复位于 platform-macos 的 `accessibility.rs` 与 `shared.rs`。`AccessibilityTextLayout` 复用已有 `TextInputState` caret 几何，仅增加真实 shaper 行范围；`Window::set_accessibility_text_layouts` 按现有语义 key 发布完整文档布局。原生桥接提供 LineForIndex、RangeForLine、InsertionPointLineNumber 与 FrameForRange。索引为 UTF-16；软换行边界取下行；无几何时不伪造。角色、值与布局匹配才公开查询。发布不改变焦点、选区、IME或编辑真源。

消费方先发布布局，再发布语义树，通知查询可见一致快照。独立 composition consumer 与隔离真实 AgentKit demo 对每个已挂载 TextSurface 发布自己的完整行布局，包含未聚焦只读 Transcript 和 Composer。键盘 IME 继续沿用原来的聚焦状态；不借另一个编辑器的数据，也不按字符串换行估算视觉行。

适用于 TextField、TextArea 及其 Search/Composer/只读阅读组合。StaticText 不自动声明可编辑文本协议。独立宿主和 AgentKit 候选已接入；旧 Gallery 的各文本发布器尚待统一迁移，其他组件族暂停，不能把这两处消费者通过扩为全系统完成。JSON token、控件绘制、Agent业务状态均未变。

最终验证包括平台40项、独立消费者130项（新增真实 CoreText 排版测试）、AgentKit UI/第二消费者27项和 headless76项；严格 UI Clippy、格式、边界、构建和离屏 GPU 分开记录。后续平台测试 Clippy 发现单元素 Range Vec 的测试写法告警，改成显式 push；最终重跑日志/源码指纹见证据目录。全仓 pure-Rust 检查器最初对嵌套消费者快照误用外层 workspace，导致依赖解析失败；修复为最近所属 workspace，新增嵌套公开依赖/私有引用/越界路径回归。最终10项门禁自测和全仓扫描通过，初次失败日志保留。

新小宿主在 U2790B、scale2、Light 启动，原生文本角色、布局、Transcript/Composer/动作可见性已观察。离屏 PNG SHA `819c3c0d9415b7c17eacec9b47825dfd19c897e817c241e6b4a396d48ac05fc5` 与旧封存复现图一致。Gold7 G5/G6 的文本导航体验仍按下述现场结果收口；静态 Gold7 检查不代表体验通过。

人声复验：VO+Shift+Down 似无响应，交互入口仍未通过；点击历史行取得焦点后 VO+L 可读该行，记为有限通过。原生普通 Down Arrow 已由人声确认移动插入点并朗读下一行；VO+Down仍无响应，不能互相替代为通过。原生普通 Down 越过视口底部时会滚动并继续朗读新揭示行，由人声确认通过。Shift+Tab未离开Composer，经代码核对其显式 `TextTabPolicy::Insert` 契约消费Backtab；此前把该快捷键当作反向焦点导航的测试预期不适用，不擅自改写消费者策略。VoiceOver独立进入/下一行、旧Gallery迁移及完整AX矩阵仍待验收。既有会话 B 激活、停止不可用、整段历史朗读是在旧二进制上的有限通过，保留原归属，不挪作新二进制证据。

小宿主流程：旧宿主关闭后原/克隆 wake 均为 Closed；经 fresh lsof 检查后清理旧 runtime。本轮测试可执行文件及重复链接产物清理，公共库缓存复用；新runtime在记录有限通过及未解决项后已关闭并经fresh lsof检查清理；原/克隆wake均Closed，VoiceOver保留用户开启状态。源码、WIP快照、补丁和证据保留。性能未测；无提交、推送、tag、发布、Issue写入或关闭。公开不可变 pin、消费者提升候选仍是独立交付阻塞。

用户随后明确要求结束VoiceOver并自行关闭；进程检查确认VoiceOver已退出，临时Activity Monitor筛选已清空。工具快捷键未形成关闭证据，不计作成功。
