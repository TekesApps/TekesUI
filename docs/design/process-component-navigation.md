# ProcessComponent、无 View 导航模块与 nav.json

状态：2026-10-21 确认的设计契约，供 TekesUI 公共抽取与 AppHost 接入使用；本文件不表示当前 V1 runtime 已支持无 View 消息或持久化事务。既有原生进程、IPC、共享纹理实现继续复用，不创建第二套运行器。

实现进展：基于公共 V2 ProcessComponent 的服务侧最小导航原型、窗口 toolbar 注册和 nav.json 故障验收已完成，见 [原型实现与证据](navigation-prototype-20261021.md)。该进展不改变下文完整窗口/联合发布的验收要求。

## 1. 基础抽象与装配

- **ProcessComponent**：固定装配的独立模块进程，拥有状态、生命周期、请求/事件及可选 toolbar 贡献。View 是显式可选能力，toolbar 不依赖 View 或 frame-ready。
- **带 View 的 ProcessComponent**：在模块进程内使用 TekesUI 组件布局/绘制；复用现有共享纹理、帧代次、背压和输入契约。ProcessContainer 是承载这类模块的呈现/输入适配，不是另一套进程管理器。
- **无 View 的 ProcessComponent**：不分配容器、不伪造 Panel/View ID、不创建 renderer、GPU device、Scene、glyph atlas 或共享纹理。不用隐藏容器、透明帧或零尺寸 View 模拟。握手/服务就绪、toolbar 注册、命令和事件必须能够独立于渲染循环运行。
- **导航模块**：独立、无 View 的 ProcessComponent，维护导航历史、游标、事务和 nav.json；注册前进/后退的纯图标 toolbar 贡献及可用状态。
- **AppHost**：装配三个带 View 的 ProcessContainer 和一个无 View 的导航 ProcessComponent；负责窗口/布局、执行已声明 route 的导航并返回执行结果。它不维护第二份历史或直接写 nav.json。

三列 View 基线仍是 1 Host + 3 View 模块进程。包含导航的完整装配是 1 Host + 3 View 模块进程 + 1 无 View 导航进程，共 5 个应用进程。新增导航进程不能拿其中一列充当，也不增加第四个内容容器。

能力协商应区分服务消息、toolbar 与 View/渲染能力。现有 PanelModule / Request/Frame V1 是带 View 原型接口；应兼容适配并显式扩展/版本协商，不能声称它天然支持无 View 服务。本文消息名为语义名称，最终 Rust/API 名称由公共实现确定。

## 2. 所有权与 toolbar

| 所有者 | 权威职责 |
|---|---|
| 导航模块 | 历史 entries、cursor、revision、请求排队/事务、持久化、canGoBack/canGoForward 投影 |
| AppHost / target app 路由适配器 | 将 route + params 解析为已装配目标，执行切换，维护实际窗口/内容上下文，回报成功/失败/取消 |
| 各内容模块 | 自己的 content、业务状态及 Panel toolbar 声明 |
| TekesUI | 唯一的进程/IPC/资源生命周期机制；Toolbar 的图标/单行测量、溢出和交互 |

导航模块的 toolbar 贡献使用 `scope=window(window_id)` 挂到常驻窗口级槽位，以所有者模块实例、贡献作用域、注册代次和 item/command ID 共同识别，不依赖 Panel 或 View。前进/后退按钮及其启用状态由导航模块声明；纯图标使用 TekesUI 图标引用、可访问名称与提示。切换尚未完成或持久化未确认时不允许继续修改历史。

toolbar 统一作用域规则：所有注册绑定所有者模块实例及其获准的贡献作用域；`scope=view(window_id, panel_id, view_id)` 额外绑定真实 Panel/View，`scope=window(window_id)` 只绑定真实窗口，不要求也不允许伪造 Panel/View。命令携带原注册作用域及代次并由所有者复核；相同 command ID 在不同模块/作用域中不混淆。View 的隐藏、移动或关闭不撤销无关窗口级贡献；窗口级贡献仅按所属窗口、所有者实例或授权生命周期清理。导航窗口命令按 history revision/事务校验，不套用 content 视口门控。

