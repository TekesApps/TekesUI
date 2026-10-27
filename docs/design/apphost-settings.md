# AppHost Settings：TOML、action 与动态 Provider

日期：2026-10-22

状态：完整契约为设计要求；TOML/共享表单/两列窗口、本地自动保存及独立 Undo/Redo ProcessComponent 已进入最小原型，验证范围见 [原型记录](apphost-settings-prototype-20261022.md)，其余能力尚未实现或验收；本文的字段和操作名称为契约示意，未冻结 Rust API、TOML schema 或 IPC 编码。本文是 [AppHost 框架设计 §9.2](app-host-architecture.md#92-声明式-settings-与代码扩展) 的组成部分。原型不修改 ClientKit 或 Kernel 的业务实现。

## 当前 AppHost 保存策略（2026-10-22 更新）

AppHost Settings 默认使用自动保存，不显示底栏，也不提供 Apply/Cancel/Reset 按钮。本文后续 Apply/Cancel 的完整协议仍用于显式选择草稿提交模式的共享表单消费者，不是当前 AppHost 的交互要求；自动保存用相同的校验、稳定提交 ID、Provider 回执和 unknown 对账规则。

- 原 TextField/TextSurface 继续拥有编辑、选择、光标、输入法和文本 Undo/Redo。Settings 输入框使用无边框内容呈现与 trailing 对齐；长文本仍由原 TextSurface 滚动，不能只移动绘制而保留旧输入坐标。
- 有效的字段变更在短暂合并窗口（当前 300 ms）后串行保存，避免逐键磁盘写入；失焦及关闭请求触发立即 flush。非法中间输入和 IME marked text 不提交。校验失败或写入失败保留编辑，仅失败时出现错误和 Retry；成功/未修改时没有状态底栏。
- 关闭/Escape 不再丢弃草稿。存在有效待保存编辑时，等持久化回执后关闭；非法输入或保存失败保持窗口可纠正/重试。已保存值始终不因关闭而回滚。正常保存期间允许继续输入，回执只退休被捕获批次，不清除新输入。
- 独立无 View 的 `settings-undo` ProcessComponent 拥有配置历史和 Undo/Redo toolbar 注册；AppHost 仅装配命令、投影 enabled 并由原 Provider 执行配置写入。按钮放在 Settings 左侧 navbar，center 仍只提供面包屑。导航历史与配置历史互不写入。
- Undo/Redo 针对已保存配置，以一次保存批次为历史单位，撤销也是一个新 revision/commit ID 的持久化操作。输入框内的 ⌘Z/⇧⌘Z 仍由文本组件处理；toolbar 的 Undo/Redo 针对配置，未聚焦文本时快捷键可路由到配置历史。编辑未保存、保存/对账未完成或 IME 活跃时禁用配置撤销；执行配置撤销时暂时关闭表单输入，更新表单后再开放。
- 配置继续保存在 `settings.toml`；相邻 `undo.json` 是有命名空间、版本、cursor 和 pending 的持久历史，独占写锁、0600、临时文件 fsync/rename/目录 fsync。历史 prepare 必须先持久化；Provider 确认配置 durable 后才推进 cursor。未知结果只核对原提交，不再产生新写入。历史配置快照通过 TOML 字符串携带，保留未知键、datetime 等 TOML 类型。
- 重启时先读取实际配置并对账 pending：等于 before 则撤销未执行的 prepare，等于 after 则补齐历史确认；与两者均不同则停用历史并报告冲突。恢复完成前不开放撤销。确认丢失、重复确认不能重复增加历史。Undo 后的新保存清理 redo 分支；显隐和导航不产生配置历史。
- 此原型只处理本地非凭证配置；凭证、多 Provider、业务动作及动态 Provider 仍需下文完整契约，不把撤销声称为回滚外部副作用。

验收必须覆盖：无底栏；有效编辑自动落盘；非法输入保留且不落盘；尾部文字/光标/点击一致；原生 toolbar Undo/Redo 同时更新表单及磁盘；撤销后不被退休编辑回写；保存期间较新编辑保留；写入前中断不增加历史；写入后确认丢失重启只补齐一次；重启后历史可用；未知 TOML 键完整往返；导航、leading 显隐和原有 WIP 保持。

## 1. 目标与职责

消费者声明设置的名称、层级、值类型和约束，框架生成 UI；消费者通过代码注册 action 和动态 Provider，保留启动 Harness、重启、选择路径、连接测试等行为。共享的是运行框架及组件，不是复制到每个 App 的页面模板。

| 所有者 | 职责 |
|---|---|
| TekesUI | 通用表单、嵌套对象、列表/表格与类型编辑器；布局、样式、焦点、IME、键盘、无障碍、校验和动作反馈 |
| AppHost | Settings 菜单/快捷键入口与窗口；固定注册表、路由、Provider 绑定与生命周期、草稿/关闭协调；组合共享控件 |
| 消费者 / Kit | 定义和本地化资源、业务校验、权威配置存储、动态数据、动作实现、权限与系统能力适配 |
| ProcessComponent | 可选的无 View Provider 服务运输与进程生命周期；不拥有通用 Settings UI，不新增一套 IPC |

Agent Settings 和其业务呈现状态仍由 ClientKit 所有，Host 不复制其状态机。本文借鉴现有能力建立通用扩展面，不承诺直接把 NSView/Swift closure 放入 Rust 或跨 IPC 传递。未来 Agent 适配需要明确桥接/迁移方案，遵守既有所有权及跨仓验证要求。本次不修改 Kernel。

### 1.1 AppWindow 与 SettingsWindow

AppHost 对外提供共享 `AppWindow` 装配入口（两列 leading/center 或三列 leading/center/trailing；单列保留为技术验证 profile）。`SettingsWindow` 使用公共 `AppWindow::from_columns` 的两个普通 `Container`（原始目录与表单 ContentHost）装配，是同一 AppWindow 的两列消费者：不装配 trailing，复用 toolbar、导航 ProcessComponent、leading 显隐/动画及 splitter。Settings 的表单与目录使用 Host 内本地 TekesUI 内容，首版不为每个设置页启动 View 子进程；业务 Provider 仍可按 §7 的服务契约扩展。

消费者以 AppWindow 装配 Main，以 SettingsWindow 提供 TOML、Provider/action 绑定及展示策略；AppHost 不包含产品页面或业务动作分支。展示策略不是 TOML 字段类型：

| DisplayMode | 行为 |
|---|---|
| `Exclusive`（默认） | Settings 在首次显示前使用发起窗口当时的 frame/大小，先覆盖发起窗口，再在同一无动画窗口操作组内隐藏包括发起窗口在内的其他原本可见 App 窗口；拖动 Settings 不露出 Main。关闭时先在 Settings 下方恢复先前可见窗口，再移除 Settings 并恢复原焦点，不主动显示原先隐藏或最小化的窗口 |
| `Independent` | Settings 为独立窗口，其他 App 窗口保持各自状态，可以并存及切换 |

Settings 左栏用标准 TekesUI `List` / `ListView` 展示 Group 下的 Item，选择色、hover、行高及键盘选择均由共享控件控制，消费者不覆写。左栏不声明 toolbar title。右侧当前 Item 根据其 Group 祖先路径提供面包屑标题（例如 `阅读 › 文本与语言`），作为 center 的 `ToolbarGroup(ToolbarPlacement::Navigation)` contribution；前进/后退保留在左侧 navbar 的独立导航 contribution，由 `ToolbarContent` 负责组内间距及 placement；窄宽度时由共享 toolbar 的约束/overflow 处理，不新增 AppHost 自定义显示规则。窗口的系统标题仍为 Settings，与导航标题独立。

同一消费者重复打开 Settings 复用现有窗口及草稿，不创建第二个配置写入者。关闭 Settings 不等于退出 App；正常退出须遵守正在写入及 dirty 的保护。Exclusive 是窗口展示策略，不要求嵌套 AppKit 事件循环，也不承诺是系统模态 sheet。

## 2. 统一定义：TOML 声明、代码绑定

`settings.schema.toml` 随目标 App 固定发布，声明 definition 和 presentation；`settings.toml` 是可选的用户覆盖存储格式，两者不是同一个文件。已有配置仓库、凭证存储或远端服务通过 Provider 接入，不为采用本框架另造一份权威配置。

TOML 和程序化声明都转换到同一 Settings 定义模型。稳定 namespace、page/group/item ID、结构化数据 path 与展示文案分离；改名、本地化或移动展示位置不改变值身份。路径按字符串段存储，不靠拆分 label、斜杠或点推断。schema_version 与用户配置迁移版本分开；持久化重命名由权威 Provider 显式迁移。

示例：

```toml
schema_version = 1
namespace = "app"
provider = "app.settings"

[[groups]]
id = "runtime"
label = "运行环境"

[[groups.items]]
id = "harness"
label = "Harness"

[[groups.items.fields]]
id = "directory"
path = ["runtime", "directory"]
label = "安装目录"
type = "string"
default = ""

[[groups.items.fields]]
id = "model"
path = ["runtime", "model"]
label = "模型"
type = "enum"
options_provider = "runtime.models"

[[groups.items.actions]]
id = "launch"
label = "启动 Harness"
action = "harness.launch"
input_policy = "committed"
```

消费者以代码注册 `app.settings`、`runtime.models` 和 `harness.launch`。Item.actions 中的动作声明不持有数据 path/default，不把点击记录成配置值；处理器不在 TOML 中表达，也不将 TOML 字符串直接作为 shell 命令执行。

标准业务层次采用 **Group → Item → Field**：左侧以 Group 分组并列出可选择的 Item，center 展示当前 Item 的 Fields 与 Actions。Group/Item 是展示与路由身份，Field 的持久化 path 单独声明；改组、改标题不能隐式迁移配置。TOML 用 `[[groups]]`、`[[groups.items]]`、`[[groups.items.fields]]` 表达三层；动作使用 Item 下的 `[[groups.items.actions]]` 声明绑定，代码提供实现。可选 page 是外层装配/导航元数据，不是必需的第四级业务层次。

页面/组支持 parent ID、顺序、标题、描述、footer、图标与本地化 key；字段支持说明、只读、可用性、隐藏、范围、格式和 editor hint。列表/表格列及嵌套字段同样用稳定 ID 引用。注册时校验重复身份、循环层级、缺失引用、默认值类型和不支持的 schema 版本，诊断包含文件位置/字段路径。固定缺失绑定是构建/装配错误；已注册 Provider 暂时离线是运行状态，显示不可用和原因，不能静默丢行。

## 3. 类型、层级与存储

| 数据/语义 | 默认 UI 与扩展 |
|---|---|
| boolean | Switch；多开关支持命名、锁定和不支持状态 |
| string | TextField；多行 hint 用 TextEditor；支持只读标签、占位符和 secure 输入 |
| integer / decimal | 数值编辑；范围校验，可声明 stepper/slider hint |
| enum / multiselect | choice、segmented 或列表；静态或动态选项，值 ID 与显示名分离 |
| object | 命名分组、嵌套详情、折叠展开；展示分组不要求与数据对象一一对应 |
| array / map | 列表/表格、增删、选择、单元格编辑和详情；条目稳定 ID，不能把当前行号当身份 |
| action / status | 按钮或只读状态、进度、结果、错误；不伪造持久化值 |
| 特殊编辑器 | 文件/目录、凭证、日期时间等语义提示或注册 editor ID；未知类型给出明确诊断 |

统一值模型区分未设置/继承、显式空字符串、空集合与可选的显式 null。TOML 没有原生 null，不能用空字符串冒充；需要 null 的 namespace 必须使用明确带标签的覆盖编码或由 Provider 的既有格式存储，首版若不支持则拒绝该声明并提示。时间值、数值范围和序列化表示必须由类型契约规定，不能靠控件猜测。

用户覆盖文件的普通示例：

```toml
[appearance]
theme = "dark"

[runtime]
model = "model-a"
```

只保存覆盖值，不把继承/解析后的有效值整份写回。Reset 生成 unset，重新计算默认值与继承，不把当前默认值写成永久覆盖。Provider 负责精确类型往返、未知配置键保留、迁移和持久化结果。secret 只读回 configured/source/writable 等元信息；新输入走专用写入/清除能力，不回显已有凭证，不落入普通 TOML、日志或错误消息。

## 4. 动态 Provider 与提交

Provider 是代码实现的能力接口，不限于数据存储：配置 Provider 提供 describe/read/validate/commit/reset/subscribe，选项和集合 Provider 提供动态枚举、查询/分页、增删编辑、可用性及状态订阅。TOML 引用稳定 Provider ID，Host 固定装配具体实现；不动态发现任意插件。

配置快照包含 namespace、schema revision、数据 revision、有效值、base/user 覆盖、权限和生效策略。UI 草稿与快照分离；commit 携带 expected revision、唯一请求 ID 及 set/unset 路径操作。namespace 内的一次配置操作批次是最小原子提交单位；此原子性默认不涵盖独立凭证存储或外部副作用。跨 Provider/资源 Apply 分别报告结果，不声称分布式原子性。Provider 必须声明持久化确认和生效状态：已保存但待重启、已保存但应用失败都不能伪装为未保存或完全生效。

默认 Apply/Cancel；即时保存只可作为显式策略，仍需失败反馈、写入跟踪与权威回执。确定失败/冲突保留草稿；结果未知时进入待对账状态，不当作失败重新提交；确认成功才接受新 revision。外部更新无脏字段可增量更新；脏字段冲突显式解决，不覆盖正在输入的值。Provider 使用服务端业务校验，客户端校验只负责及时反馈。

动态请求关联 owner/session、请求序号、schema/数据 revision 及依赖值版本；新查询或切页后撤销旧订阅，迟到结果不能覆盖新状态。动态选项减少时保留当前值并显示已失效，不自动选中第一项。稳定 row/item ID 保持焦点、IME、选择、展开和滚动，不能每次通知重建整页。

### 4.1 保存回执丢失与恢复屏障

配置 commit 与 action 都可能出现结果未知。commit 使用跨运输 session 稳定的 `commit_id`，绑定 Provider/namespace、schema/base revision 和不可变操作批次；同 ID 不得换 payload。Provider 在核对新请求的 expected revision 前先识别已知 commit ID，重发同请求返回原结果或处理中状态，不能将已提交请求简单判为旧 revision 冲突。结果查询也绑定同一资源身份；旧 session 的直接回执不可更新新实例，新连接通过当前 session 的对账返回权威结果。

Provider 区分以下状态，不能仅凭超时、进程退出或当前值相等判断：

| 状态 | 含义与处理 |
|---|---|
| 确定未写入 / 拒绝 / 冲突 | 权威方确认未接受或未持久化；保留草稿，可在重新读取/解决冲突后形成新请求 |
| 处理中 / unknown | 无终态证据；保留请求身份，查询或重连对账，不能自动换 ID 重提 |
| 已替换但持久性未确认 | 新值可能已可见；继续确认原提交的持久性，不宣称旧文件仍在，也不反向覆盖伪造回滚 |
| 已持久化 | 提供 commit ID、提交 revision/结果以及持久性确认；保存完成与应用生效另行记录 |

Provider 提供可恢复的 commit 结果查询或等价的持久化日志/快照对账，使写入和提交身份的恢复相互一致；不能留下“值已保存但永远无法判定属于哪个 commit”的正常恢复窗口。结果记录保留至协调方确认退休，或有等价的可恢复压缩基线；若证据丢失/超出保留能力，明确返回 unresolved，不能以 not-found 推断从未写入。首版采用无此能力的 Provider 时，必须明确限制恢复保证，不能宣称跨进程故障验收通过。

Provider 重启/断线后，Settings 协调方先收敛相关 pending commit，再读取带 revision 的权威快照，核对最新基线后开放该资源的新 Apply、Reset 提交与依赖它的动作；其他无关资源仍可操作。对账发现原提交成功，即使当前快照已被后续写入推进，也分别记录原结果和当前基线，不能回退权威 revision。成功回执只清理该批次已保存的编辑，提交期间的新修改保持 dirty；冲突按当前基线显式解决。

Host 保存未退休的请求身份、批次版本、复合 Apply 进度和动作衔接状态，重启后仍可查询；这是恢复协调记录，不是第二份配置真值。不得把 secret 明文放进协调记录。无法恢复的未提交草稿/凭证输入须明确提示重新输入，不声称已恢复。Host 或 Provider 任一侧崩溃、重复恢复通知与查询均须幂等。

本文明确沿用 [主设计 §10](app-host-architecture.md#10-隐藏关闭退出与恢复)：关闭批准绑定编辑/保存版本和实例，执行前复核。未确认写入时不把 Cancel 当作撤回成功；正常关闭先等待/对账，提供重连、查询和错误反馈，不能无限无提示 busy。强制退出不制造保存成功或失败，留待下次启动恢复；重新开启窗口也不能绕过该资源的恢复屏障。

### 4.2 配置与凭证的部分成功

一次页面 Apply 可以协调多个配置 namespace、凭证 write/clear 及后续业务动作，但默认没有跨存储事务。每个资源有自己的请求 ID、版本/条件和终态；凭证能力也须提供不泄露值的去重、状态查询和未知结果恢复。只有 Provider 明确声明并实现统一事务，才可把配置和凭证称作同一原子提交。

复合 Apply 固定本轮输入、必要资源及提交顺序，按资源记录并展示结果：配置成功则接受其 revision，不能因凭证失败撤销该回执；失败或 unknown 的资源保持未完成状态，保留尚有效的草稿。已确认成功的凭证写入可清理对应敏感输入，但同时有新编辑时不能清理新版本。重试仅针对确定失败且仍需提交的资源；unknown 先按 §4.1 对账，不能重放已成功部分。凭证不可恢复的内存输入丢失后要求重新输入，不自动读取旧秘密来填回。

`commit_then_run` 显式声明全部必要资源，只有本轮配置/凭证全部确认成功，且执行时这些版本/凭证元信息仍适用，才允许动作执行；任一失败、冲突或 unknown 都不启动。动作携带本轮资源回执引用与稳定 invocation ID，不读取漂移的“最新值”冒充本轮输入。保存后启动失败分别显示“已保存、启动失败”，不自动回滚配置或凭证；动作显式承诺的业务补偿另行报告。

Cancel 只丢弃尚未提交的编辑，不回滚任何已保存资源。Cancel/离页丢弃撤销尚未开始的本轮自动后续动作；已运行的动作仍按 §5 查询/取消和对账。若崩溃发生在全部资源成功与动作确认之间，恢复须核对已记录的衔接状态和 invocation 结果，不能靠重新打开页面就再次启动；无法确认时保留等待或请求明确的继续操作。

## 5. action 代码注册与副作用

每个 action 以 namespace/owner 下稳定 ID 注册异步处理器、输入/结果类型、可用条件和并发策略；TOML 只引用它，并声明标题、位置、样式提示和配置输入策略：

- `committed`：默认，只读取已提交的明确 revision；脏草稿不会悄悄带入启动动作。
- `commit_then_run`：先校验并提交声明的必要 namespace/凭证资源，再以全部成功回执及其版本执行动作；失败、冲突或 unknown 则不运行。启动/重启组合不保证外部副作用与持久化原子，分别反馈结果。
- `draft`：仅显式声明支持草稿的处理器可接收带基础 revision 的草稿快照，例如连接测试；不据此将草稿标成已保存。

例如 Harness 的选择目录、恢复默认目录、Apply and Connect、Restart Runtime 可以分别绑定 action；动态状态决定按钮 enabled/主次样式/提示，进度和输出由 Provider 投影。切换页面或窗口重绘不会重复执行。动作有 invocation ID，busy 时按声明拒绝或串行化重复点击；同一请求重复到达由执行方去重。

框架统一显示忙碌、进度、成功、失败与可取消状态。取消请求不等于副作用已回滚；超时/断线后结果可为 unknown，先向执行方查询，不自动重试启动或重启。进程重启改变 session，旧回执不更新新实例；跨重启的副作用对账由动作所有者提供，不能仅靠新会话 ID 就再次执行。Settings 关闭/取消编辑不自动取消运行任务，也不自动撤销已经保存的配置。

脏草稿离页/关闭需保留或明确放弃；有未确认写入时阻止误关闭，并提供可诊断的等待/失败处理，不能永久忙碌。业务任务可以有独立寿命，关闭设置后仍由所有者管理。框架不在 UI 线程等待文件写入、进程或动作完成。

### 5.1 草稿操作、异步返回与 Cancel

默认 Apply/Cancel 模式下，字段编辑、Reset/unset、集合增删改/重排、选择目录和恢复默认目录均只产生草稿操作；集合 Provider 的持久化变更必须等 Apply 才执行。尚未保存的新条目也有稳定草稿 ID，提交后如映射为持久 ID，由回执明确映射并保留对应选择/编辑状态。Reset 不直接调用会立即持久化的 Provider reset；配置 reset 能力接受提交上下文并遵循 §4。集合操作的权威业务校验在提交时再检查，不能靠旧列表索引执行删除。

普通 `draft` input_policy 仅授权处理器读取快照，不自动授权写回。能改变草稿的 action 另行声明结果为 draft patch，允许的目标字段/集合及操作类型；目录选择只返回路径 patch，关闭文件选择器时的 Cancel 不产生 patch。特殊编辑器遵守同一协议，不直接调用持久化存储。确需即时副作用的删除/安装等动作必须显式声明并说明无法由编辑 Cancel 撤销，走独立 action 的结果/取消语义，不伪装成待 Apply 的草稿操作。

每个草稿 patch 绑定 `edit_session_id`、草稿代次、基础配置 revision、目标字段版本，以及受影响集合/条目的稳定 ID 和结构版本。只在会话仍有效、声明目标存在且相关版本匹配时按批次校验并应用。无关字段更新不必作废结果；目标已变化、条目已删除、集合已重排导致依赖失效或配置冲突时，拒绝覆盖并给出重新选择/显式合并入口，不能把旧选择写进新目标。IME 未确认的输入也属于待协调的编辑，不能被异步 patch 覆盖。

Settings 的 Cancel/明确放弃丢弃所选范围的未提交草稿，推进编辑代次，使该范围全部挂起草稿 patch 失效，并取消未开始的相关提交/后续动作；已发出的写入不能据此当作未发生，仍由 §4 对账。导航到其他页面若保留草稿，会话继续存在；若选择放弃则执行相同失效规则。主设计 §10 的保存/关闭版本检查同样适用，保存期间继续编辑不会被旧回执清空。

编辑 Cancel、正在运行 action 的取消请求、Provider 写入终态是三件不同的事。Cancel 不是事务回滚，也不能使迟到的目录返回复活草稿；已经开始的外部任务由所有者继续管理。即使 UI 已丢弃该范围草稿，已发出写入的恢复记录仍保留至终态，不接受仅凭旧窗口回执重新创建编辑会话。

## 6. 保留 ClientKit 的能力

以下来自当前 ClientKit 源码的能力作为新框架的验收基线；通用能力可分期实现，但未完成前不能声称能力等价：

| ClientKit 参考 | 需保留的能力 |
|---|---|
| `SettingsSectionView.swift` / `SettingsSectionItem` | 标签、toggle/text/secure/choice/segmented/stepper/多开关、说明/图标、隐藏、嵌套详情、稳定行与展开状态 |
| `ClientSettingsContent.swift` / `ClientSettingsFormPageView.swift` | form/list/table/custom，增删/选择/单元格编辑、busy/空态、页宽和缩放、增量更新及输入焦点保留 |
| `Agent/ClientAgentSettingsConfiguration.swift` | 消费者注册自己的导航条目与页面，同时复用 Kit 已有业务页面；搜索/路由与显示文案分离 |
| `Agent/ClientHarnessStartupSettings.swift` | 目录验证与选择、默认目录、Apply and Connect、Restart Runtime、进度/结果/连接状态，以及由业务代码决定的恢复逻辑 |
| `SettingsCloseGuard.swift` | 未保存与正在写入保护、显式放弃、多编辑者协调 |
| `SessionSettings.swift` / `DSHProviderSettingsDraft.swift` | namespace/schema/base/user/revision、路径 set/unset、凭证元信息、失效通知、成功后接受草稿 |

自定义内容通过代码注册 editor/page ID，TOML 引用；通用字段优先使用共享渲染器。自定义内容仍受统一宽度、主题、缩放、焦点/IME/AX、草稿和关闭生命周期约束。Host 本地工厂可提供 TekesUI 内容，但不得跨 IPC 传 NSView、裸指针或 closure。进程 Provider 首版只输出数据与动作；特殊的远端自定义 View 若有需要，另用显式 View/ProcessContainer 契约设计，不能借 Settings 扩展绕过进程边界。

## 7. Host 与进程装配

Settings UI 初版在 Host 呈现；本地 Provider 与无 View ProcessComponent Provider 实现同一职责契约，运输复用既有服务消息及版本协商。schema 和注册关系来自固定装配，打开 Settings 不要求启动隐藏的 leading/trailing Panel；动态能力确需进程时可按服务生命周期启动，不创建空容器、GPU 或共享纹理。

Settings 的 Item 导航复用现有导航 ProcessComponent，在独立的 Settings history/目录内保存 route 与稳定 Item ID；每次访问使用独立 history entry ID，重复访问同一 Item 不复用旧 entry ID；它不改 Main 的导航历史。字段修改、Apply/Cancel/Reset 和 leading 显隐不写导航历史，选择 Item 和前进/后退才推进 Settings history。页面草稿仍归编辑协调方，配置值仍归 Provider，导航记录不得包含字段值、秘密或草稿。Item 切换保留 namespace 草稿，返回时重建该 Item 的共享表单投影；同一 Item 内普通更新不重建控件。设置改变全局主题/字体时，经 Host 的 environment revision 广播，遵守主设计 §8.2；只保存配置不代表所有进程已应用新环境。Toolbar、系统红绿灯及三列渲染/输入契约不因本扩展改变。

## 8. 分期与验收

1. 定义模型及 TOML 解析：验证多层名称/对象、数组/map、不同值类型、默认/继承/unset/null、类型诊断、重复/缺失引用、迁移及 action/provider 绑定；文档示例可解析不等于框架已实现。
2. TekesUI 通用渲染器：只增声明就能增加常规字段，不再写逐字段 UI；覆盖表单、嵌套、列表/表格、动态选项、特殊 editor、主题/缩放、键盘/IME/AX，更新时不丢草稿/焦点。
3. AppHost 装配：真实 Settings 入口、窗口、Apply/Cancel/Reset 与关闭保护；至少一个独立公共 API 消费者复用同一实现。确认既有配置仓库仍为唯一权威来源，业务副作用不进入 Host 分支。
4. 动态与进程：演示 Harness 类启动/重启动作、目录选择、实时进度与选项；验证加载/离线、迟到结果、脏字段外部变更、revision 冲突、重复点击、写入失败、保存成功但应用失败、取消/unknown 及进程重启对账。隐藏 Panel 不启动也能看到固定 Settings 定义，且不出现额外 View/GPU。

每期分别记录源码/测试和真实窗口交互证据；通用编译、TOML 解析或协议定义不能替代 native Settings 与动作验收。涉及 ClientKit/Agent 公共界面改动时另行验证共享包、Tekes Host、第二消费者及所有权检查；当前完整契约的原型落实范围、构建及真实窗口证据另见原型记录；未覆盖能力不能宣称完成。

### 8.1 三组 P2 故障与交互验收

以下是进入对应功能验收前必须通过的场景，不是实现已通过的声明。记录资源身份、编辑/进程代次、请求/commit/invocation ID、revision、终态与门控顺序；证据不得包含凭证明文。

| 场景 | 注入与必须观察的结果 |
|---|---|
| 保存后丢回执 | 配置已持久化后杀 Provider，再分别重启 Provider/Host；恢复查询识别同一 commit，仅写入一次，接受已保存 revision，再开放同资源 Apply/依赖动作；同 ID 重发不误报旧 revision 冲突 |
| 保存前失败与持久性未知 | 分别在写入前、替换后持久性确认前中断；前者确认未写入并保留草稿，后者保持待对账，不声称旧文件保留、不自动换 ID 重提；查询 not-found 且无证明时保持 unresolved |
| 对账与新编辑 | 丢回执后外部写入推进 revision，或保存期间用户继续编辑；原提交和当前基线分别确认，新修改保持 dirty。重复/迟到旧 session 回执不越过恢复屏障，不回退版本；关闭批准失效后重新准备 |
| 配置成功、凭证失败 | 修改 URL 与 API key，注入凭证失败；配置 revision 被接受、凭证仍未完成、启动次数为零。只重试失败资源；Cancel 不回滚已保存 URL |
| 凭证成功、配置失败或 unknown | 交换资源顺序并丢失一侧回执；显示各自结果、不重复写成功资源；unknown 先查询。两侧确认且版本仍适用才最多触发一次连接动作；保存成功但启动失败显示独立结果 |
| 凭证/复合 Apply 重启 | 在各资源完成及衔接动作前后退出 Host/Provider；恢复不记录/回显 secret，缺失未提交凭证明确要求重输；动作未知先查询，不因新 session 或重新打开页面重复执行；丢弃后不启动尚未开始的动作 |
| 集合编辑与 Reset 后 Cancel | 新增、删除、编辑、重排嵌套集合并 Reset，随后 Cancel；权威存储和 revision 均未改变，草稿复原。Apply 时原子提交声明的配置批次，稳定 ID 未误指向其他行 |
| 目录选择迟到 | 打开选择器后修改目标字段、删除目标条目、丢弃草稿或关闭再重开设置；旧返回 patch 被拒绝，不能覆盖新编辑或复活旧会话。无关字段改变时合法 patch 可接受；选择器自身 Cancel 不改草稿 |
| 取消与已发出写入并发 | 保存发出后 Cancel/离页，同时注入迟到成功与进程中断；不虚报撤回、不丢 pending 身份、不重建已丢弃草稿，最终准确展示已保存部分；其他资源/窗口保持响应 |

纯 TOML/静态表单可以先实现；进入 Apply/Cancel、凭证、Harness 动作或跨进程恢复阶段时，必须完成各自对应场景及真实窗口验证，不能以静态 UI 或单次成功写入替代。
