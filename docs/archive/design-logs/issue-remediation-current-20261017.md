# #13 当前整改总览（全部 WIP）

> 2026-10-19 最新交付补充：用户已授权本轮独立提交、推送及正式 pin 消费切换；公开 `431093786de47b48cd566d0d9c0eafa088b10d68` 已经 AgentKit 实际 git+rev 独立复验，#15–#21 的62条原条件满足，七项全部关闭。下文此前“缺公开 pin／禁止本轮提交”的描述属于历史，不再作为当前阻塞。当前记录见 [正式交付与关闭](agentkit-public-pin-20261019.md)。公开 CI 的 Gallery golden 失败继续在 #2；Menu/Popover 新私有修复待原生/GPU/Gold7复验，#9仍开放。

> 2026-10-19 当前入口：[账本、门禁与Gallery生命周期](gallery-lifecycle-gates-20261019.md)。下面为历史增量；其中旧Issue数量、CI提交、候选身份、未测项和状态计数不能作为当前结论。#15–#21最终037开发候选已由AgentKit独立接收，但公开pin仍阻塞关闭。

2026-10-18，本文件是后续增量入口，不覆盖早期盘点或历史收据。最新 GitHub 快照为 14 个开放 Issue、全部状态 0 个 PR；#14 最新正文更新时间仍为 `2026-10-17T08:15:16Z`。当前基线是活跃 `gallery-design-baseline/TekesUI` 的 `bc3de79` 加全部未提交 WIP，而非仅远端旧提交。

逐项原验收条件的五态结果及可恢复工作清单见 `design/remediation/ledger.json`（GitHub 原文快照同目录），共 14 项 Issue、74 条当前检查/审计交付项。#13/#14 没有 checkbox 清单，按正文审计交付分组记录；其他项使用最新正文原文。未变化的条件保留既有源码/证据结论，不冒充本轮重新完成全部体验矩阵。

| Issue | 当前状态 | 主要剩余条件 |
| --- | --- | --- |
| #1 共享强调与边界 | 部分完成 | 既有 JSON 规则和共享绘制 WIP 保留；全组件语义、状态/品牌和整页人工核对未齐 |
| #2 CI 合并测试 | 已实现待验收 | 本地修复与检查保留；远端仍是旧 bc3de79 的失败，未推送不能验证新 CI |
| #3 焦点遍历 | 已实现待验收 | 最小场景已有原生 TextCommand、Tab 和组字保护；全部控件/物理输入矩阵未齐 |
| #4 浮层输入生命周期 | 部分完成 | Palette 既有修复、组合 Dialog scope 和文本 owner 已推进；其余浮层/宿主统一接入未齐 |
| #5 原生可访问性 | 部分完成 | 共享语义、AX 树/动作/文本选区和通知已有最小场景证据；生产 Gallery 全组件导出及 VoiceOver 仍有依赖/未验证项 |
| #6 设计投影完整性 | 已实现待验收 | 本地内容绑定与负例保留；远端 CI 未验证 |
| #7 消费品牌入口 | 已实现待验收 | 显式离线artifact、双外部消费者、四主题/状态及语义隔离、未知配置/品牌/角色拒绝和持续脚本已验证；完整体验与托管CI未完成 |
| #8 能力清单 | 部分完成 | 45项/80条组件绑定、184条类型分类与40→45映射保留；269项状态要求中95项有公共API状态断言，161项夹具待核验；Tabs/Disclosure内容6状态+4组合已有调用方最小消费者和有界token观测，完整依赖与体验仍未齐 |
| #9 浮层动效 | 已实现待验收 | 组合 Rich Dialog 已复用 OverlayPresence；完整原生中断/失活与物理体验未齐 |
| #10 阅读保护 | 已实现待验收 | ReadingController 已公开并复用原锚点/跟随实现，Gallery与组合消费者共用；修复滚轮拉回光标及阅读动作focus裁剪；物理键鼠/VoiceOver仍未验收 |
| #11 草稿恢复 | 已实现待验收 | 公开 DraftJournal 复用既有原子适配；独立文件消费者与原生失败/重试/迟到确认/重启恢复已验证，系统IME与AX仍待验 |
| #12 统一验收 | 部分完成 | 自动、GPU、系统 IME、AX 已分别留证；候选列表视觉/跨屏、VoiceOver、全组件逐页验收仍未齐 |
| #13 总览 | 部分完成 | 依当前 WIP 持续归档和更新，不按旧报告或局部通过自动关单 |
| #14 组合架构审计 | 已实现待验收 | 当前45组件矩阵、94个API源锚点和共享协作/兼容性/Window Toolbar规划已补；修复失败移除提前关闭Dialog。后续迁移和Toolbar未自动实施 |