leading/trailing 显隐仍是 AppHost 的窗口布局操作，与导航历史分开；其纯图标控件保持可恢复入口，隐藏默认保留对应进程和状态。它们绑定物理槽位，不随内容模块换位。nav.json 不兼任布局状态文件。

## 3. nav.json 的持久化模型

导航模块是一个 history namespace 内唯一写入者；AppHost 仅在装配时提供存储位置/作用域及可恢复 route 的能力。多个 App/profile、workspace 或独立窗口历史必须使用不同 namespace/path；同一文件的第二个写入者须被拒绝，不能各持有可写副本。

示例（UTF-8 JSON）：

```json
{
  "version": 1,
  "history_id": "main-window-history",
  "revision": 12,
  "last_commit_id": "navtx-012",
  "cursor": 1,
  "entries": [
    { "id": "nav-001", "route": "projects", "params": {} },
    { "id": "nav-002", "route": "thread", "params": { "thread_id": "thread-123" } }
  ]
}
```

- `version` 是磁盘格式版本，与 IPC 协议版本独立；升级必须显式迁移，未知新版本不得按旧格式覆盖。
- `history_id` 是稳定的历史作用域，`revision` 是非负、每次成功历史提交递增的整数。`last_commit_id` 在已确认快照中标识最后一次提交，用于重复回执/重试去重；初始可为 null。替换后尚未确认的候选字段按 5.1 解释，不能仅凭可见 revision/last_commit_id 发布提交成功。
- entries 中 `id` 唯一且稳定，route 与 params 是可恢复的纯数据。仅记录 route、目标稳定 ID 和恢复参数；不保存 PID、运行时实例代次、Rust 对象、GPU/纹理句柄。
- 空历史为 `entries: []`、`cursor: null`；非空时 cursor 必须是 `[0, entries.length)` 的整数，指向当前已提交条目。加载时校验这些跨字段约束及 route/params 格式。
- push 成功时在当前 cursor 之后截断前进分支、追加一个新条目并移动游标。back/forward 成功只移动游标，不追加条目。不成功时不修改 entries、cursor、revision 或前进分支。
- 文件不存在时可初始化空历史；文件损坏、版本不支持或作用域不匹配时保留原文件并报告恢复状态，不静默覆盖为空历史。

`nav.json` 是已提交历史的恢复快照目标；只有已获 Host 执行成功的候选才允许进入替换阶段。替换前不得把待执行意图写成当前 cursor；替换后、持久性确认前可能已能读到完整候选，此时“路径可见”不等于已完成持久化确认，也不得提前发布内存历史、toolbar 或 HistoryCommitted。待确认事务必须具有可恢复日志；可实现为独立的 `nav.pending.json`，具体日志格式属于实现选择。它不是第二份已提交历史数据源，其创建、恢复和删除同样由导航模块独占。

## 4. 导航执行与提交顺序

每个 history 同时最多一个待完成事务，后续意图由导航模块明确排队或返回 busy。不得让多个 Host 回执并发修改一个游标。

1. **准备意图**：导航模块生成唯一 `navigation_id`，绑定 history_id、base_revision、push/back/forward、目标 entry/route/params、模块实例及当前 host_epoch。校验 back/forward 边界与目标格式，但不改已提交历史。发送执行请求前将待执行意图写入可恢复日志；日志保存失败就不执行导航。
2. **请求执行**：导航模块发送 NavigateRequest。AppHost 核对实例、当前执行上下文及目标能力，将 route 交给 target app 的路由适配器执行；不能把消息已送达或 worker 已接受当作导航成功。
3. **执行终态**：AppHost 对同一 navigation_id 只裁定一个执行终态。成功回执须表示目标上下文已按主架构第 9.1 节联合发布要求提交，并携带对应 transition/context 标识；失败、取消、被更新意图取代均返回明确结果，不发送成功。部分参与者准备完成不算成功。
4. **提交历史**：导航模块核对 navigation_id、实例/host_epoch、目标与 base_revision，收到有效成功回执后才构造应用 push 或 cursor 移动的候选快照，不提前替换内存中的已提交历史。以同目录临时文件写完整快照，flush/sync、原子替换 nav.json，并完成所声明持久化保证所需的父目录同步；写入不得暴露半份 JSON。替换发生与持久性确认是两个阶段，按 5.1 分别处理失败或结果未知。
5. **发布确认**：持久化成功后，导航模块一次更新内存中的已提交 entries/cursor/revision 和 toolbar 投影，返回 HistoryCommitted，再幂等清理 pending 日志。AppHost 必须保留本次执行状态和 receipt 至提交或对账明确完成，不能因此维护另一套历史列表；保留和回收规则见 5.2。

