# #17/#18/#20 输入上下文对照

基于全部最新 WIP，公共候选 `4a02d30bd65c1117f433a874a7c421492bf76ad101e92f7260ecd470323554a1` 的188个文件与当前源码相符。AgentKit主checkout与既有冻结不变。此轮定位基础文本服务／公共文本契约，未证实新的生产缺口，没有增加共享层、状态或token。

隔离实际消费者增加只读输入上下文与事件观察，两个改动文件完整保留于 audit-only.patch；不改变文本分发、编辑和AX发布。外接屏U2790B的活动窗口中，key window、第一响应者、当前inputContext及client归属均一致，输入源为系统SCIM.ITABC。CUA的n/i键码45/34正确、inputContext返回handled，但直接产生TextCommit而没有Preedit。

随后以独立Rust测试例构建系统NSTextView对照，仅覆盖keyDown用于前后观察，实际编辑调用原AppKit superclass；没有加入生产组件和依赖。相同外接屏、输入法、活动窗口和自动按键，也得到`ni`、marked=false。该对照排除了“只因测试窗口不活动、输入源不对或自定义组件独有”这些简单解释；不能据此断言所有工具输入均不支持IME，也不能证明物理键盘组字通过。oracle源码及其额外Cargo.lock单独由该轮inputs.json记录；没有覆盖之前实际消费者316文件的构建输入清单。

公共API和真实接入仍沿用同候选已封存的editor/controlled/viewport回归；本轮没有源码改动，不重复运行整套测试。新增证据是实际消费者输入归属、实际原生事件、系统标准文本对照。Gold7仅增加G5编辑保持、G6事件归属与G7可复现消费者的诊断证据；不声明完整体验、物理IME或VoiceOver通过。合理例外：测试专用原生oracle只是平台对照，生产仍纯Rust和现有共享文本实现。

#17候选确认Enter及组字Tab/Escape、#18物理组字时外部同步、#20物理候选窗和VoiceOver独立进入／逐行导航仍未验收。自动输入不替代物理输入，AX查询不替代听读。公开pin单列交付阻塞，不妨碍继续自动检查。

两个小宿主已关闭、运行时及本轮准确归属的构建产物已清理，公共依赖缓存、源码、日志、截图和可重建脚本保留。此轮未更改输入源、未开启VoiceOver、未测性能、未提交推送或写Issue。详细证据见 `design/reviews/text-input-context-20261019/native-result.json`。
