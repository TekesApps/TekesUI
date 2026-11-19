# #14 / #5 最小场景第四批：文本辅助契约与组字撤销保护

2026-10-17，接续 [Rich Dialog 过渡批次](composition-presence-20261017.md)。继续使用活跃 checkout 的 `bc3de79466834e99062a1b4ef0872323e2860001` 加全部未提交 WIP。未迁移新组件、未改变 JSON 设计真源、未测试性能、未提交或修改远端 Issue。组件保持 experimental。源身份、测试、原生记录与 WIP 保护见 `design/reviews/composition-ime-accessibility-20261017/receipt.json`。

## 根因、共享位置及适用范围

| 根因归属 | 共享修复与复用 | 适用组件及例外 |
| --- | --- | --- |
| 公共组件契约：原生文本值存在，但辅助技术没有选区查询/设置和选中内容替换回路 | 复用 `TextInput` 的 UTF-16、字素边界、insert 与 undo。新增 `select_range` 表达范围选择，返回是否接受请求；`ContentHost::text_surface(id)` 只暴露当前 active/modal scope 内的真实编辑状态；现有 AccessibilityAction 增加 SetSelection / ReplaceSelectedText | 所有 TextFieldContent，包括异构 List 与 Rich Dialog。只读允许选择，禁止改值；disabled、关闭节点、背景 modal 节点拒绝修改。marked composition 保持输入法独占，AX 不能改值、选区或替换。没有另造编辑状态机 |
| 平台适配：AX 节点缺少 UTF-16 选区与只读信息 | 现有 AccessibilityNode 增加可选 text_selection 和 read_only；同一原生 NSAccessibilityElement 提供字符数、所选文本、选区、范围字符串及 setter。无效/溢出/代理对中间位置被拒绝，不返回替换字符冒充正确文本 | 现有公开平台桥与消费者映射；非文本节点不发布编辑选区。单范围普通输入是本批边界，富文本 TextMarker、AttributedString、完整多行文本辅助协议未实现，不因本批推定支持 |
| 平台适配：所有变化都被通知为 layout，缺少文本/焦点通知 | 同一 Window snapshot 更新路径区分结构、焦点、值和选区变化；向真实原生 owner 发布对应 AppKit 通知，不为单纯选区变化发布 layout；发布前释放 RefCell 借用，避免回调重入持有可变借用 | 所有已接原生快照的组件；没有为场景另建通知总线。通知实现和原生调用回归通过，不等于 VoiceOver 已正确接收并朗读 |
| 基础交互原语：IME 组字期间 undo 取消组字后仍继续 pop 历史，导致草稿多退一步；redo 也能跳过组字直接应用历史 | 修复共享 `TextInput::undo/redo`：有 composition 时本次操作只取消组字并恢复进入组字前的文本与选区；下一次才操作已有历史栈 | 所有使用 TextInput 的编辑表面，包含本批两个消费者输入。只读/禁用继续拒绝撤销。此根因来自真实系统拼音 Preedit 的原生观察，继续归原 IME/输入保护验收，不新建重复 Issue |

公开实验 API 增加 enum variants 及平台快照字段，穷举匹配与结构体字面量消费者需要同步适配；本批已更新独立消费者及现有工作区调用。未完成全库消费者或发布兼容性批准。

## 公共 API 与真实接入验证

- TextInput 单测覆盖代理对与组合附加符号、只读选区、无效范围、disabled、组字优先，以及组字取消与下一笔 undo/redo 的分离。
- 独立消费者通过真实 Rich Dialog 的 TextFieldContent 验证选择 `😀é`、替换、撤销、只读选择、组字期间 AX 请求被拒绝、TextCommand undo 只取消组字、关闭后编辑状态不可查询。
- 平台快照单测拒绝半个代理对、位于代理对内部的空选区、越界范围和非文本 owner 选区。
- 原生 `ax-text` 脚本调用实际 NSAccessibility selectors，检查值、UTF-16 字符数、选区、所选字符串和范围查询；随后检查替换、撤销、组字期间拒绝修改、undo 取消组字，以及关闭移除 owner。6 项快照检查通过。原有 NSTextInputClient 范围脚本 8 项回归通过，仍属于合成回调实验。
- 外部系统 AX 调用在独立原生 List 窗口中成功选择中文、emoji 和组合附加符号；可见选区、焦点与原生日志分别留证。依赖 AX 写值的后续选择需先刷新快照；同一批工具调用中立即查询旧值可能失败，刷新后验证通过。