“执行成功”和“历史提交成功”是两个不同确认。AppHost 执行导航、导航模块提交历史；二者不是跨进程原子事务。

## 5. 失败、重复、取消与恢复

- 执行失败/取消/取代：保留原已提交历史和前进分支；pending 进入明确终态后清理。Host 维持/收敛内容上下文的规则沿用联合切换契约，不能用历史回滚代替参与者恢复。
- 超时/失联：视为结果未知，查询原 navigation_id 的执行结果；不能自行当作成功、失败或生成新 ID 重放 push。重复执行请求必须返回原结果；同一 ID 的不同 payload 必须拒绝。
- 迟到、重复或旧实例回执：按事务 ID、base_revision、模块实例和 host_epoch 核对；已终止事务不可复活，已提交 ID 不得重复追加/递增 revision。取消与成功按 Host 执行终态串行裁定，结果为成功时不能再声称已取消。
- 执行已成功但 nav.json 保存失败或持久性未确认：进入 applied-awaiting-persistence 并记录 5.1 的具体阶段，保留 pending、冻结后续历史变更并报告错误。只有替换前已知失败才可声称路径仍是旧快照；替换后不得这样声称。不得伪造 HistoryCommitted，也不得声称显示已自动回退。
- 导航进程重启而 Host 仍在：通过 5.2 恢复屏障重新读取完整快照与 pending，查询保留的事务 receipt、收敛写盘并核对实际上下文；不能仅凭当前画面猜测已经写盘。恢复查询须重新绑定握手身份，不能直接接受旧实例迟到的普通回执；Host 不因导航实例换代而清除待对账 receipt。
- Host 重启：生成新 host_epoch，必须先通过导航模块串行化的 5.2 恢复屏障，不能自行读取某一时刻的 nav.json 就恢复并宣布 ready。不盲目重放旧 host_epoch 的 pending 意图；旧结果无法判定或持久性未确认时保持明确的待对账/错误状态，不静默增添或丢弃历史。
- 恢复已提交目标不追加新历史；目标已删除/不可达或 schema 不兼容时返回失败/恢复选择，不静默改写历史。只有明确的新导航意图才可改变条目。
- 原子替换前崩溃、替换后确认丢失、pending 删除前崩溃均应可恢复：利用已提交 revision/last_commit_id 区分未提交与已提交，幂等清理；磁盘恢复只能接受完整旧快照或完整新快照；接受哪一基线及何时重新开放导航，仍须经过 5.1/5.2 的确认，不能仅凭文件能解析便宣布 ready。

### 5.1 替换前失败与替换后持久性未确认

以事务及其候选内容标识记录写盘阶段，不将一次“保存失败”笼统解释为文件未变化：

| 阶段/结果 | 可作出的判断 | 必须采取的动作 |
|---|---|---|
| 替换前已知失败：临时文件写入/sync 失败，或已确认替换未发生 | 仍是完整旧快照；候选未发布 | 保留 pending 和候选信息，冻结历史变更；使用同一 navigation_id 重试，不重做 Host 导航 |
| 替换已发生，但父目录 sync 失败或结果未知；或替换结果本身未知 | 路径可能已指向完整新快照；持久性尚未确认 | 标记 `replacement-durability-unknown`，保留 pending，禁止新事务和 HistoryCommitted；不得声称旧文件仍在，也不得反向覆盖旧文件来假装未提交 |
| 候选替换及所声明持久化步骤均已确认 | 新快照可作为已确认持久化基线 | 一次发布内存 entries/cursor/revision 和 toolbar，返回幂等 HistoryCommitted，再清理 pending |

替换后核对 history_id、候选 revision、last_commit_id 及候选内容是否与 pending 一致。当前路径为目标候选时，优先幂等重试/确认尚未完成的持久化步骤，不再次追加条目或递增 revision。已确认仍是原基线且原执行成功 receipt 可核实时，可重新保存同一个候选。路径为不相关版本、文件无效或无法确定执行/持久性结果时，保持待对账/错误状态，不猜测成功、回滚或覆盖。

