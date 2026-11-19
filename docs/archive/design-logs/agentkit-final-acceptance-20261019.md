# AgentKit 最终候选验收与独立交付

2026-10-19。本记录取代旧批次中“当前尚缺”的判断，旧失败与封存证据不改写。#15–#21 最新正文均仍 OPEN；62 条功能/交付验收逐条记录在 `design/reviews/agentkit-final-acceptance-20261019/acceptance-matrix.json`。其他组件族暂停。当前TekesUI无开放PR；正文关联的AgentKit PR #6已合并，其合并不等于接收本次037候选，仍需消费会话明确提升。

最终开发候选为 `03721866a362b1d31261ebbbc86140eb07e9e23320871c325234a5212591213d`，188 个公共源码输入与全部最新 WIP 一致。整体状态为 **已实现待验收**：本地技术验收已具备，AgentKit 会话正式接收/提升候选与公开不可变 pin 仍未完成。没有发布、提交、推送、关闭 Issue；性能未测。

| Issue | 技术验收及证据 | 还缺什么 / 谁负责 / 通过条件 |
| --- | --- | --- |
| #15 P0 | 既有 WindowWake 的并发合并/竞态/关闭公共测试；无持续帧泵消费者；活双 NSWindow 定向投递/关闭与静止事件计数；最终037原生发送、流式、取消、重连和关闭代理再次通过 | AgentKit会话以本包独立构建并确认提升；公开pin待授权。平台及通知链与活双窗已验版字节一致，未声称本轮重跑双窗 |
| #16 P0 | 独立公开消费者和真实 CoreText 前插、混合追加、宽度/字号重排、删除/过期版本、选区与焦点测试；最终037实际连续 Earlier 两页，同一可见80 UTF-16单元，Y误差均0pt；A→B→A与回到最新通过 | 同上；历史加载数据通过不单独当作锚点通过，本轮另有布局坐标证据 |
| #17 | 公开 disposition/修饰键/repeat/多行/提交和Tab策略，真实接入；物理拼音Enter/Space/Tab/Escape已单独确认；最终037文本服务端到端通过 | 同上；物理键结果保留其原候选身份。037仅改TextSurface几何，键/IME路由字节相同，不把旧人声/物理记录改名为新测试 |
| #18 | 公开事务、选区/undo、UTF-16、A/B隔离、过期拒绝；最终037物理组字期间同步/缩放剪裁后确认，你已确认无中断、无REMOTE覆盖，日志公共/真实消费者冲突均正确 | 同上；不新增同步队列或Agent状态机 |
| #19 | 元数据翻译、公共/真实选择测试；原生Shift、双/三击、出界拖选、次按钮、按住失焦/取消分别已有证据，当前独立消费者完整复跑 | 同上；按住失焦是工具驱动原生链，系统pressedMouseButtons=0，未宣称硬件鼠标按住体验；相关平台/组合实现未变 |
| #20 | Fill/Grow、单行默认、真实组件缩放/裁剪/重排状态；最终037物理候选位置通过；本轮真实AppKit命名剪贴板逐字复制两次，并拒绝commit/paste/delete，Light/Dark切换/resize保留；最终037 VoiceOver独立进入/当前行/下一行三步均由用户确认 | 同上；命名测试剪贴板不等于跨应用通用剪贴板；Dark事件通过不等于全主题视觉矩阵通过 |
| #21 | 公共动作ID/source、有序drain、禁用、重挂/结构变更、目的地产生时捕获；真实消费者无计数轮询，最终端到端发送/取消/会话切换通过；既有原生pointer/key/AX结果按原版保留 | 同上；业务destination/scope仍由AgentKit负责 |

62行中48行已完成、7行已实现待AgentKit验收、7行被公开pin依赖阻塞。“已完成”限定对应验收行，不代表原Issue已经可以在GitHub关闭。没有未实现项被藏在pin阻塞后。AgentKit验收应校验包哈希、运行本文所列三套检查、确认当前源提升与实际demo；后续获得发布授权，再将正式pin按相同输入复验。