## 新增证据与依赖顺序

1. 设计真源/CI 的本地门禁继续执行。新编译绑定进入既有生成检查与 workspace tests；不提交、不推送，也不把本地通过称为远端 CI 成功。
2. #14 继续作为组合架构入口。已完成的公共最小协议、异构 List、Rich Dialog、OverlayPresence、文本 owner/IME/AX 增量见下列各批报告；生产组件族不自动扩迁移。
3. 当前 #8 的反向类型分类与状态记录遗漏门禁已补齐；不能把记录完整当成状态实现完整。选择族error契约、TextSurface disabled光标和输入状态夹具已修复；本批补齐输入hover/pressed共享契约、公共状态快照及真实捕获夹具，输入族27项公共API状态已核验。接下来核实184项夹具待核验记录及其余组件族的真实运行时token依赖；本批已推进 #10 公共阅读边界，现已补齐 #11 公共草稿协议与限定原生验收；接下来核实实际系统AX缺口，现已补齐 #7 消费品牌入口；接下来刷新 #14 当前组合矩阵和其余组件族。
4. #5 与 #12 持续保留原生体验边界。当前已观察系统拼音/日文真实转换与提交，已修复组字撤销、转换会话残留、空组字及失焦事务；候选窗视觉和 VoiceOver 仍未通过完整验收。
5. 最后逐组件、逐页面统一验收；性能按用户要求暂缓，新增组件不自动进入范围。

各批实现报告：

- [最小组合协议](composition-minimum-protocol-20261017.md)
- [异构 List / Rich Dialog](composition-list-dialog-20261017.md)
- [Dialog presence 与原生语义桥](composition-presence-20261017.md)
- [文本选区与组字撤销](composition-ime-accessibility-20261017.md)
- [输入会话与候选坐标](composition-input-session-20261017.md)
- [当前公开编译绑定与历史映射](component-registry-public-api-20261017.md)
- [公开类型反向覆盖与漏登记门禁](component-public-surface-20261017.md)
- [必需状态与契约 token 覆盖](component-state-token-coverage-20261017.md)
- [选择控件族 error 整改与运行时 token 观测](selection-family-20261018.md)
- [输入控件族 TextSurface 状态与真实 token 核验](input-family-20261018.md)
- [输入 hover/pressed 共享契约与真实捕获](input-pointer-20261018.md)
- [公开阅读控制器与共享滚轮/焦点修复](reading-controller-20261018.md)

持续任务清单、依赖、分层证据与下一动作以 `design/remediation/ledger.json` 为准；当前目标保持active，单批完成不表示全部Issue或原生验收完成。

原验收条件未满足的继续留在原 Issue。当前没有创建重复 Issue、提交、推送、tag、关单或发布；历史原包、截图、收据与无关 WIP 不被覆盖。

2026-10-18当前全组件组合审计见 [composition-audit-20261018.md](composition-audit-20261018.md)。#14的后续实施规划不等于整套迁移已获本批执行；下一批继续既有动作族状态/token核验。