导航模块须串行管理这些写盘结果；已完成替换、尚未确认的候选不是另一份可写历史。平台/文件系统若不能提供要求的持久化确认，应明确报告能力限制或失败，不能把原子可见性当作断电持久性保证。上述异常状态及重试同样参与 Host 恢复屏障。

### 5.2 Host / 导航进程重启恢复屏障

恢复必须由导航模块的同一 history 串行协调器裁定，与正常历史提交互斥。串行化指状态裁定顺序，不要求 AppHost UI 线程同步等待磁盘或子进程。Host 不读取任意文件快照后独立恢复到旧 revision。

1. **建立恢复会话**：新 Host 提交新的 host_epoch；导航进程重建时通过新实例身份重新握手。生成 recovery_id，进入 recovering，禁用新导航和历史动作；旧 epoch 的普通回执不能启动新提交。涉及恢复的内容交互仍受主架构 9.1 的上下文/输入门控约束。
2. **收敛提交和对账**：先收敛此前已接受成功回执而正在写盘的任务，包括替换前后操作及尚未处理的完成回调。若新 epoch 握手与写盘交错，必须先等该任务进入已确认提交、明确未提交或持久性未知状态，再决定恢复基线；不得一边发出旧基线，一边允许旧写盘把 revision 推进。进程重建后结合 pending、完整文件与原执行 receipt 恢复；未确认状态按 5.1 解决，无法解决则恢复失败/等待对账，不宣布 ready。
3. **提供冻结的恢复快照**：基线明确后，由导航模块提供 RestoreOffer，绑定 history_id、recovery_id、当前导航实例、host_epoch、revision、last_commit_id 及当前目标 entry/route/params。空历史显式提供空基线/默认显示策略。发出后继续冻结正常导航和历史提交；恢复本身不追加历史。
4. **执行与复核**：Host 根据 RestoreOffer 恢复目标，按联合发布契约完成后返回携带同一 token/revision 和实际提交上下文的 RecoveryApplied。导航模块在同一串行协调器内确认 token、epoch、实例、revision 及目标仍为当前基线，且没有未收敛事务/写盘。若基线已变化或旧 offer 被撤销，拒绝这次恢复完成，提供最新快照让 Host 重新恢复，不能宣布 ready。
5. **双方确认后开放**：导航模块返回绑定同一恢复会话的 RecoveryReady；Host 仅在自身实际上下文仍匹配该会话/基线时开放导航及相关输入，导航模块才发布正常 canGoBack/canGoForward。恢复失败、再次崩溃或迟到旧会话回执保持门控，重新握手；不能用一次旧成功通知跨过新屏障。

这条屏障保证：若旧 Host 在 A/revision N 后成功发布 B 且导航模块已接受提交，随后 Host 重启，则先完成/对账该 N+1 提交，再决定恢复 B 或明确的已确认基线；不能出现新 Host 恢复 A 并 ready，而旧写盘随后把历史推进到 B 的交错。

**执行 receipt 保留契约**：Host 对每个未收敛事务保留 navigation_id、history_id、发起实例/host_epoch、请求内容身份及执行状态/终态；终态成功附带目标和实际 context/transition 标识。收到可验证的 HistoryCommitted 或明确的终止/恢复对账完成确认前，不得按普通超时、导航断连、实例换代或容量策略丢弃该 receipt；容量不足应拒绝后续事务而不是丢弃对账依据。新导航实例通过受限恢复握手查询旧事务，不把旧普通回执直接当新实例消息。收敛后可回收详情，但仍须拒绝已终止事务的重放。receipt 是临时执行记录，不含可独立修改的 entries/cursor，不构成第二份导航历史。Host 本身崩溃导致 receipt 不可用时，必须明确结果未知并走恢复对账，不能伪造原成功或失败结果。

## 6. 验收与实现边界