## 本轮实际补齐

没有新增公共抽象，也没有重新实现已有能力。共享根因复核沿用现有 `TextSurface`、`ReadingController`、composition键/动作/指针契约、AppKit文本桥接；此前几何缺口的公共修复已在037中。本轮补齐的是交付环境和未完成证据。

独立解压发现本机默认Cargo1.83，原宿主目录隐式继承1.95；交付包现在显式锁定Rust1.95.0。两个公开消费者测试曾引用Gallery薄转译文件，包内迁至tests/support，字节相同，仅改include路径；现有icons依赖作为测试支持带入，不扩组件整改范围。没有依赖工作区外的兄弟checkout。

原生端到端第一轮在没有编辑焦点时提前投递preedit，失败日志原样保留。审计副本改为实际窗口active且原生指针已聚焦Composer后一次性启动，复用既有公共事件、焦点和脚本，之后通过。此变更只在audit host，不进入交付生产代码；其完整补丁含新增native_qa.rs，消费者版本和二进制哈希分别留档。

只读复制复用平台现有TEKES_SELFTEST命名AppKit pasteboard，未读写用户通用剪贴板；真正通过宿主copy路径检查完整UTF-16文本。适用readonly Transcript；Composer正常编辑能力继续由原有测试覆盖，是合理例外。Final VO审计现代AppKit owners3/4自动查询先通过，人声结果另存，不能相互替代。

## 可直接交给AgentKit的包

- 源码：`design/reviews/agentkit-final-acceptance-20261019/agentkit-tekesui-0372186.tar.gz`
- 包SHA256：`a220dfca80748f0b7d82f115697b59a156186852f5c433891e05cd7c65eb24d0`；旁置`.sha256`文件。
- 解压后进入`handoff/`，按README先`python3 verify.py`，再分别跑UI workspace、第二公开API consumer、默认headless测试与demo build。原生用README的cargo run入口，默认优先外接屏；共享CARGO_TARGET_DIR，验收后只清理自己不用的产物。
- 545个payload文件和188公共文件校验通过；包外真实解压到/private/tmp后复跑，UI28、独立consumer131、headless76、build及Metal GPU输出均通过，三套Cargo metadata确认所有本地依赖都在解压包内。新机器允许正常获取registry crates，并非离线供应所有第三方依赖。
- GPU PNG SHA `819c3c0d9415b7c17eacec9b47825dfd19c897e817c241e6b4a396d48ac05fc5` 与基线一致。真实GPU、公共API、实际组件、原生输入、物理IME及VoiceOver证据分开保存。
- `replay-results.json`、三份metadata、`acceptance-matrix.json`、`prior-native-provenance.json`、原生round receipts、`voiceover-result.json`、`native-anchor-result.json`与证据归档供独立复核。源码包不含audit instrumentation。

相对AgentKit primary，既有消费适配涉及native.rs、text_viewport.rs及显式tekesui-source清单/对应188文件冻结依赖。原始128文件与消费者自持冻结快照保持不变。包内是可构建的开发候选，不替消费者做正式pin提升，不混入其他组件族WIP。

Gold7按已有共享规则核对G5阅读连续/只读复制、G6输入组字/几何/无障碍、G7源码和复现证据。当前Light外接2×与公开1×/2×布局测试范围明确；没有宣称全主题/全部组件/完整Gold7矩阵。JSON Design System未新增token，纯Rust，未引入Swift/GPUI。

VoiceOver已正常Quit并确认进程不存在，三个本轮小宿主及失败轮次运行时已关闭并清理。133项精确归属构建产物（逻辑约347MB）经fresh lsof检查删除；公共依赖缓存保留。工作区外临时解压目录已清理，归档、源代码、WIP快照与验收证据保留；最终保护检查另见completion-check.json。
