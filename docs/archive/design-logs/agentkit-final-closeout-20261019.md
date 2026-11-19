# AgentKit #15–#21 最终候选收口

> 当前剩余项请以 [最终候选验收与独立交付](agentkit-final-acceptance-20261019.md) 为准。下文为逐批历史，保留原版本的失败/通过边界。

2026-10-19。其他组件族整改暂停。以最新 Issue 正文、全部未提交源码和实际消费者为准；不提交、推送、发布、关闭 Issue，不测性能。

## 责任与验收顺序

公共实现及本轮消费者适配由本会话单一负责；AgentKit 交付验收由其消费会话负责。公开不可变 pin 需要后续明确的提交/发布授权，单列为交付阻塞，不阻止本地收口。

| Issue | 当前缺口 / 本轮责任 | 技术通过所需证据 |
| --- | --- | --- |
| #15 P0 | 已有 WindowWake 与 controller watch，不重复实现。核对最新平台生命周期与宿主适配；本会话补最终源码复跑，消费者会话接收结果 | 公共并发/关闭/重入测试；实际流式、取消、连接通知；无业务帧泵的事件记录；关闭后 clone Closed；多目标隔离现场证据 |
| #16 P0 | 已有稳定消息映射与 ReadingController。复验新候选的真实 Earlier，不新增像素补偿 | 连续两次前插、尾部同批、重排、A/B、删除/过期版本、选区与焦点的真实布局测试；原生分页锚点偏差不超过 1pt |
| #17 | 复用既有 TextKey/IME disposition；完成最终消费者和物理输入验收 | 单键一次归属、换行/提交/Tab 配置自动化；真实中文拼音 Enter/Tab/Escape，不能以合成组字代替 |
| #18 | 复用外部值事务；确认最终消费者仍按版本与目的地同步 | 同值 ACK、清空 undo、组字延迟/冲突、A→B→A、UTF-16、过期请求测试；原生组字与外部更新交错 |
| #19 | 复用完整 PointerEvent 与 TextSurface；核对宿主失焦/捕获取消 | 公开翻译测试、真实组件选择；原生 Shift、双/三击、拖选出界和按住时失焦独立留证 |
| #20 | 复用 TextViewport/FieldPresentation。宿主多行 AX 角色未跟随后续共享能力，需实际修正 | 真实 Transcript Fill / Composer Grow、裁切/候选窗/选区坐标、重排保留状态；多行 AX 和只读语义；原生布局及 Gold7 |
| #21 | 复用动作队列及产生时 Intent。宿主需采用活动窗口约束 | 指针/键盘/AX 单次、禁用拒绝、有序 drain、重挂不重放、A/B 目的地固定；失焦窗口不执行旧 AX 请求 |

顺序：#15/#16 → #19 与 #17 事件链 → #18/#20 编辑与视口 → #21 最终动作闭环。已有 API 和测试先复用，只有失败或未覆盖的验收点才修改。

## 版本身份与边界

原 AgentKit 冻结候选 `e4c6cb6f725ff9df5ec4688927dd265d64dbbdb2380572d963c5b1017dde5bbc` 的 188 个文件逐一验证完整；当前 WIP 其中 10 个文件已变化。差异保存在 `design/reviews/agentkit-final-candidate-20261019/candidate-diff.json`，涉及公共组件、平台活动窗口与可访问性，不应直接沿用旧通过结果。

本轮在独立证据目录创建显式消费者副本和完整公共源码候选，原 AgentKit `ui/.deps` 与 manifest 保留。初始复验候选为 `d879400dd5fe0bea847ff39f69f7be18f522889ce3b5b38796dc0299288c2399`。消费者自身后续修复另存增量与最终输入哈希，不把公共源码哈希误作消费者版本。

公共 API、真实消费者、GPU、原生、Gold7、物理 IME、VoiceOver 分开判断。本轮用户已明确授权操作 Gallery 和 AgentKit demo，均在外接屏验收。系统设置另行确认。性能不测，不构成此次功能整改的测试失败。

构建共用 `target/review-cache`，禁止新建每组件 target。只保留源码、日志、截图和哈希；验收后经活跃句柄检查清理本轮二进制/应用，公共缓存继续复用。

## 本轮实际修复及验证

最终候选先出现真实编译失败：AgentKit 宿主穷尽匹配旧 `AccessibilityAction`，未处理当前 WIP 的 `SetExpanded`。根因属于公共能力演进后的消费者适配，现有公共契约足够，不新增抽象。候选宿主显式拒绝未挂载 Tree 的 disclosure 请求，避免错作 Press。同时复用 `Window::is_active()` 限定 AX 动作/焦点，失焦沿原 Cancel 释放交互；按实际 multiline 输入模型发布 TextArea；复用既有 Auto 放置到外接屏。

适用当前 AgentKit demo 的 Transcript、Composer、全部动作按钮。没有 Tree 的宿主拒绝 SetExpanded 是合理例外；业务状态、提交身份、草稿、阅读算法和 JSON token 均不重写。本次修复只位于隔离交付候选的 `ui/demo/src/native.rs`，不改写消费者持有的原 `.deps` 或 manifest。最终补丁 `agentkit-delivery-host.patch` 可对原 WIP 应用；`consumer-source` 是完整、可独立构建的真实消费者，不是 API 模拟器。提升消费侧冻结候选由 AgentKit 会话负责。