- 独立启动导航进程，无 Panel/容器、零 renderer/GPU/shared-surface 创建，仍可完成握手、窗口作用域 toolbar 注册、命令/事件和停止回收；拒绝缺作用域、越权窗口、伪造 View 或旧注册代次。View 关闭/隐藏/换位不误注销无关窗口贡献，同 command ID 不跨所有者/作用域误路由。
- 同时运行三列 View 进程和一个导航进程；有 View 的资源/同步路径不变，无 View 的生命周期不等待 frame-ready 或 GPU ack。
- 正常 push/back/forward、到头禁用、后退后新建分支、空历史、重新打开恢复；Host 成功回执之前 nav.json 字节与游标均不变。
- 注入执行失败、取消/成功竞态、重复/乱序/旧实例回执、结果未知超时，确保一次意图最多提交一次。
- 分别注入替换前写入/sync 失败、rename 成功后父目录 sync 失败、替换结果未知及写盘各阶段崩溃；核对已可见候选不被误报为旧文件，持久性未确认时不发布 HistoryCommitted、不接收新事务，重试不反向覆盖/重复追加。
- 暂停提交线程，在 rename 前和 rename 后分别重启 Host：验证旧提交先收敛，RestoreOffer 与 RecoveryApplied/Ready 绑定同一 revision；基线变化必须重新恢复，最终可交互上下文与已提交 cursor 一致。
- 在收到执行成功后、HistoryCommitted 前重启导航进程，验证 Host 保留并可查询 receipt，恢复只补交一次；注入 receipt 不可用、旧 offer/旧 epoch 回执和再次重启，确认屏障不被绕过。
- 校验坏 JSON、未来版本、越界 cursor、重复 entry ID、同一 namespace 双写入者、不可恢复 route；原有数据不被静默覆盖。
- AppHost 不包含历史列表/游标的第二个可写模型或 nav.json 写入逻辑；导航模块不操纵窗口或实现另一套 View 容器。
- 当前 V1 输入/帧原型、真实桌面交互、完整捕获/IME/AX 以及发布/性能验收仍按既有证据分别标记；文档不等于这些功能已实现。

### 6.1 最小原型故障验收

以下三组故障注入是最小导航原型的必测要求。设计已允许进入实现；未通过这些验证不得声称持久化/重启恢复已验收。

1. **RecoveryReady 最后确认丢失**：导航模块核验 RecoveryApplied 并生成 Ready 后，丢弃发往 Host 的 RecoveryReady，再重复同一 RecoveryApplied 或查询恢复状态。Host 收到匹配确认前保持导航及相关输入门控；导航模块应保留可幂等查询/重发的当前会话 Ready 结果，不能因自身已转 ready 就拒绝重复 Applied，使 Host 永久等待；也可重新握手恢复。返回和使用 Ready 仍须核对当前 recovery_id、实例、host_epoch、revision 及实际上下文，过期会话不得被重发复活。导航侧核验 Applied 后记录 Ready，Host 收到并核验后开放本地输入，无需无限追加确认。
2. **HistoryCommitted、pending 与 receipt 的两种回收顺序**：分别注入① HistoryCommitted 未到 Host，导航已清理 pending 后崩溃；② Host 已收到确认并回收 receipt，导航尚未清理 pending 时崩溃。通过受限恢复握手，核对匹配的 history_id、已确认持久化快照、revision、last_commit_id 和事务/候选身份，幂等补发确认、完成对账或清理残留 pending。不得因任一临时记录已回收而重复执行导航、重复追加/递增 revision，或永久等待已不存在的记录。仅有可见候选时仍须按 5.1 确认持久性；证据不匹配则明确报错，不伪造执行 receipt 或提交结果。
3. **RecoveryApplied 与 RecoveryReady 之间再次崩溃**：在 Host 完成恢复发布后、Ready 生成前或到达前，分别杀死 Host、导航模块，并覆盖双方先后重启及延迟投递旧 Applied/Ready。新实例/host_epoch 重新进入 5.2 屏障，旧 token 不得开放新会话；未收敛写盘先完成对账，再核对恢复基线。最终成功时可交互上下文与确认的历史游标一致；无法确定结果时保持明确待对账/错误状态。暂停磁盘或子进程响应期间，Host UI 线程仍能处理窗口事件，不同步等待写盘、IPC 或进程退出。

每组验收记录须包含故障注入点、进程实例/host_epoch、recovery_id/navigation_id、基线和最终 revision/last_commit_id、输入门控状态及对账结果；不能只凭进程存活或 JSON 可解析判定通过。