Action 族本批：公共状态累计72项；共享309、消费者23、能力门禁34项通过。见 [分层证据与剩余项](action-family-20261018.md)。原生/AX 未因此晋级。

导航族当前：17项头部状态与7组合具公共API断言，公共状态累计89项、夹具待核验167项；内容状态6项+4组合未晋级。共享317、消费者26项通过；原生发现并修复Breadcrumb Space宿主兜底误激活。见 [导航族分层证据](navigation-family-20261018.md)。

内容状态补证：共享 active 祖先输入隔离、引用文字服务转发及 Gallery 共享页签路由已修复；公共状态累计95项、夹具待核验161项。独立消费者、GPU、原生呈现与AX边界见 [内容状态分层证据](content-status-20261018.md)。

选择弹层交互前置修复：旧捕获不再选中已经变化/关闭/禁用的候选，CommandPalette 禁用 open 保留查询；原生发现并修复 Escape 误记候选激活。状态数仍95、夹具待核验161，继续逐状态/token核验，不由本轮捕获数推定全族完成。见 [选择契约证据](choice-contract-20261018.md)。

选择弹层几何补证：复用共享放置与真实行测量，修复高锚点、可用空间及空候选幽灵面板；现有 CommandPalette／ComboBox 接入通过大字号窄预览原生核验。共享332、消费者30，576张最终GPU图，192项独立几何观测；公共状态95／夹具待核验161不变。见 [共享几何与分层证据](popup-geometry-20261018.md)。

选择族触发器补证：CommandPalette 补齐公共移动转发与独立焦点移交；Gallery 修复冷启动键盘焦点和拖出释放遗漏。五项触发器状态带真实 token 观测后，公共状态累计100、夹具待核验156；其余选择族25项继续处理。336项控件/token/export测试与31项消费者测试、GPU及原生证据见 [触发器状态与接入](choice-hover-20261018.md)。

Select／ComboBox 状态补证：共享 Select 关闭点击不再重新按下，宿主无需 trigger blur 补救；复用测量和 TextSurface，增加15项精确公共状态、180项真实token观测，累计115／夹具待核验141。原生发现确认键76遗漏并在Gallery统一入口修复，36／76完整路由回归和原生选择通过。见 [选择状态与共享修复](choice-selection-20261018.md)。

2026-10-18 选择关闭焦点：公共 SelectView 与真实 Menu caller 在键盘关闭后恢复已有焦点呈现；鼠标路径仍保持鼠标模式。未增加 token、状态或协议；115 个精确公共状态与剩余10项要求不变。见 [证据](choice-focus-20261018.md)。

2026-10-18 CommandPalette结果：修复ComboBox/CommandPalette公共key便捷入口遗漏重复确认回调；真实消费者和Gallery分别验证，新增CommandPalette selected/open/empty精确状态及测量样例。118项公共状态/138项夹具缺口；选择族剩余7项，完整体验和AX不计完成。见 [证据](command-results-20261018.md)。

2026-10-18 Select可用性：空选项不再打开菜单或遗留按压；公共set_error投影既有验证边界并允许修正。空/验证错误两项公共状态补证，120项公共状态/136项夹具缺口；5项选择profile要求及异步内容失败、完整体验与AX继续保留。见 [证据](select-availability-20261018.md)。

2026-10-18 Select后台刷新：公共触发器/菜单分阶段绘制复用原实现，独立消费者用ContentHost/Stack/Progress/Banner保留缓存值、候选与焦点，失败不等同验证错误。121项公共状态/135项fixture缺口；4项选择profile要求及完整体验/AX继续保留，见 [证据](select-async-20261018.md)。

2026-10-18 ComboBox独立刷新：复用TextFieldContent/ComboBoxView/Status/Feedback；修正共享ContentHost的IME控制键及指针捕获优先级。123项公共状态/133项fixture缺口；2项CommandPalette profile及完整原生异步/动效/AX验收仍开放，见 [证据](combo-async-20261018.md)。