最终公共 API 352、独立公共 consumer 129、AgentKit UI workspace/第二 consumer 27、默认 headless 76 均通过。最终 UI strict Clippy、构建及两项 Rust 所有权门禁通过。初次未处理 SetExpanded 的失败、系统 Python 缺少 tomllib 的执行失败及后续修正日志均保留。原测试计数没有改写成新候选结果。

U2790B（builtin=false，scale2，Light）上的最终二进制：

- 流式、运行中取消与 Reconnecting→Connected 主动刷新；29.48 秒稳定空闲阶段 Wake、发布、重绘及动画帧增量均为 0，animating=false。这是事件正确性核验，不是性能测试。
- 两次真实 Earlier 后同一可见文本的 Y 坐标均为72pt，偏差0pt；CoreText 消费回归另覆盖600→400宽度、1→1.5字号、焦点/选区以及A/B，均通过。
- 回车只插入一处换行、不发送；撤销、A→B→A草稿恢复、混合字素、24行Grow上限/内部滚动及发送可见分别留证。
- 真实 Shift-click 后替换得到 `Xbeta gamma`，双击后得到 `alpha BETA gamma`，三击后得到 `PARAGRAPH`；没有用 AX 设置选区冒充物理指针。
- 最小化时发布焦点受 active 约束，AX 写入未改变草稿。关闭后原 WindowWake 与克隆均为 Closed。内部树的焦点字段与发布的活动 AX 焦点分开判断，不能用原始内部字段声称窗口仍有活动焦点。
- 最新源码上的生产目标表审计32断言通过（两个真实 Shared、19代理、撤销/Drop/延迟回调）；它仍是带明确测试入口的审计副本，不是两个活 NSWindow 的现场证明。

Gold7：复用原 JSON DS，实际 CoreText/Metal 初态、输入反馈、焦点、阅读连续性、Grow与动作可见性已核对。仅当前Light/外接scale2代表场景；不声称全主题、全部原生输入、VoiceOver或完整Gold7通过。坐标激活前的误输入/空发送尝试单独保存，不计为有效发送验收；最终截图内残留旧错误提示也不能解释成全体验通过。

## 尚未达到关闭条件的部分

完整逐项记录在 `design/reviews/agentkit-final-candidate-20261019/acceptance-matrix.json`，含最新正文的62条功能/交付验收行、负责人、证据与剩余项。

#15/#16 的主要功能与真实消费者路径已补证；公开 pin、消费方提升候选及最终验收确认仍未完成。活多窗口现场组合保持未测，不将登记表审计等同活窗口体验。

#17 的真实系统拼音候选 Enter/Tab/Escape 仍无可接受证据。本轮用户已授权临时输入源切换与 VoiceOver，工具尝试没有形成可确认的候选组字链；后续回读确实发现输入源曾切为SCIM，不能声称从未切换。VoiceOver 快捷键未启用服务，原生入口超时，服务仍未运行。此处是工具/现场验收缺口，不再请求同一授权，也不以合成事件替代。

#19 仍缺最终消费者中混合字素拖选出界/自动滚动与按住时失焦现场组合；#20 仍缺原生字号/宽度/候选窗/完整AX矩阵。#18 的外部事务和 #21 的动作来源/目的地自动化通过，但相应系统IME与完整VoiceOver体验仍分开保留。以上由本会话继续准备可复现场景，消费方负责交付版现场确认。原 Issue 均不关闭。

### 系统拼音补查与最终产物

为排除切换尚未就绪，在同一最终二进制（SHA `bdcc2bd967615f7324a23d02709f4578ec31a691492b2fd2911be0c054b62ba1`）上，将切换、状态回读和输入拆为独立操作。SCIM.ITABC 已在输入前确认，但工具按键仍直接提交 `ni`，Enter插入换行，没有观察到有效候选/marked-text链。根因尚不能归于工具或平台任何一方，需硬件键盘交叉核验；#17不能按“只有交付阻塞”收口。此证据在 `native-ime-stable/`，输入源再次恢复ABC。VoiceOver仍未启用、未验收。

新增复现入口 `design/reviews/agentkit-final-candidate-20261019/reproduce.py` 已实际执行8项检查全部成功，并自动清理76个精确产物；最终离屏图与交付二进制原输出SHA相同。受保护WIP核对：TekesUI旧基线24774文件中24773字节不变，仅本轮账本更新；AgentKit全部128文件不变。三段增量明确区分此前公共WIP、本轮宿主适配和显式开发manifest；316文件独立源码归档约0.98MB。提交、推送、tag、发布和Issue写入均未发生。

## 后续现场 VoiceOver 与文本行契约（不改写上述封存批次）

用户后续已在测试应用实际听到消息/会话A/发送，确认会话B激活、停止不可用且无动作、VO+A读取实际历史。这些是旧最终二进制上的有限人声验收，不能据此将整个 #20 标为完成。VO+L和下一行导航另有失败，推动共享平台补齐真实排版行查询。

新实现及独立消费者/真实 AgentKit 候选见 [文本 AX 行跟进](text-ax-lines-20261019.md)。旧候选636项封存内容未变，原 AgentKit 冻结快照未提升。新宿主已启动在外接屏；点击历史行后VO+L通过，VO+Shift+Down入口无响应仍待定位。新旧源码、二进制及分层证据分别记录，公开pin继续单列阻塞。

本轮新宿主人声补证：普通Down逐行朗读及越界自动滚动通过；VO+Down无响应，VoiceOver-only路径仍待定位。Shift+Tab被当前Composer显式Insert策略消费，不能据此判定共享键路由缺陷，也不把它记作焦点遍历通过。