最终工作区 release 测试 891 passed、0 failed、1 原有 ignored；独立消费者 11 passed。Clippy、fmt、DS、Gold7 mechanical、reference alignment、纯 Rust 与能力清单检查通过，具体日志与身份见收据。工作区继续显式过滤两项 CPU/GPU 计时测试，原 ignored 不算通过。真实 GPU 重绘回归仍覆盖 4 主题 × @1x/@2x × 10 过渡状态，共 80 帧；本批没有新增局部视觉样式。

## 系统 IME 与 VoiceOver 实验边界

系统已启用 ABC、拼音和日文输入源。测试前选中拼音；尝试切换后恢复拼音。首次自动输入没有足够证据证明候选窗参与，保留尝试记录。后续独立窗口的实际按键进入系统拼音，日志记录了 `Preedit`，并复现上述 undo 草稿丢失：`A😀é中文` 选区上的 `x` 组字被撤销时，错误退回最初的 `Tekes`。修复前记录未覆盖；修复后另行采集。

修复后，同一原生窗口路径再次产生系统 `Preedit("x")`；⌘Z 仅取消组字，恢复 `A😀é中文` 及 `1..5` 的原选区，而不再回退到 `Tekes`。修复前后原生日志、AX 状态与截图分别保存，明确验证此根因的实际体验。

这证明了系统输入法到当前组件的真实 Preedit 路径，仍不能代替物理键盘、中日文转换/选候选、跨屏候选窗定位的完整矩阵。原生日志还出现 IMKCFRunLoopWakeUpReliable 的 mach port 警告，候选会话及环境影响尚未查明，不能据此宣称候选窗通过。通过自动化直接输入中文没有产生可核对 Commit 时，不按提交成功计。

本批实际启动 VoiceOver 并通过首次欢迎界面的 Use VoiceOver；自动化导航命令未产生可核对的朗读/导航证据，不能判断完整辅助体验正确。VoiceOver 已通过系统设置恢复关闭，并保留仅含开关状态的记录；未启用 AppleScript 控制权限或改变安全设置。AX 树/选区通过与 VoiceOver 验收继续分开。

## Gold7 和原验收项状态

| 项目 | 公共契约 / 真实接入 | 最终体验状态 |
| --- | --- | --- |
| G1–G3 | 沿用既有 JSON 字体、颜色、焦点、表面和边界；无视觉 token 变化 | 原生选区与 GPU 样图抽查；不扩大整页/品牌验收 |
| G4 | 既有 OverlayPresence 和 Reduced Motion 回归 | 不重报物理动效或性能通过 |
| G5 / #14 IME | 共享组字撤销根因已修复，API、真实系统 Preedit 观察及 AppKit 回调分开留证 | 该缺陷回归与完整 IME 矩阵分开；物理候选窗、中日文完整转换仍未验证 |
| G6 / #5 可访问性 | 选区、选中内容替换、只读及对应通知已实现；原生 AX 操作通过 | VoiceOver 朗读、导航、文本编辑完整矩阵未验证；富文本辅助协议仍缺失 |
| G7 / #12 / #14 | 独立消费者、真实生产 TextInput、平台桥及 GPU fixture 回归 | 最小场景继续已实现待完整验收，不能晋级全库 registry |

下一步仍是原最小场景的候选窗和 VoiceOver 验收；根据具体可重现失败补现有共享能力，再决定迁移范围。未满足的原 Issue 验收条件保持开放，不以这些局部证据关闭。