2026-10-18 CommandPalette独立刷新：复用原触发器/候选菜单、TextFieldContent和Status/Feedback，无需新增共享协议；保留IME/查询/选区/候选并恢复触发器焦点。125项公共状态/131项fixture缺口；choice profile公共接入缺口本批补齐，完整原生异步/动效/AX验收及其他组件族继续开放，见 [证据](command-async-20261018.md)。

2026-10-18 选择运行时可用性：Slider/Switch/RadioGroup事件边界观察到禁用后释放旧交互，三项公共红绿复现已修复。Slider六态/216条真实token及Gallery真实拖动样例补齐；131项公共状态/125项fixture缺口，Component range/actions与原生AX等仍开放，见 [证据](range-availability-20261018.md)。

同批原生核对发现Gallery通用移动分发漏接AppKit左键Dragged；已改为五个既有移动入口统一接纳，并以真实Dragged回归取代仅Moved的滑块拖动输入。旧失败现场保留，原生与GPU复验另留证。

2026-10-18 Slider组合契约：原模型/View经SliderContent接入Component；可选范围元数据、原步进增减/设值和稳定ID保留已接入；native数值角色/边界/动作桥编译与快照验证，实际AX/VoiceOver仍未验收。131公共状态/125fixture缺口不变，无新组件，见 [证据](slider-composition-20261018.md)。

## 2026-10-18 · 新需求 #15–#21

最新 21 个开放 Issue 已逐项对照全部 WIP；Slider 组合证据先封存，随后补 #15 WindowWake。#16 复用 ReadingController 补前插锚点，#17–#20 按共享编辑契约推进，#21 随公共动作契约。见 [最新分诊与执行顺序](agentkit-demand-triage-20261018.md)、[WindowWake 公共契约与分层验证](window-wake-20261018.md)。原 #5/#12/#14 的实际数值 AX/VoiceOver 验收仍未完成；无提交/推送/关闭。

## 2026-10-18 · #16 历史前插

在原ReadingController/TextSurface上补只读文档版本与UTF16保留映射；旧WIP204pt偏移已复现，新GPU/原生位置0pt。共享357、consumer50、GPU48及原生后台/两页/尾追加/latest分层留证；可编辑事务/物理IME/VoiceOver/AgentKit联调/正式pin未冒充完成。见 [本轮修复与边界](prepend-reading-20261018.md)。下一批统一#19/#17事件契约，继续#18/#20/#21。

## P0 消费侧收口更新

用户要求先将 #15/#16 推进到可关闭程度，暂停 #19 实施。本轮已接入 AgentKit 最新快照，移除业务 frame pump、映射稳定消息身份，UI 11 项/headless 76 项/Clippy/build/GPU 离屏与保护检查分别记录。实际 AgentKit 原生最终体验、多目标唤醒隔离与正式公开 pin 未完成；不能宣称可关闭。详见 [记录](p0-consumer-closeout-20261018.md)。

## #15/#16 定向补证

生产弱目标表的两个Shared宿主/19代理/32断言通过；不创建窗口，现场多窗口仍未验收。AgentKit实际第二页叠加400pt/150%字号后锚点误差0pt，焦点/选区与A→B→A保留；UI11项/strict Clippy再次通过。见 [目标与重排记录](wake-targets-20261018.md)。原生授权和公开pin阻塞保留，继续可执行的#19公共事件传递链。

## #19 指针与共享选择修复

完成公共元数据入口、兼容默认策略、三个宿主及AgentText转交；原生发现emoji误选前字并在共享TextInput/TextSurface修复，同时统一段落及CoreText硬换行。共享359、独立57、AgentKit13/headless76、4500 token查找、GPU16图及Gallery原生前后对照分别留证。多行/失焦拖动原生组合、AgentKit原生、正式pin和IME/VoiceOver仍未验收。见 [本批记录](pointer-metadata-20261018.md)。后续#17/#18/#20/#21持续推进，不关闭Issue。
