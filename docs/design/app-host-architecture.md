## 2026-11-11：Container 的 tab/list 互斥呈现（#229）

LocalContainer 和 ProcessContainer 使用 `.presentation(ContainerPresentation::Tab)`（默认）或 `.presentation(ContainerPresentation::List { height })`；旧 `.list_height(height)` 保留为选择 List 的兼容写法，共用同一份模式存储。

每个 Container 仅声明一种呈现模式：tab 或 list。此模式与 Toolbar 的 host=window/container 是两个独立概念。tab 模式进入内容类型图标组，不作为聚合 List 成员；list 模式仅进入聚合 List，不同时产生单独类型图标。当前没有打开的 list 模式 Container 时，不显示聚合 List 图标。Gallery 的 Inspector 示例使用 tab 模式，因此不重复出现在 List。

工具栏背景、内容图标组与弹出菜单分别保持原层次；弹出菜单出现时只提升 popup，不连同整块 Toolbar 背景覆盖图标组。渲染沿原一次布局与绘制拆分 base/overlay，旧调用保留原合并行为。

`+` 仅展开创建/打开菜单，菜单展开本身不改变 Container 的 opened/selected 状态或已有类型图标投影；只有实际选择创建/打开项、关闭实例等操作才更新成员。保留既有实例身份与输入撤销合同，不由菜单开关重建或隐藏原图标组。此节覆盖此前允许同一成员同时出现在 tab 与 List 的例子；具体实现和验证状态以 #229 当前记录为准。

## 2026-11-11: registered Others creation (#229)

Others visibility and creation are separate actions: its main icon only hides/shows the existing selection; an adjacent `+` icon opens the menu of registered creatable types. An empty visible Others presents the same registrations as a creation grid. The same directory includes existing fixed containers as explicit Open entries which retain their original Content, navigation and identity; factory entries genuinely create according to their policy. Type radio icons project opened instances only; hiding preserves them, closing the last instance removes that type from the radio group. Cancel does not create or select anything.

`ContainerFactory` explicitly declares Single or Multiple and constructs original local Containers, or names a compile-time Process ModuleDefinition registered before worker dispatch. Single reopens its retained mounted instance; Multiple creates a fresh stable instance and closing it cancels its worker and releases its local Content/Toolbar. A lightweight non-routable index tombstone preserves other original Source indices. Instance IDs never recycle. Process instance identity is separate from worker factory ID, which only selects the existing executable worker; IPC remains unchanged. Each Process worker ID is registered once; one Multiple factory covers all instances of that type. Duplicate worker IDs are rejected rather than comparing function addresses. Existing Vec<Container> assemblies remain supported. Creation appends original Panel records and PresentationState membership; it does not rebuild other containers, change Workspace Switch/Nav ownership, or introduce another selection state.

Toolbar icon buttons, including menu triggers, do not paint a focus border. Logical focus, keyboard activation, and hover/pressed/checked fills remain; ordinary non-toolbar buttons retain their focus decoration. Family selection menus retain their icon-and-chevron presentation.

# TekesUI AppHost 框架设计

## 2026-11-11：Workspace 交换与 Secondary actions（#229）

Switch 是 Workspace（Focus + Others）的单一布局命令，不由每个 Container 重复声明；它交换当前 Focus 和当前选中的普通 Others Container。Others 隐藏、未配置或选中聚合 List 时不可用，不将 List 整体提升。动作执行仍保持原实例、导航绑定与输入取消规则。

Secondary actions 通过三点图标菜单呈现，菜单项目沿原命令来源派发，不能因汇总丢失 Container 的实例/代际身份。Workspace 命令只注册一次；Container 的业务次要动作仍由 Container 声明，按它自己的 toolbar host 呈现，不迁移 Nav/Focus/Others 的列归属。前进后退、内容图标选择组及 Others 的展开/关闭/显隐保留直接导航按钮。菜单打开后若目标切换、关闭或失效，原捕获与旧动作必须撤销；不创建第二套菜单/焦点状态机。

展开时隐藏 Focus 仍承载原 Window Toolbar；绘制必须保留布局给出的零宽原始 frame，不能每帧改成不同的空 Rect，造成未变布局反复取消菜单。真实几何变化仍撤销旧输入。

此节是本批追加设计，替代旧版每个 Container 的 Switch 注册示例；实现和验收状态以 #229 当前记录为准。

## 2026-11-11：图标组状态与 Others 展开导航（#229 / #245）

本节为用户最新确认的增量，取代下方旧批次“Others Close 固定在内部右上”的位置规则；旧记录只保留设计沿革。源码与受影响验收按 #229 / #245 当前交付记录核对；设计声明不代替验收。

Window Toolbar 继续按 Nav / Focus / Others 实际区域组织。 前进/后退是 NavContainer 注册的 icon button commands：Gallery 与带 Nav 的 AppHost 模板把导航 Toolbar 显式绑定 NavContainer，模板的 Nav Toolbar 使用 Window host，保持在导航区域，不随 Focus/Others 交换。导航命令的展示 owner 与业务 navigation_targets 独立，业务目标仍为原文档实例；Nav 隐藏时不把历史命令搬到 Focus。无 Nav 的显式消费者保留自己的 Toolbar 绑定。Others 的内容类型图标组成一个共享选择组：`radio=true` 表示同组互斥，`radio=false` 表示无该互斥约束；本场景使用 true，选中身份仍来自原当前内容，不复制第二份选中状态。普通单图标按钮使用与常规 icon button 相同的自然尺寸、hover/pressed/checked 状态和 token；family 图标与 chevron 水平居中，额外宽度只来自实际箭头和既有间距，不任意拉宽命中背景。

每个 Container（包括当前 Focus 和 Others 成员）沿现有 `switcher_icon` 声明自己的图标；无声明统一采用现有图标库 `tekes-file-generic`。图标随稳定 Container 身份交换，不存储第二份角色图标；family trigger 采用该组当前实例（未选中组采用首个可见实例）的图标加 chevron，菜单仍使用原实例标题和 checked 身份。原 family 配置保留其分组身份，不覆盖实例图标。聚合 List 不提供一键 Close／Expand；其成员仍按单个 Container 合同处理。

Others 布局导航独立于内容选择组，按 `[expand/collapse] [close] [on/off]` 排列在对应 Window 段。Close 为原当前可关闭的单个 Others Container 派发，Focus 不显示 Close；聚合 List 不凭一个 Close 关闭所有成员。业务 Toolbar 的 `.host` 仍只决定业务项呈现位置，不隐式把这些 Window 导航动作复制到内部 Toolbar。关闭按钮迁出后不保留空白内部 Close 条。

Expand 将 Others 当前显示区域向左扩展到 Focus+Others 的总宽度，Nav 不受影响；Focus 隐藏而非关闭，保留内容实例、草稿、导航绑定和原布局比例。Collapse 恢复原 Focus/Other 列与 splitter 比例，不以重建组件实现恢复。显隐、关闭、交换和窄宽情况下保留可达的恢复入口，并按原协议撤销已失效的 capture/菜单/AX 目标；不得把原容器身份替换成一个新的聚合容器。具体状态转换复用既有 PresentationState，并对照 Swift Tekes 的实际 expand/collapse 实现。

Window switch、内容图标与 dropdown 的 hover 必须沿原输入链到达控件并可见；不在 AppHost 单独补颜色。Textbox 的插入点、鼠标 IBeam 和焦点边框由 #245 单独核对：Gold7 G2 微对比、G3 精细且清晰的焦点边界、G5 输入保护分别留代码与实际体验证据。统一 Theme 契约继续作为状态色值真源。

验收只覆盖本次受影响路径：选择组互斥与身份、普通 icon/dropdown 布局、hover/leave、展开收起/显示隐藏/单项关闭、splitter恢复与旧输入目标失效；复用未变输入和原生证据，保留真实失败，不启动 VoiceOver。


## 2026-11-10: Window Toolbar partition correction (#229)

Window Toolbar keeps the actual Nav / Focus / Others column allocations. A Container's host=Window presents its title, navigation and business commands in its current column partition, never in a global mixed command pool. The partition follows splitter bounds, visibility and Focus exchange while original Source/command ownership stays unchanged. Host=Container continues to use its declared internal edge. Others single-instance icons and family icon menus start at the leading edge of the Others partition, to the right of its splitter; their measured width reserves space from business commands in that same partition. Hidden-column restore actions remain reachable at a visible partition edge.

Switcher selection uses the existing fill-only style without vertical separators. Family dropdown decoration uses original icon content and an icon arrow. Close remains the original transparent X at the closable Container's internal top-right; Top business and Close share one chrome height and continuous background. Other edges retain only the established natural close-height top strip, without a new rail or patched color values.

## 2026-11-10：Toolbar host 与容器内部方向（#229）

Container 始终拥有自己的 Toolbar 内容、业务状态与动作来源；`host` 只决定唯一呈现位置。`window` 将声明放入窗口统一 Toolbar 的对应 Container 分区；`container` 在该 Container 内部呈现，并选择 `top`、`bottom`、`leading` 或 `trailing`。内部默认 `top`；top/bottom 横排，leading/trailing 真实竖排，不能只把横排条塞入狭窄 Rect。

类型契约采用 `ContainerToolbarHost::Window` 或 `ContainerToolbarHost::Container { position: ContainerToolbarPosition }`，避免 window 与无意义内部方向组合。未显式声明 host 的路径一律默认 Window，包括隐藏后保留按钮的目标；只有目标显式选择 Container 才采用内部方向，内部方向默认 Top。Window 分区继续与各 Container 的实际列空间对齐，不合并成全窗业务池。依赖原内部顶部呈现的已有消费者／夹具须显式迁移为 Container/Top，保留原验收的适用范围，新 Window 模式另验。

关闭是可关闭 Container 自己右上角的透明 ×，独立于内容切换组及业务 Toolbar 的 host/方向；不向不可关闭的 Focus/NavBar 或聚合 List 增加关闭语义。顶部业务条与关闭共用一次高度，右侧业务条为关闭留出顶部命中；其它方向用关闭按钮自然高度的最小顶部 chrome 保护正文，不为单个关闭按钮浪费一整条右侧空 rail，也不虚构标题。业务条若放在 top，leading 使用其业务内容；无业务项不预留业务条空行。正文 padding 继续只由既有一层 token 配置拥有。

以一份四边 Insets、toolbar rect、close rect 和 content rect 同时决定 Local 绘制／clip、Process Layout 尺寸及输入坐标；`content_frame()` 保持正文真源，`content_point()` 沿原原点转换。不增加 IPC，不重复扣 Window 条和内部条。原 Source 保留 instance/module/pid/generation，每项只在一个位置有效。`retained_buttons` 仅在原 owner 隐藏时使用显式目标，呈现服从该目标 Toolbar 的 host：默认或显式 Window 汇入统一条，显式 Container 才进入目标内部声明的位置；保留配置本身不隐式创建内部 Top。不在原位置和替代位置重复挂载，重绑时通过原稳定角色身份撤销旧 capture/AX，不修改进程 generation。

共享 ToolbarRegion 使用默认 Horizontal 的 `axis`，经原 ToolbarContent/ToolbarGroup/ToolbarControl 同步测量、布局、overflow、键盘及菜单定位；`ToolbarControl::set_axis` 供原声明在 mount 前测量真实侧栏宽度。新配置字段不等于功能通过：Window 聚合和四边呈现、单次原 owner 派发、正文原点/尺寸/点击、关闭不重叠、旧捕获退休都要有受影响回归及精确候选原生证据。

用户已说明本次没有 CU2；后续原生使用唯一 gpt-6.1-terra/medium 子代理的官方 Computer Use，分列为官方自动原生操作。已通过且条件不变的旧 CU2 证据保留原程序身份，不重新签为官方输入。本节定义已确认的接口与布局契约；源码、原生验收和公开交付状态分别记录在 #229 与权威清单 T50/T51。

## 2026-11-10：内容组切换对照 Swift Tekes（#229）

用户已明确参照项目是 Swift 版 Tekes。实际源码为 `Tekes/AppShell/MainWindowToolbar.swift`：`PanelTabFamily` 按内容类型声明分组；`makePanelNavigation` 组装普通图标和 family 下拉图标；`fill` 在菜单打开时读取实例并勾选当前项；`selectedPanelNavigationIdentifier` 将同一个当前实例映射回互斥图标组。参照源的精确提交、SHA 与摘录保存在协调输出 `outputs/tabs-overflow-20261110/TEKES-SWIFT-REFERENCE.json`，这是源码对照而非 Swift App 的新原生验收。

单实例内容显示图标，原标题作为 helper／悬停提示。多实例内容类型显示图标加下拉，原实例标题作为菜单项，当前实例行带选中标记。多实例能力由类型分组声明决定，即使目前只有一个实例仍保持下拉；不根据当前数量猜类型。整组使用唯一当前内容身份保持互斥；仅打开或取消菜单不改变当前内容。Swift 对多个切换项使用 selectOne、仅一个切换项使用 momentary，这不改变原内容实例的身份。

TekesUI 沿原 Container 和 Others 呈现配置声明通用图标／分组，复用共享 Button、MenuContent 和 ToolbarGroup；原 PresentationState 是唯一内容选择来源。单项和菜单选择都交回原容器身份，不重建内容、重绑导航或复制编辑状态。保留关闭当前普通容器的透明 ×、List 不可整体关闭、Focus/Others 交换及原 IME 屏障。主题状态、图标尺寸和间距沿既有 token；不照搬 Tekes 的业务类型或另造控件绘制。 共享 Button 公开最小的 selection-fill-only 与 transparent-chrome 配置，委托原模型／透明绘制路径；前者供互斥图标及菜单 trigger 使用，后者供当前项透明 × 使用，默认行为不变。

原宽度可达性要求继续有效：图标组仍放不下时，优先显示当前组，隐藏组放入末尾“…”；隐藏多实例组通过原子菜单列出实例。关闭、交换、缩窗或菜单取消时撤销旧捕获，所有打开内容仍可达。实现与验收按本批最终候选进行；此前普通文本 Tabs overflow 修订仅为已冻结的未集成历史候选，不代签这套内容组切换。

## 2026-11-09：Gallery 的共享 Settings 与显示偏好契约（#229 / #245 / #56）

Gallery 每次激活 AppHost item 仍创建新的 AppHost 窗口；各窗口保留自己的文档、Container 和导航。Gallery Host 从消费者 TOML 与独立持久目录创建一次 `SettingsOwner::new(SettingsWindow)`，在装配前显式 `.with_display_preferences("general/appearance/appearance", "general/appearance/text-size")` 绑定两个字段，然后每个 AppWindow 使用 `.shared_settings(owner.clone())`。绑定参数是 Schema 归一化后的 `group/item/field` 完整 ID，不是 TOML 局部字段名或持久路径；保持精确匹配，不猜测短名。原 `.settings(SettingsWindow)` API 仍创建单独 owner，默认 Exclusive 模式保持；未绑定显示偏好的旧消费者继续沿原 `.theme` 和窗口内字号契约，不读取 Gallery 偏好。

一个 owner 持设置定义、一个 FileProvider、一个 Settings 子窗口及 Weak 加载票据。请求 Host 持 receiver 和唯一强票据直到 native child 发布完成；它在读取完成前关闭会释放票据，其他窗口可以重新申请。子 Settings Host 不强持 owner，复用原表单、历史 Client 和导航服务，不另开 undo.json writer。Independent Settings 可在发起窗口关闭后继续使用；原 Exclusive 模式沿平台原 present/restore 策略。保存、错误重试、Undo/Redo、无效数字/IME 保护和关闭等待均沿用原 SettingsSurface。

通用 TOML 定义字体大小（50–300%，默认 100%）和主题（System / Light / Dark，默认 System），本批已连接实际显示效果。新 AppHost 在首次显示阶段后台读取有效 Snapshot，不必先打开 Settings；已有 owner 的有效值可直接用于新窗口。保存及 Undo/Redo 只发布原 Client 成功确认的 Snapshot，不在 UI 线程重新读盘，不让尚未保存或无效的编辑影响样式。迟到的初始读取不能覆盖较新已确认 revision。加载值经原 Schema 校验，无效值报错，不静默退回默认值。

显示值和弱 wake 订阅存于一个不引用 owner、provider 或 native child 的小型共享状态。各 Host 保留自己的订阅票据，关闭窗口即释放；成功确认后唤醒仍存活的窗口。Settings 原 Surface 同样保留此显示状态，所以即使最后一个 owner 持有者关闭，Independent Settings 的保存/Undo 仍能修改自身显示，且没有 owner → child → owner 强环。

AppHost 和 Settings 使用现成 `Text::set_body_size` 更新原文字服务，重新排版内容及工具栏，不重建 ContentHost、不重置焦点、组字或编辑。System 跟随各窗口实际收到的 `SystemAppearanceChanged`；显式 Light/Dark 只覆盖 appearance，仍保留系统 contrast。共享侧栏 ListView 使用与 Gallery 导航相同的 `set_navigation_selection_fill_only(true)`，不添加主题特例。绑定范围是 AppHost 的本地文字服务与界面；独立进程组件自行绘制的业务内容仍遵循其既有消费者协议，本批没有扩充进程协议。

Settings 的普通 Tab 使用窗口级顺序：可见且可用的原 Toolbar groups → 可见 sidebar → form，并在这些 scope 整体回绕；Shift-Tab 反向。两个当前原 ContentHost 设置 `set_focus_wrap(false)`，先让当前 scope 处理原事件，只有其真实 `event_bubbled()` 才跨 scope，以原 `enter_focus` 进入目标。新 scope 成功接纳后清除旧 keyboard owner，不用 Cancel 强制迁移，hover 所有权保持独立。原 native `insertTab:` / `insertBacktab:` 与普通 Tab 使用相同协调核心；菜单/modal、组字、非法数值及导航恢复门禁保持原拒绝行为，modified Tab 继续原系统路径。此规则仅适用于 Settings；其他 AppHost/remote Toolbar 的原循环范围保留。实际首击 mouse 未出现菜单仍作为独立未定因项，不以键盘路径修复代签。

`Settings…` 以显式 application placement 贡献到原生应用菜单 About 之后，快捷键 Command-comma；默认 Menu 仍为顶层菜单。焦点切换只替换框架贡献，保留标准 About/Hide/Quit/Window 菜单身份及响应链动作，贡献标题不重命名应用菜单。Settings 自身保留该命令而不强持 owner。

TOML 位于 `crates/tekes-ui-gallery/src/gallery/apphost.settings.toml`；存储根为既有 `TEKESUI_APPHOST_DATA_DIR`，未设置时使用 `~/Library/Application Support/TekesUI Gallery/AppHost`，本配置在 `settings/general/settings.toml`。先前 sample 目录保留、不迁移覆盖，Gallery 自身偏好不读写。这里绑定实际 General 字段，没有通用配置框架或新 apply callback。

验收分列：源级 owner/menu/TOML 与已保存显示值回归、受控构建测试、真实多 AppHost + Settings 保存/Undo/重开/重启及系统外观变化。早先仅 sample 字段的 native 保存/Undo 证据不能代签新显示绑定；本节不是原生、物理 IME、VoiceOver 或发布通过声明。

## 2026-11-09 当前修正：单层正文留白、导航 List 与透明关闭叉

NavBar、Focus、Others 正文到 Toolbar 下缘和相邻 splitter/窗边统一使用 `Spacing::Loose`（16pt）四边留白，且只由一层拥有。Gallery 四个 Container 显式选择零外 padding，原 ScrollContent 通过 token 请求 16pt 内 padding；focus bleed 是这个内距的最小安全值，滚动条只占用已有内距中必要的区域，不再在右侧额外叠一层间距。其他消费者未指定 ContainerOptions.content_padding 时沿用原默认。TextSurface Field 的 `layout.scroll_area.content_padding` 12pt 和 Button/List 标签内距属于控件内部层级，保持原值，不用 pane 16pt 替换。

导航的原 ListView 显式采用 Gallery 左导航的 `ControlSize::RowHeight`（32pt）与 `Spacing::Micro`（2pt）节奏，分组仍由原 Items/Model 和 Stack 组织，保留 Workspace 标题、原 item IDs 和唯一 ScrollArea。普通 List 默认行高/行距不变。

**当前 Tab 关闭样式为透明底的单个 ×**，保留独立关闭命中、键盘和 AX 语义；此项按用户 2026-11-09 决定取代下节历史“圆圈内叉”。不得再把圆圈要求作为当前验收条件。相关实现、原生行为和发布证据分别记录，不因文档修订冒称通过。

## 2026-11-08 修正记录：Others 工具栏内 Tabs 与等距正文留白（关闭样式已被上节取代）

用户在恢复旧视觉后的实际试用提出：Others 的 Tabs 是工具栏组成部分，必须在该工具栏同一行完成选择，不另占正文上方一行；关闭 affordance 当时要求使用圆圈内叉（历史，现行样式见 2026-11-09 节），保留独立关闭语义与原命中/键盘/AX。Focus 和 Others 正文使用已有 `Spacing::Loose`（16pt）四边一致留白，Toolbar 高度不算作正文 padding，不在内容树和宿主重复叠加。现有 token 表是尺寸真源，不新增任意常量。此修正明确取代上一批“Tabs在Toolbar下方”和“撤回正文padding”的安排；旧窗口尺寸、边缘图标及导航功能继续保留。通过受影响验证后安装并更新 Dock，源码交付、原生验收及公开 Release 分列。


## 2026-11-08 最新纠偏：保留原视觉与功能，只迁移容器关系

用户明确要求沿用本次 AppHost 修改前满意的布局、尺寸、位置、图标及功能，只将容器关系定义为左侧 Nav 容器与右侧 Focus／Others。结构调整不授权整体视觉重设计；下面同日的新外观计划须按本节重新核对。先以历史源码和证据确认基线，保持现有 WIP，不整批回滚共享组件修复；当前 v0.3.0 候选暂停发布。前进/后退、侧栏显隐等既有功能必须保留，不能用大段文字按钮替代原图标而改变整体空间布局。用户当前试用窗口保持，自动 GUI 操作暂停。


当前实现基线核对为结构改造前 `3718ee9548ff0e20ee19ad3aea7ae0dd6e46e65c`。恢复 1100×640、原窗口位置、1pt 正文分隔及三等分初始几何；隐藏单侧时只扩展 Focus，另一侧保持原宽。原窗口边缘 sidebar/panel 图标和 Back/Forward 位置沿用原 ToolbarWindowEdge；各 Container 仍拥有自己的 Toolbar。普通 Others 的 Toolbar 与系统按钮同行，Tabs 位于其下，正文/输入/进程视口只扣一次 header。Gallery 显式 `nav_fraction=1/3`，默认 None 保持旧固定宽消费者。根正文新增的 16pt 统一 padding 和整条额外背景覆盖撤回；共享输入框、菜单和 Tabs 的独立修复保留。

交付规则：今后对本批说“改好了”须已完成适用验证、安装并将 Dock 指向该准确程序，不能以源码或单独试用程序代替；公开 tag/Release 和本机安装仍分别留收据。

## 2026-11-08 较早计划（下列视觉安排已被上述纠偏取代）

本次用户要求进入下一交付批次，沿用每个 Container 自有 Toolbar，不恢复窗口级工具栏所有权。

- 顶部所有 Toolbar 背景连续填满各自区域及容器交界，不留外 margin、不绘制贯穿工具栏的竖向 divider；系统按钮安全区域及正文区域的 resize 命中能力保留。Toolbar 内部按钮间距与系统避让不以“无外 margin”清零。
- Container 正文四边使用现有 `Spacing::Loose`（16pt）token，测量、绘制、滚动、输入与 AX 几何必须一致；不把 padding 叠加到 Toolbar 外围。
- 容器自有动作提供明确交换入口：others 单容器显示 `Switch to Focus`；focus 显示 `Switch to Others` 并与当前选中的普通 Container Tab 交换；当前为 List 或没有单容器目标时禁用，不自动选择第一项。实例、内容、命令来源保持；NavBar 不参与这两个展示位的交换。
- Others 的普通 Tabs 提供真正关闭操作，关闭与选择分离；关闭后的选择、内容生命周期及恢复规则沿用容器模型，不仅绘制关闭图标。聚合 List 与单容器关闭语义分开，不能借关闭 List 删除所有成员。
- Gallery 是可用的 AppHost 框架/模板，不能退化为静态组件拼盘。必须接回旧版已有前进/后退、NavBar 收起/展开、Others 收起/展开，并通过原导航服务维护历史与当前业务选择。收起后的恢复入口始终可达；恢复原容器实例、选中项及内容状态。导航列表选择、主内容与 Inspector 呈现一致，前进/后退真正恢复上下文。 `navigation_toolbar_follows_focus` 为显式选项，默认关闭保持既有消费者；Gallery不启用该选项：Back/Forward 绑定 NavContainer；业务 `navigation_targets` 仍绑定稳定文档容器。NavBar 的受控导航请求经过原服务 Push/publication，Back/Forward 的 publication 不再次 Push。
- Gallery AppHost 使用实际共享组件展示职责：分 section 的导航 List、多行主内容及换行、Inspector/Property 表单。使用有限真实示例，不新建演示框架。
- AppHost 各原有内容根及 Toolbar/Tabs 使用现有 SVG 图标服务；正文排版由实际 AppHost Text 渲染器按有限宽度测量和绘制，不用 Gallery 的其他渲染器测试代签。换行布局宽度与 CoreText 尾部空白 advance 分别处理，保留输入系统原有排版契约。
- 共享 TextField focus、菜单行和 Tabs 视觉另按关联组件工作项核对。对照 AppKit，优先去掉无意义逐项描边和重复焦点外框，保留清晰键盘焦点、选择、禁用和高对比度状态。以实际渲染及 token 读取判断原因，不能只换颜色掩盖几何问题。

对应 #229（AppHost/Tabs 接入）与 #245（共享视觉修正）；关闭请求由共享 Tabs 明确提供，AppHost 关闭展示保留 backing 并可恢复。通过后按实现、测试、提交、语义版本 tag、Release 交付。历史证据保留，不把旧版本检查直接改签为新外观通过。


初稿：2026-10-21；容器与工具栏职责修订：2026-11-05。

## 2026-11-07 实施批次（#229，目标 v0.2.0）

用户已授权实现、测试、提交、语义版本标签与正式 Release。下文 2026-11-05 的“仅文档/未授权/尚未实现”描述保留其历史身份；本节说明当前实施范围，不能代替测试或公开交付回执。

本批实现统一 Container 装配、容器自有 Toolbar、可选 NavBar、focus/others、Tabs/List、单容器交换、保留按钮路由，并迁移 Main/Settings 的受影响运行机制。旧水平布局入口可通过适配继续使用同一 Host 和同一容器所有权，不维护两套输入/工具栏状态机。历史文档中的动态插件、沙箱、新 IPC、完整联合发布协议不随本批扩入。

公共入口为 `AppWindow::from_containers(Vec<Container>, PresentationOptions)`；`Container::{Local, Process}`，本地内容由 `LocalContainer` 持有。旧本地类型可暂用 `legacy::Container`。`navigation_toolbar(id)` 指定导航贡献的显示宿主，与业务 `navigation_targets(ids)` 独立。未配置 others 时，装配额外无角色容器会报错；配置 off/empty others 时保留 Open 恢复入口。

本次落实的最小选择：

- 初始 focus、可选 NavBar 及其响应目标均以应用提供的稳定容器 ID 配置；不从数组位置或键盘焦点推断业务订阅。
- others 未配置表示无区域/开关；配置后 off 保留选择、顺序与实例。普通 Tab 不改变 focus；提升单容器与原 focus 交换原 Tab 槽位。
- List 资格由容器显式声明，item 高度由应用配置，外层纵向滚动；顺序与普通 Tabs 分离。首版使用上移/下移操作，不增加拖拽排序或持久化框架。空 List 不展示其 Tab，成员恢复后可重新出现。
- Toolbar 由各运行容器真实持有，Window 只协调位置与路由；无标题且无条目时无工具栏空行。跨宿主保留按钮先撤销旧呈现/瞬时交互，再在显式目标绑定原来源，只保留一个有效入口。
- 不引入新的 AppKit 窗口 flag；保持现有平台窗口机制并验证逐容器安全区域、系统控件、拖动与缩放。VoiceOver 仍属后续版本。
- 旧本地 `Container` 改为 `LocalContainer`，新的 `Container` 是统一装配概念；必要兼容入口明确标注，`module_runtime::ProcessContainer` 不改名或协议。

当前进度与有效验收清单见 [#229](https://github.com/TekesApps/TekesUI/issues/229)。测试需覆盖四种布局组合、超过三容器、状态/进程身份保持、工具栏来源与单次执行、过期交互拒绝、Main/Settings 和原生几何；已有条件未变的证据复用。源码、构建、原生行为、公开 Release 与消费方采用分别记录。

<a id="container-owned-toolbar"></a>
## 2026-11-05 确认：Window 直接承载 Container

**用户已确认设计方向；本轮仅文档与 Issue，不修改代码。** 本节及修订后的第 3、4 节定义目标；第 3 节进一步明确 [可选 NavBar + focus / others 范式](#apphost-focus-others)。日期化原型与验收记录保留原身份。跟踪 [#229](https://github.com/TekesApps/TekesUI/issues/229)，规划入口 [#56](https://github.com/TekesApps/TekesUI/issues/56)。旧 [#14](https://github.com/TekesApps/TekesUI/issues/14)、[#69](https://github.com/TekesApps/TekesUI/issues/69)、[#70](https://github.com/TekesApps/TekesUI/issues/70) 已关闭，不因新设计重开，也不代表新结构已实现。

### 关系与命名

```text
Window / AppWindow
└─ Container（窗口直接装配）
   ├─ Toolbar（可选；标题可作为其中的内容）
   └─ Content

Container 的承载方式：
├─ LocalContainer   → 当前进程的 ContentHost / 内容树
└─ ProcessContainer → 独立进程提供内容与业务动作
```

`Container` 是统一容器概念；`LocalContainer` / `ProcessContainer` 是两种承载方式，不是再套一层分区。两者都有可选标题、可选 Toolbar 和 Content。概念关系不要求 Rust 继承；正式类型签名、枚举/适配形式和兼容迁移在实施前落实。

| 当前名称 | 目标名称/职责 | 边界 |
|---|---|---|
| `app_host::Column` | `Container`：统一装配类型 | 不引入窗口分区 Column 或 Pane |
| `app_host::Container`（仅本地） | `LocalContainer` | 保留 ContentHost/内容树 |
| `app_host::ProcessContainer` | `ProcessContainer` | 复用既有进程运行器 |
| `AppWindow::from_columns` | `AppWindow::from_containers` | 目标名称，尚非可调用 API |
| 窗口级 `ToolbarHost` | 各容器自有的共享 `Toolbar` 组件实例 | 调整真实所有权，不是只改名字 |
| `window.rs` 内部 `Panel` | 容器运行状态记录 | 不成为公共分区层；内部名字实施时确定 |

当前 `module_runtime::ProcessContainer` 是内容树中的进程适配组件，与 `app_host::ProcessContainer` 装配声明不是同一 Rust 类型。实施时说明复用/衔接关系，不做全库字符串替换。`ContentHost`、`PanelContent`、`SplitViewContent` 与 SplitView 内部表示两侧的 `Pane` 保持各自职责，不在本轮改名范围。

### 职责与所有权

| 对象 | 负责 | 边界 |
|---|---|---|
| Window / AppWindow | 容器排列、尺寸、分隔条、显隐；原生窗口行为与安全区域；跨容器输入路由/焦点顺序；保留按钮的目标显示位置 | 不集中拥有所有 Toolbar 条目，不复制各 Toolbar/ContentHost 的菜单、hover/press、焦点状态机 |
| Container | 自己的 Toolbar 实例/生命周期、可选标题、Content、内部测量与视口、事件协调 | 不复制业务状态、不管理其他容器的工具栏 |
| Toolbar | 本区域内标题/按钮/已支持内容的布局、交互、焦点、帮助、溢出和语义 | 复用原组件能力，不决定窗口布局 |
| Content / ContentHost | 内容树与既有布局、输入、绘制契约 | 不接管窗口装配或业务真源 |
| 业务模块 / ProcessComponent | 命令、enabled/checked 等状态、文档或导航历史 | 不随按钮显示位置改变业务所有者 |

Toolbar 归 ProcessContainer 不等于必须在子进程绘制。窗口侧容器可呈现子进程声明，动作回到原业务所有者。各容器复用同一共享 Toolbar 和 Content 输入机制；不复制交互实现，也不另建全窗口 ToolbarHost。

### 隐藏时保留按钮

按钮可声明“所属容器隐藏后保留显示”，默认随容器隐藏；以下明确最小行为边界，不冻结字段名、目标选择 API 或 IPC 编码。

1. 默认随容器隐藏；仅明确声明保留的按钮可显示到其他可见容器的 Toolbar。首版不迁移任意编辑器、组字内容、整组或完整 Toolbar。
2. Window 按装配配置选择可见且有 Toolbar 的目标容器；具体目标映射/API 在实施前落实，当前不规定自动寻找“第一个可见容器”。容器之间不互相寻找/搬运控件，不增加通用优先级或路由引擎；目标不可用时走第 5 条恢复入口。
3. 原声明容器保留按钮身份与生命周期，原业务所有者保留命令/状态/进程来源；目标 Toolbar 承接当前位置的布局和交互。只保留一个有效呈现/激活入口，不克隆另一份可写状态。
4. 原容器恢复可见时按钮回到原 Toolbar。沿既有机制取消旧按压/capture、处理菜单/焦点并更新 AX；旧位置 release 不能激活新位置目标。稳定身份不承诺保留失效的瞬时交互。
5. 宽度不足走目标 Toolbar 既有溢出。没有可见且有 Toolbar 的目标，或恢复按钮无法保持可达时，通过窗口菜单/快捷键恢复；不自动补空 Toolbar 或虚构容器，全部容器隐藏也应可恢复。
6. 隐藏不等于关闭/销毁/进程重启。原所有者结束清理其条目；进程失效不得执行旧命令。保留显示不绕过命令条件、几何要求或隐藏内容的输入禁用。

针对保留按钮，Window 只决定目标位置与分配区域；目标 Toolbar 管理本地交互。Window 仍协调跨容器输入路由、Tab/焦点移动与原生拖动交接，实际焦点/capture/菜单状态沿原组件宿主维护，不另建集中式工具栏或焦点状态机。

### 可选标题与原生窗口边界

各 Container 自己决定是否显示标题、是否有 Toolbar。标题可作为 Toolbar 内容；仅标题也可使用该组件。无标题且无 Toolbar 时不预留空行，不强制全窗口 Toolbar 等高。

目标不要求独立可见的原生标题栏，也不由 Window 强制绘制公共标题行。系统窗口标题元数据、关闭/最小化/缩放、系统窗口按钮、拖动、resize、全屏与安全区域仍归 Window/平台。具体平台实现须另验，本轮不选择 AppKit flag 或声称原生效果通过。

Window 提供容器实际区域及与系统控件相交的保留区域；Container 自己测量 Toolbar、计算 Content 视口。布局/绘制/命中/原生语义使用一致几何，不重复扣标题栏高度，不覆盖系统控件。

### 实施与验收边界

只读源码基线为 `aecd20e3254afff77895ecc7cbe73ac68b9887bd`：代码仍使用 Column、本地 Container、from_columns 和窗口级 ToolbarHost。新名字、所有权、按钮属性与标题栏行为均未实现/未验。后续按 #229 落实有界接口与兼容迁移任务，再实施；旧 #69/#70 通过记录不代替新结构验收。

后续检查覆盖本地/进程/混合容器、无标题/无 Toolbar、按钮隐藏/恢复/无目标/溢出、单次命令与原 owner、Main/Settings/导航及平台窗口行为。复用未受影响的证据，补受影响的交互、进程、GPU/原生检查；文档通过不等于功能通过。

本轮不扩展任意 docking、自动合并 Toolbar、通用跨容器控件迁移、动态插件或新进程协议。第 3 节的 focus/others 左右布局及多容器 Tabs/List 是用户新增的设计目标；既有一至三列源码限制不是新范式的永久数量上限，也不能因文档更新声称这些能力已实现。后文历史协议的 panel_id、Panel/列及验收夹具名称保留来源含义，不构成新的公共分区层。

## 既有设计与交付记录（按原日期保留）

状态：设计基线与实验性基础通路原型并行；接口名称为职责示意，未冻结 Rust API 或 IPC 格式。原型验证记录见 `docs/handoffs/app-host-native-prototype-20261021.md`，不表示下列全部契约已实现或验收。

2026-10-26 已交付架构核对：本地 main `e39bfaa1785dbedd9672081a8877e27047155885` 的 app-host/app-protocol/module-runtime 是同仓实验性宿主集成层，基础 controls/text/scene/render 不反向依赖它们。当前 PanelModule/固定进程通路不等于本文完整 ProcessContainer、插件系统或权限沙箱。普通 UI 库消费者不必采用 AppHost；选择该能力的消费者仍须按 #48 原 AC 验收真实 IPC、公开版本、输入/IME/AX、资源和生命周期。现状与目标的共同索引见 [架构与平台边界](../platform-boundary.md)，本次不修改下文既有功能契约或启动新的实现。

本次修订纳入 advisor 对 `503e3ac` 的只读审查结论及后续讨论，并补齐对 `f99bf9e` 复审提出的几何身份、切换终态和文本/几何独立发布规则。整体架构可进入受控原型；公共接口冻结前必须验证这些契约。以下约定是设计要求，不表示技术原型或原生体验已经通过验证。

## 当前 Settings 基础交付（2026-10-27，#70/#136）

本地 main 基线 `a6e9debdbfdaf1142ee5f70720070fdac2dba3f6` 接收现有 `tekes-ui-settings` 的 Schema/Group/Item/Field、typed Draft/Submission 和单写者 FileProvider。仅解析、校验、草稿和本地文件持久化，不依赖 Controls/AppHost/平台；原 TOML 类型及未知键保留，late receipt 只退休原批次，不清除新编辑。临时文件 fsync/rename/目录同步、revision 检查、原 commit 查询/去重沿既有实现。Provider 仅承诺串行本地最近未退休提交，不宣称完整远端崩溃恢复协议。

原10项model/store测试与all-target strict在最终接收差量上通过，锁文件使用原registry版本并经 `--locked --offline` 验证。证据为 SSD `artifacts/issue-136/settings70-foundation-maina6e9deb-intake/round01`；测试临时文件与构建输出均沿原SSD规则。该批不改模型/store原正文，原UI测试保留待后续接收。

共享UI追加接收（基线 `07f69be7750879425696433880d59678ae43c671`）：SettingsForm/Sidebar复用真实Button、TextField、Switch、Menu、NumberField、Scroll和List/ListView；保留编辑mount、无自动保存底栏、错误/Retry、help说明与固定header。原6项form/sidebar、2项number及1项row测试与all-target strict通过，证据SSD `artifacts/issue-136/settings70-ui-main07f69be-intake/round01`。基础model/store输入未变，不重复执行原10测试。这里的有限CPU行为不是完整Theme/GPU/native Settings验收；Sidebar兼容适配不等于全List根已接收。

配置UndoRedo基础追加接收（验证基线95573a7）：原journal/module、真实Worker与完整demo验证入口已接入固定child registry。原3项journal测试、AppHost strict与真实子进程验证通过，覆盖未写prepare、丢confirm后重启、保存后的Undo/Redo及旧registration/window/session拒绝。证据SSD `artifacts/issue-136/settings70-undo-main95573a7-intake/`；两次demo锁文件拒绝保留原身份，最终按原锁内离线metadata修正精确feature依赖，未改变版本或重跑有效journal检查。

SettingsRow 焦点接收补充（本地 main `5992b2dc6c327d8fd96d2fd6adaa40c7db6ca8ed`）：仅在有效 arrange 且原子节点身份/顺序仍匹配时声明现有 reveal 平移能力；measure、lifecycle 和节点替换使该能力失效。Host 对未知布局的拒绝保持。原 Row 平移/失效检查、真实 Form 焦点与未知布局反例、原非法数字/marked 关闭函数回归及相关 strict 已通过；证据 SSD `artifacts/issue-136/settings70-row-reveal-main9a431c2-intake/round01`。真实 Form 检查运行于记录的 Window 候选，不能据此称完整 Settings Window 已交付。

Settings List 本地输入适配已以 `1fd83df7ef73990310a45c74c8a1d76095765487` 接收：原 Scroll Host 内的 pointer 正向、未知 owner 拒绝，以及矩形内但曲线外的捕获 Move/Up 反例通过；原选择与键盘焦点保持，Settings/AppHost strict 通过。该轮暴露的导航 AX publication 失效保留为原失败，后续窗口批次按下述只读准入和先派发后清理修复。证据 SSD `artifacts/issue-136/settings70-navigation-main5992b2d-intake/round03`。

本次本地窗口批次沿原 SettingsSurface、后台 Client 和配置 Undo 协调器装配两列 Settings 窗口。跨 Host 焦点转移前只读检查数值和组字状态，实际侧栏或 Back/Forward 请求发出前才调用原字段提交器；拒绝不替换表单、不写导航历史。原 publication 必须先成功派发，再清理另一 Host 焦点；侧栏 Back 关闭也必须在旧编辑焦点退休前执行输入准入。已收集的合法字段差量仍经原 Draft 自动保存调度，错误提示单独保留。发布阶段只投影已准入的 Item，原 navigation busy 门禁冻结期间输入。List 兼容适配必须兑现原 Host 的几何能力，包括裁剪区域外的捕获事件，不能依靠直接调组件绕过准入。

本次窗口源码已完成有限接收验证：真实侧栏 pointer/key/AX、同份 publication 准入前后有效、成功后的旧焦点退休和原 List 键盘所有权、Back/Forward 拒绝与纠正后可用、原 Back 的 pointer/key/AX Focus/Press 及关闭保存屏障通过；Settings/AppHost strict 通过。证据 SSD `artifacts/issue-136/settings70-navigation-main5992b2d-intake/round04`、`round06`；原 partial autosave、Row/close、List 和 IPC/迁移证据保持其源码身份。Back 测试遗漏原动作消费步骤的失败与修正保留，未改变生产契约或删除断言。

有限原生 Settings 验收追加：仅既有 `--window-acceptance` 加 `TEKESUI_SETTINGS_WINDOW_ACCEPTANCE=1` 启用私有 `settings-window-only` 驱动；正常产品入口和无 env 的旧 Main23 路径保持。真实 Main 模块及原 Settings 命令均保留，子 Host 自己等待导航、有效 AX publication 与启用输入框，再经原输入提交、自动保存、Toolbar Undo IPC 和原关闭路径执行。Exclusive 显示→保存110→Undo回100/实际字段100%→Back关闭/父窗及焦点恢复通过；Independent 同二进制无截图旁路重放也通过，其此前截图已实际检查。证据 SSD `artifacts/issue-70/native-window-main113c0f9/exclusive-round05`、`independent-round06`、`independent-round07-no-capture`。截图只证明实际看到的窗口内容，不替代完整像素验收。

原失败保留：旧 Main23 `exclusive-round03` 50秒超时，无首帧/READY；本 scope 的 Main resize/panel-frame 为 not_run，不转签。scope04 异步字段未就绪的驱动误报已修；Independent 首次在关闭前失活，焦点终态失败，同程序重放成功不抹除该失败，也不推定具体失活来源。该 Main 首帧失败已由后续真实诊断定位，旧失败保留，后续修正及范围见下。

Main 冷启动修正（2026-10-27，验证基线 `477baed` + `window.rs` 精确差量）：首次几何更新早于进程 panel 启动，Layout 被未启动门禁丢弃；启动后几何相同，不会再次派发，工作进程因此一直等待 stdin。可见 panel 现在先启动、再提交首次几何；隐藏 panel、local Container、协议、场景和原门禁不变。AppHost all-target strict 及原 Main23 真实验收通过，实际收到23帧并达到原 READY，截图已人工核对内容；READY 后验证包装器终止自己的进程，不将其视为产品退出生命周期验收。相同最终源码下，Exclusive 与 Independent 均经真实 Main、原 Settings 命令、原输入及 Undo 路径通过显示→保存110→Undo100/实际字段100%→关闭与父窗焦点恢复。仅因启动顺序输入发生变化补验这两条集成链，无关有效回归不重跑。证据 SSD `artifacts/issue-70/main-cold-start-fix/{run02,settings-exclusive-run03,settings-independent-run03}`，诊断失败见 `main-first-frame-diagnostic/run01`；后两条不含新的视觉截图。

#70 原 PR52 样例追加接收（验证基线 `5de0b01` + 冻结差量）：Main 样例以 `AppWindow::from_columns` 装配一/二/三列及 mixed-columns，仍在创建 Settings 或执行 HOME 迁移之前处理固定 worker argv。三列原文档 A/B/C 通过消费者 `NavigationOptions::view_command` 映射到中间列已有命令通道；Host 等对应内容帧与 GPU 提交后才确认导航，Settings 使用 `None` 保持其原表单导航。中性模块新增真实文档切换，同时保留计数 Button、Toolbar、counter revision 和原验证十步/全部断言。demo 锁文件只增加现有 Controls 直接依赖边，版本/校验和未变。

实际验证：Host strict、demo all-feature/all-target strict、原导航/列契约通过；原三模块 IPC/GPU 完整回归及 A/B/C 不同文本正文、计数/Toolbar 不变检查通过。一/二列原 Main23 通过。三列与 mixed 首轮遗漏默认收起左栏的展开前置，进程数量检查失败，记录保留；原生文档路由在该轮已通过。后续有限验收仅在原 `--window-acceptance` 加 `TEKESUI_COLUMNS_WINDOW_ACCEPTANCE=1`、原 Main23 READY 后继续，经真实共享 Toolbar 的键盘路径展开左栏，等待实际内容帧，再验证三进程或两进程+本地 Container。mixed 使用当前窗口 AX publication 的真实 TextField Focus、原输入处理提交文字，重绘后检查文本和焦点保留。两组合通过，真实截图已核对；不宣称物理键盘/IME/VoiceOver 或完整退出生命周期验收。证据 SSD `artifacts/issue-70/pr52-samples-main5de0b01/` 与 `full-columns-run02/`；后续只因 opt-in 验收驱动变化重跑 Host/demo strict 和受影响三列/mixed，原一/二列及模块 IPC/GPU 保持各自源码身份。

#70 仍开放：AC1/2 的 API、Schema、平台设计和兼容依赖已有明确交付，本地所有共享库均为本仓库已提交源码路径；开发路径与公开不可变 Git 交付仍分列，不凭 path 字样推导需要额外发布所有 crate。AC3/5 尚缺最终 PR 独立 checkout 的精确依赖与原范围接收、同源 CI 和公开交付记录，本批主工作树结果不代签独立 checkout。AC4 的 #48/Toolbar 一般进程、多窗口及 VoiceOver 长期目标保留，物理 IME/完整 VoiceOver 未执行且不是本票明文独立全量门禁。唯一负责人 root；下一交付是最终提交的独立源码/依赖接收，随后明确需要授权的 PR/远端更新及同源 CI。

原 #70 AC3 的最终 PR 独立 checkout、精确公开依赖和同源 CI 尚未完成。AC4 要求关联 #48/Toolbar 剩余项并明确本 PR 不承诺的一般远端、多窗口与 VoiceOver 范围；物理 IME/完整 VoiceOver 并非 #70 明文规定的独立全量关闭门禁。本批只证明程序驱动输入与语义 AX，未宣称物理输入/IME/VoiceOver通过；#48 等原能力目标继续保留，不因本次范围说明被删除。完整既有Settings设计和历史原型保留在Gallery Root的 `docs/design/apphost-settings.md` 与 `apphost-settings-prototype-20261022.md`，后续接收沿原契约，不把历史实现转签为当前main完成。


平台Settings前置接收（2026-10-27，#70）：真实WindowHandle、普通/Exclusive子窗口、可见窗口恢复、key-window菜单/焦点分发和dirty退出通知已接入；重复present保留首次恢复名单与key，按身份去重。原47平台契约、strict及AppHost接口检查通过；定向真实AppKit回归验证实际激活/key前提、四窗口重复present、关闭恢复两可见窗口且不显示原隐藏者、恢复原焦点。证据SSD `artifacts/issue-136/settings70-window-main3e37ff7-intake/round02` 的exclusive-native及平台各round；早期harness缺正常生命周期而失败，记录保留。此有限原生结果不是完整Settings自动保存/Independent/物理IME/VoiceOver验收。


## 1. 目标与范围

维护一套共享 App Host 源码，按构建 profile 预定模块组合、平台能力和发行资源，构建并打包成不同 target apps。Window 直接装配 Container，使用共享布局和 splitter 分配区域；各 Container 自行组织可选 Toolbar 与 Content，可采用 LocalContainer 或 ProcessContainer 承载方式。进程内容复用带 View ProcessComponent，模块提供内容/工具项、状态和动作。另可固定装配无 View 的导航 ProcessComponent：它独占 nav.json 和导航历史，Host 只执行导航，成功后回报，由导航模块提交历史。

这是一套公共框架与共享启动入口，不是生成多份 App 源码的代码模板。消费产品通过装配配置及必要的 host adapter 接入，不复制 Host 的状态机和 View。

AppHost、模块运行器及其接口在 TekesUI 仓库内分层维护；仓库共享不表示底层 controls/text/scene/render 反向依赖 Host。消费者提供用途型原生模块、固定装配配置和产品资源/能力适配，由共享构建工具产出目标 App。

本规范只讨论通用 Host、模块边界、容器布局、进程管理、渲染合成和输入输出契约；三列是既有布局与验收场景，不是公共容器类型；不列举具体业务模块，不定义业务 Kit 的内部结构或业务模型。

首版不提供用户安装插件、运行时发现任意模块、动态库加载、产品切换、模块热替换或热卸载。消费者通过普通 Rust crate 和模块工厂在编译期固定装配；首版将 Host、runtime 和选定模块链接为同一目标 App 可执行文件。Host 启动同一可执行文件的独立子进程，通过固定模块 ID 选择工厂；每个子进程只创建自己的模块实例，Host 不调用业务 View 工厂。崩溃后重新启动同一预定模块不属于动态插件替换。

去掉 WASM 后不再有虚拟机、线性内存、fuel 或 guest/host ABI。原生模块是可信的随 App 发布代码；独立进程提供故障边界，不自动提供文件、网络或凭证的安全沙箱。现有原型继承父进程权限，不声称已实现能力隔离或 CPU/内存配额。未来权限限制由平台进程沙箱及 Host adapter 实现并单独验收。

## 2. 分层与所有权

| 层 | 负责 | 边界 |
|---|---|---|
| TekesUI | 通用组件、布局、文本、输入、焦点、无障碍、动画与绘制；ProcessComponent 基础进程能力及带 View 的 ProcessContainer 适配 | 不依赖 TekesAppHost、业务类型或模块目录；无 View 不启动 GPU/共享纹理 |
| TekesAppHost | 固定装配、Window/Container 边界、容器布局、各容器 Toolbar 接入、窗口输入协调、导航执行/回报 | 不建立全窗口 ToolbarHost；不实现业务 View、复制进程机制或导航历史；不直接写 nav.json |
| 模块运行进程 | 公共 runtime 管理生命周期、IPC、命令/事件与可选 View/平台/GPU 输出 | 带 View 的 Panel 独立运行；无 View 模块不伪造容器/帧；不直接控制整个窗口 |
| 无 View 导航模块 | 导航历史、游标、前进/后退 toolbar 声明、nav.json 持久化与恢复 | 独立 ProcessComponent；Host 成功执行后才提交历史 |
| content 原生模块 | 按用途组织的业务界面与逻辑、toolbar 注册、事件/请求输出 | 公共接口是模块级接口，不是逐个导出底层组件 |
| target app / 外部能力提供方 | 固定装配、产品资源、业务权威状态、业务协调与能力适配 | 通过通用契约注入；Host 不包含产品条件分支 |

Kit 是源码和能力的复用边界，原生模块是按用途组织的运行单元，二者不要求一一对应。多个原生模块可以复用同一 Kit 源码；一个模块内部也可组合多个组件。本规范不要求模块内部再为每个组件启动进程。

可复用 Agent 界面及其呈现状态遵守 TekesClientKit 的单一所有权约定。未来使用其他模块组织名称或交付形式，不能据此在 Host 中复制 Agent 的 run/transcript/composer、设置、任务或线程列表实现。需要改变既有所有权时，先另行明确迁移方案。本设计不修改 Kernel。

### 2.1 仓库目录与依赖方向

```text
crates/
  tekes-ui-app-protocol/       # 纯数据：消息、身份/代次、toolbar 描述
  tekes-ui-module-runtime/     # 复用的模块/进程/IPC；ProcessComponent 基础及可选 View 运行能力
  tekes-ui-app-host/           # 装配、窗口、三列、导航执行与父级呈现约束
  tekes-ui-render/             # 通用绘制；可选 macOS shared-surface 适配
  tekes-ui-platform-macos/     # 原生窗口 chrome、平台输入
  tekes-ui-controls/          # toolbar、Panel、splitter 等通用控件
examples/
  panel-module-demo/          # 中性 native 模块消费者；工厂/状态/组件在子进程内
  app-host-demo/              # 固定 profile 的消费 App 入口，无 Host 代码副本
```

`protocol` 不依赖 UI/GPU/Host；runtime 依赖 protocol 与底层 TekesUI；Host 依赖 protocol、runtime 的启动/模块定义接口及 TekesUI，不依赖任何业务模块实现。业务模块依赖 runtime 接口与业务 Kit；消费 App 同时依赖 Host 和选定模块，完成固定装配。底层 controls/text/scene/render/platform 不反向依赖 Host/runtime。共享 IOSurface 的通用 Metal 包装留在 render；进程、IPC 和共享资源生命周期复用公共实现，Host 只组合与调用。ProcessComponent / ProcessContainer 的具体 API 与落点由公共抽取确定，不能将以上既有目录直接视为新契约已实现。

当前原型直接传递本地 `Scene`/`GlyphAtlas` 给 runtime；typed toolbar 描述跨 IPC 发送。首版实现 Layout、指针、Command、Cancel、viewport 与帧序号；后文完整几何代次、capture、IME 和联合切换仍是设计契约，不能误认为本次拆分已实现。

<a id="apphost-focus-others"></a>
## 3. AppHost 范式：可选 NavBar + focus / others（2026-11-08 纠正）

```text
Window
┌──────────────────┬────────────────────────┬────────────────────────┐
│ NavBar Container │ focus：单个 Container  │ others：Tabs / List     │
│ 可选             │ 可选 Toolbar + Content │ 可选 Toolbar + Content │
│                  │                        │                        │
└──────────────────┴────────────────────────┴────────────────────────┘
List Tab 内的单个容器仍纵向排列，可上下调序。
```

本节当前布局以 2026-11-08 用户纠正为准：focus 在左、others 在右，按宽度分配；可选 NavBar 在更左侧。原 2026-11-05 上下堆叠示意及 v0.2.0/v0.2.1 实现为历史设计/交付记录，不再作为当前布局。顶部 Container 从窗口内容坐标 y=0 分配，Toolbar 与系统红绿灯所在原生行对齐，不再整体预扣标题栏高度后又扣一行 Toolbar。仅与红绿灯相交的 Toolbar 应用原生左侧 safe area；对应无 Toolbar 内容仅避让系统保留高度，其他无 Toolbar 容器不预留空行。List 内部 Toolbar 使用普通行。


这是一套 AppHost 装配范式，不是每个基础 Window/组件消费者都必须采用的布局。Window 仍直接组织容器；NavBar 是左侧容器的用途，focus/others 是展示角色与布局状态，不新增 Column、Pane、FocusContainer 或 OthersContainer 类型。每个实际 Container 仍可采用本地或进程承载方式，并拥有自己的可选 Toolbar/标题和 Content。

### 3.1 启动配置与运行时状态

| 部分 | 启动配置 | 运行时语义 |
|---|---|---|
| 左侧 NavBar Container | 可配置一个，也可以完全不配置 | 提供应用自己的导航内容；缺省不自动生成导航容器或占位 |
| focus | 应用从右侧可用容器中指定初始主展示者 | 当前展示一个容器，可与 others 中的单个容器互换 |
| others | 可配置，也可完全没有；配置时可给初始显隐状态 | 开启时展示右侧 Tabs；关闭时隐藏，保留已打开容器、选择与顺序 |
| 普通 Tab | 应用装配可打开的容器 | 打开/切换一个容器；可将该容器与 focus 互换 |
| List Tab | 由容器是否支持 List 展示决定参与集合 | 一个聚合 Tab，纵向展示 others 中所有支持 List 的容器；各 item 可上下调序 |

NavBar 与 others 的配置相互独立，支持“NavBar + focus + others”“focus + others”“NavBar + focus”“仅 focus”。完全未配置 others 与运行时 off 不同：前者没有 others 的打开入口/空 Tab 区，后者可以重新开启。关闭 others 让 focus 使用释放的右侧空间，不等于关闭其容器、文档或进程。应用未配置 NavBar 时，右侧区域使用其释放的空间。

这里的 `focus` 表示**主展示位置**，不是键盘输入焦点。用户可以在 others 中输入，而不自动将该容器提升为 focus。键盘/IME/AX 焦点仍沿原输入协议管理。

### 3.2 focus 与 others 任意单容器互换

用户已明确选择：**focus 可与 others 任意单个容器交换位置，原 focus 回到该容器的原位。** 目标无需是当前选中 Tab，也可以是 List 的一个成员；List 聚合整体不可提升。 例如 focus=A，others 的普通 Tab=B；提升 B 后 focus=B，A 回到 B 原普通 Tab 的位置，不复制 A/B 的业务状态或新建进程。只切换 others 的当前 Tab 不改变 focus；提升为 focus 是另一项操作。

切换位置保留原容器身份与业务状态，Toolbar/标题随容器移动。旧几何的按压/capture/菜单和原输入焦点按既有生命周期处理，不把主展示切换当作无条件转移键盘焦点或业务导航。切换不更改应用为容器配置的导航响应关系。

### 3.3 List 是一个特殊 Tab

List 不替代整个 others 的 Tabs 模式，而是其中一个 Tab。其集合按“当前属于 others，且声明支持 List 展示”形成；支持能力表示同一容器可以被纵向组织，不是把任意复杂内容自动压缩成摘要或复制一份业务 View。普通容器 Tab 与 List item 引用同一稳定容器身份，切换呈现不产生第二份可写状态。

List 中的每个 item 是实际 Container，仍拥有自己的 Toolbar/标题/Content。List 仅组织纵向排列与顺序，不合并 item 的 Toolbar，不接管各容器业务状态。调序只改变列表位置；拖拽、按钮、快捷键等具体交互及顺序持久化策略留待后续讨论，本轮不选择。

用户已明确选择：**List Tab 不能整体成为 focus；只有单个容器可以。** List 的聚合身份不能被当成一个新的 Container。为避免同一容器在 focus 和 List 中重复展示，focus 容器不计入 others 的 List 集合；焦点位置互换后按容器身份重新形成集合。若从 List item 提升单个容器，原 focus 回到 others；其是否进入 List 仍取决于自身 List 支持声明，不支持 List 的容器仍可作为普通 Tab。普通 Tab 和 List 顺序是两种布局状态，不因 item 调序自动改普通 Tab 顺序。

List 为空时的占位/隐藏方式、item 高度和滚动分配、打开/关闭 Tab 的具体交互在实施前细化；不把这些待定细节声称为已确认的产品行为。

### 3.4 导航响应由应用决定

哪些容器响应左侧 NavBar 的选择或上下文变化，由具体应用绑定；可以是 focus、部分 others、多个容器或没有接收者。AppHost 不默认广播给全部容器，也不默认只更新 focus。业务订阅/路由绑定稳定容器身份，不因 Tabs/List 切换或 focus 互换自动改绑。

应用决定业务响应及其是否需要联合切换；Host 只执行获准路由并按原协议维护输入/呈现一致性。未显示或 others off 时是否继续处理业务更新由应用决定，不能把隐藏自动解释为取消业务订阅。NavBar Container 是可选可见 UI，与既有无 View 的导航历史 ProcessComponent 是不同概念；本范式不强制每次左侧选择写入 nav.json，也不强制采用该服务。

### 3.5 通用布局与验收边界

以下容器/输入约束继续适用。旧三列原型是历史验收场景，不再定义此范式的固定槽位或功能上限。

- Window 分配容器区域、分隔条和平台保留区域，系统窗口按钮始终归 Window/平台，不随容器换位或模块结束转移所有权。
- Container 自己计算 Toolbar 与 Content 几何，splitter 或安全区变化时同步更新实际 Content 视口；布局、绘制、输入和 AX 使用同一几何，不重复扣标题栏高度。
- focus/others 的切换与配置按上文执行；移动时 Toolbar 与 Content 一起移动，不销毁模块或迁移业务状态。具体尺寸和分隔条交互实施前确定，不由旧三列的 middle/leading/trailing 规则推导。
- 隐藏不关闭会话或卸载模块，默认保留进程及状态并暂停无用内容帧；业务任务是否继续由业务所有者决定。保留按钮与无目标恢复按[已确认规则](#container-owned-toolbar)。
- 记录稳定容器 ID、顺序、显隐意图、有效宽度与可恢复焦点引用；临时窄窗折叠不覆盖用户显隐意图。多窗口实例不共享一个可变 UI 单例。
- 控件、splitter、系统控件和窗口可拖动空白区各按实际命中区域处理；内容 capture 不得占用原生窗口控件。

后续验收应覆盖四种启动组合、others 未配置与 off/on 的区别、Tab 切换不改 focus、单容器互换保状态、List 能力筛选/顺序及禁止整组提升、导航按应用绑定而不按展示位置广播。当前仅记录设计，不运行上述功能验收；实现仍由 #229 后续有界任务承接。

### Toolbar 测量与内容边界

各 Container 选择是否有 Toolbar/标题，并使用共享 Toolbar 的固有测量、单行布局、溢出和交互能力。无 Toolbar 不留空行；不强制全窗口取最大高度。标题换行按既有单行标签规则处理；窄宽沿原 Toolbar 能力，不宣称任意自定义内容可自动转为菜单。

Container 使用平台约束计算实际视口并按 backing scale 对齐；旧帧仍受原 viewport/几何输入门控。菜单由所属 Toolbar 管理，显示位置改变不改写原命令来源。无独立标题栏的系统按钮、安全区、拖动、全屏和窗口元数据必须另做平台验收，旧同排顶部栏原型不能代签。

ProcessContainer 是显式进程承载方式，不为任意组件增加 `subprocess=true`。ProcessComponent/runtime 提供已有进程基础，Container 负责窗口侧装配；内容树中的 runtime 适配器保留自己的 API 边界。无 View 导航服务不创建虚假内容容器、renderer 或共享纹理；其工具项按第 4 节接入真实容器 Toolbar。详见 [ProcessComponent 与导航契约](process-component-navigation.md)。

## 4. Container 拥有 Toolbar，模块提供命令与状态

各 Container 拥有自己的可选 Toolbar，本地内容或进程模块提供工具项、命令与状态。无 View 导航 ProcessComponent 的 window scope 表示贡献授权及生命周期，不是窗口级 ToolbarHost：装配将贡献接入指定 Container 的 Toolbar，业务所有者仍是原服务，不为其伪造 Content/View。需要在容器隐藏后保持可达的按钮使用前述保留显示规则，导航命令不依赖内容帧。leading/trailing 显隐仍归 Window 布局，不写入导航历史。

```text
content 原生模块 → 注册 toolbar 描述 → 模块进程 / IPC → 所属 Container 的 Toolbar
content 原生模块 ← toolbar action ← 模块进程 / IPC ← 用户操作
content 原生模块 → 新状态 / 描述更新 → Container 更新同一 Toolbar
```

以下 Host 指共享宿主接入/传输边界，不表示 Window 拥有工具栏实例；注册作用域与目标呈现容器是两个维度。职责级契约包括：

| 方向 | 内容 |
|---|---|
| 模块 → Host | 注册/更新 toolbar 项、稳定 item/command ID、顺序与分组、标题、图标引用、提示、enable/checked 等投影 |
| Host → 模块 | action、来源 item、所有者模块实例、贡献作用域及注册代次；View 贡献另有 Panel/View 与适用的上下文/几何代次，窗口导航贡献有 history revision/事务身份 |
| Host → 模块 | toolbar 可用空间或呈现条件；按需协商支持的控件能力 |
| 模块 → Host | 请求完成/失败、业务状态更新及必要的后续操作意图 |

无 View 导航 toolbar 不伪造 content 几何依赖，命令按 history revision、事务和模块实例校验。每个有 View toolbar action 还需声明是否依赖 content 几何。Host 按声明执行第 7.2 节的几何门控，不根据命令名称猜测业务含义；未声明的 action 保守视为依赖几何。声明不依赖几何的 action 仍需通过模块实例、上下文和业务执行条件校验。

toolbar 描述是原生控件的贡献数据，不传递原生 Rust View 对象或执行闭包。首版支持的控件集合、菜单/溢出呈现和增量更新格式在接口设计时明确，不预设任意原生自定义 View 已能跨边界传输。

约束：

1. 所有注册绑定所有者模块实例代次和获准的贡献作用域：`scope=view(window_id, panel_id, view_id)` 绑定真实 Panel/View；`scope=window(window_id)` 绑定窗口，不要求也不允许无 View 模块伪造 Panel/View。注册及动作以所有者、作用域、注册代次和 item/command ID 共同识别，模块不能改写其他所有者或未获准作用域的 toolbar。
2. 一批注册先校验 ID、命令引用和可支持能力，再原子发布；失败显示诊断，不留下半个 toolbar。
3. enable/checked 等业务状态由模块或其外部业务权威源提供，Host 只消费投影。Host 自身布局命令由 Host 的唯一布局状态派生。
4. 操作返回模块后重新校验上下文和执行条件；按钮禁用不代替权限校验。
5. 模块断连时禁用该实例的业务操作；Host 保留列外恢复入口和必要的故障提示。
6. 重启清理旧注册，新实例握手并注册完成后才发布新 toolbar；旧代次消息不能覆盖新状态。
7. 隐藏或移动 Panel 不注销其 View 注册，只改变挂载/可见性；关闭 View 只清理相应 View 作用域贡献。窗口级贡献不随 Panel 显隐、换位或无关 View 关闭而撤销，按所属窗口关闭、所有者实例结束或授权撤销清理；实例失效时清理其全部作用域，旧代次消息不得重建注册。

### 4.1 TekesUI、Host 与模块的职责

| 层 | toolbar / icons 职责 |
|---|---|
| TekesUI | 定义通用 toolbar/item/group 模型、图标资源引用与呈现、尺寸/着色、间距、对齐、伸缩、溢出优先级、交互及无障碍规则；不定义业务命令 |
| Window / AppHost 装配 | 分配容器/平台区域，确定保留按钮目标，连接原业务路由；不集中管理所有工具项和交互 |
| Container | 拥有 Toolbar 实例，接入获准的 View/服务贡献，管理挂载与内部布局，保持原命令来源；复用共享 Toolbar/Content |
| 有/无 View 的 ProcessComponent | 在获准的 View/窗口作用域注册 item/command ID、分组、建议位置、图标、标题/提示、可见性及 enabled/checked 等业务投影 |

TekesUI 的通用行为参考 Apple HIG 与 AppKit 的 toolbar 分组、优先级和空间分配约定，不直接把 NSToolbar 对象或平台私有行为暴露为跨平台协议。View/窗口作用域贡献的模块归属、授权与消息寿命属于公共注册契约；工具栏呈现归 Container，不由 AppKit 自动提供，也不要求窗口级 ToolbarHost。

位置使用 leading/center/trailing、组内顺序、间隔/伸缩和溢出优先级等布局意图，由 TekesUI 根据可用空间与布局方向求解；模块不指定按钮的屏幕绝对坐标。具体控件集合、分组是否整体溢出、中心区避让及最小宽度待 TekesUI 接口设计时确定。

图标使用命名空间化资源引用或受控资源句柄，声明尺寸、模板着色或多色等呈现意图，由 TekesUI 统一解释。Host/运行器负责校验引用、解析随包资源并按资源代次缓存；缺失资源使用可诊断的回退，不允许模块借图标引用任意读取文件。平台图标映射、缩放及辅助功能标签遵循同一通用规则。

“跟随”分别约定：Toolbar 随所属 Container 换位，由 Window 改变容器位置；跟随上下文/选区变化由模块提供投影；随滚动固定、隐藏或浮动等通用呈现行为由 TekesUI 提供，Host 按声明策略配置。首版默认 toolbar 位于本列 content 上方，不跟随 content 滚出；其他行为不视为已经支持。

toolbar 的通用布局规则不能替代第 9.1 节的上下文联合发布门控。

## 5. ProcessContainer 的独立模块进程（原三进程场景）

本节仅描述进程承载方式；不要求 LocalContainer 创建子进程。图中的 Panel A/B/C 是历史原型名称，对应目标中的三个 ProcessContainer。

```text
App Host（native TekesUI）
  ├── Panel A：toolbar + content 合成区
  │       ↕ IPC / 共享 GPU 资源
  │   模块进程 A：runtime + 预定原生模块 + GPU/平台桥接
  ├── Panel B：toolbar + content 合成区
  │       ↕ IPC / 共享 GPU 资源
  │   模块进程 B：runtime + 预定原生模块 + GPU/平台桥接
  └── Panel C：toolbar + content 合成区
          ↕ IPC / 共享 GPU 资源
      模块进程 C：runtime + 预定原生模块 + GPU/平台桥接
```

导航扩展装配在上述三个 View 模块之外，再启动一个无 Panel/View 的导航 ProcessComponent，仅通过服务/toolbar IPC 通信，不分配共享画面。

TekesUI 的公共 ProcessComponent/runtime 创建、握手、监控和回收进程，AppHost 不复制监管器。首版使用同一可执行文件的 Host/worker 两种入口，以编译期白名单模块 ID 选择工厂；独立进程不要求每个模块各维护一份运行器源码。每个实例的状态和事件循环均在子进程本地创建；仅协商有 View 的实例才创建 GPU device 和渲染资源。导航进程不占一个 Panel，也不等待帧就绪。

建议生命周期：`configured → starting → handshaking → ready → stopping → stopped`；失败进入 `failed`。ready 仅表示接口就绪，首个可显示帧和业务数据就绪分别记录。握手核对协议版本、模块身份及能力。只有带 View 的实例再核对渲染后端、GPU adapter 身份、共享资源/同步机制及资源格式，并验证一次实际资源导入；只报告同为 Metal 或格式相同不算兼容性通过。

退出、异常、启动失败或无响应时，只将对应 Panel 标为不可用，其他 Panel 和窗口外壳继续运行。Host 不在 UI 线程同步等待模块。超时不能直接等价于业务请求失败；可能有外部副作用的请求需查询权威结果，不能自动重放。

重启时生成新的模块实例代次，撤销旧输入路由、toolbar 注册、挂起请求和 GPU 资源引用。是否支持恢复草稿/任务由模块及外部服务声明；进程隔离不保证进程内状态在崩溃后保留。重启策略设置退避与次数限制，不形成无限重启循环。

进程隔离降低局部故障影响，但不承诺抵御共享 GPU 设备丢失或平台服务故障；这些故障需要统一诊断与恢复路径。

## 6. 原生模块接口与进程通信

当前带 View 原型模块实现 `PanelModule` Rust trait；消费 App 用 `ModuleDefinition { id, create }` 注册编译期固定工厂，并通过 `AppProfile` 启动共享 Host。runtime 在对应子进程内调用模块，模块直接使用 TekesUI 组件构造本地 Scene/字形资源并由本地 wgpu 绘制。这些 Rust 接口只在同一进程、同一构建内使用，不是稳定动态库 ABI。Host 与模块进程之间始终走版本化 IPC 数据和操作系统资源句柄，不能把 trait object 或 View 对象当成跨进程接口。

| 契约组 | 通用职责 |
|---|---|
| 描述与协商 | 模块/协议版本、可选 View、服务/toolbar 能力、能力需求和支持的输出方式 |
| View 生命周期 | 创建、配置、挂载、可见性、尺寸、关闭准备、释放 |
| 环境 | 版本化主题/平台环境快照、能力协商、更新确认与呈现失效 |
| toolbar | 由有/无 View 模块注册/更新本列或窗口级贡献及命令，按所有者接收 action |
| 输入 | 指针、键盘、焦点、组字、拖放、剪贴板协作 |
| 业务 input/output | 上下文投影、事件、请求、结果、错误、取消与订阅 |
| 渲染输出 | 共享画面资源、frame-ready、同步信息、失效/重绘需求与资源回收 |
| 平台语义 | 输入法光标/候选窗定位、无障碍树、光标形状等 |
| 关闭与恢复 | 保存/关闭准备、结果、检查点及可恢复能力声明 |

所有跨边界值都使用明确的序列化数据或受控资源句柄，不传递原生 Rust 指针、闭包或进程内对象。高层模块接口不要求公开 TekesUI 的每个组件；内部使用 TekesUI 所需的 GPU/平台导入是运行器基础设施契约，与业务模块对 App 的公共接口分开。

Rust 引用、对象和 GPU device/texture 只在本进程作用域内有效。runtime 维护实例代次、IPC 资源 ID 与本地对象的映射，并明确拥有、借用、归还、释放和断连清理。异步工作必须持有合法所有权或显式延长寿命；GPU 在途资源必须等待使用完成后释放。Mach right 等 OS 能力通过受控传输获取，不能把另一个进程中的数值句柄当成本地可用对象。

消息信封至少关联协议版本、window/workspace 作用域、模块实例代次；带 View 的消息另关联 Panel/View ID，无 View 消息不得伪造这些身份；请求另有 request ID，上下文和视口更新另有对应 revision。规定消息顺序、背压、取消/超时、断连、资源释放和不支持能力的错误行为。高频输入和属性更新可按语义合并，点击、文本提交等不可随意丢弃。

模块身份来自构建装配及握手，不能仅凭模块自报获得权限。Host 依据 profile 授予最小所需的文件、网络、凭证等能力，资源引用和请求经过实例/作用域校验。

## 7. 渲染与合成

本节及 7.1/7.2 的画面、视口与内容输入契约只适用于协商有 View 的 ProcessComponent。无 View 导航组件使用服务/toolbar/导航事务消息，不创建渲染资源或伪造视口；导航命令按 history revision、事务和实例身份校验。

content 在模块运行进程中完成状态更新、布局与内容绘制，Host 接收渲染结果，不在 Host 中重建业务组件树。Host 仍使用 native TekesUI 绘制窗口、三列容器和 toolbar，并完成最终合成。

模块直接编译和调用 native TekesUI，不经过 VM、组件 ABI、线性内存复制或 Scene JSON 编解码。Scene 与字形图集保持在模块进程本地，IPC 只传画面资源、同步元数据和事件/请求。移除虚拟机减少一层桥接，但布局成本、进程调度与 GPU 同步仍需实测，不能据此宣称拖动无延迟。

首选内容输出为共享 GPU surface/texture 及帧同步信息，而非每帧读回 CPU 的像素数组。wgpu 负责适合 GPU 的工作；跨进程纹理导入/导出由平台适配层提供，普通 wgpu Texture 对象不直接跨进程传递。macOS 可验证 Metal/IOSurface 路径；其他平台分别验证，不承诺已有统一零拷贝实现。

Host 选择可共享资源的 GPU adapter，并向模块运行器提供可匹配的平台身份；各进程在本地创建兼容 device 并正确导入底层共享资源，不跨进程复用 device 指针或普通纹理编号。macOS 共享纹理路径要求同一 GPU。导入失败、GPU 不匹配或平台不支持时，明确报告该列不可用；如引入复制等回退，必须在 profile/能力协商中声明并单独验收，不静默声称零拷贝。设备变化或丢失后重新握手并使旧资源代次失效。

资源协议需覆盖：尺寸、逻辑点/像素比例、像素格式、色彩空间、透明度约定、模块实例/资源代次、`viewport_revision`、`context_revision`、`environment_revision`、`geometry_environment_revision`、`layout_revision`、帧序号、生产完成信号、消费完成/归还、丢帧与设备丢失。几何身份见第 7.2 节。资源重建时在 GPU 使用完成后才释放；Host 不采样仍在写入的纹理，模块不覆盖 Host 尚在使用的缓冲。

Host 按窗口刷新时机合成，模块按内容失效和动画需要生产帧。采用有界缓冲和背压；模块暂未产生新帧时保留上一有效帧，不阻塞其他列。同一模块/View/上下文且环境兼容的连续 resize 可单调呈现较旧但已完成的视口预览：不得回退到早于已显示视口的帧，不接收超过当前请求视口的帧，帧 seq 必须前进。不能因为始终尚未追上最新请求而丢弃全部中间帧，造成拖动期间停画。当前原型按旧逻辑尺寸裁剪，不拉伸文字。

resize 预览不是可交互几何的发布。新点击与依赖当前几何的命令必须等待匹配视口；已有 capture 会话的连续输入和终止/清理仍遵守下文会话契约。该预览例外不得跨上下文、主题或不兼容环境，不得绕过联合切换、文本确认与几何发布规则。当前原型仅实现 viewport/seq 门控，完整代次协议仍待实现。

消费 App 的三个 Panel 为平面工作区，无额外卡片边框、圆角或阴影；这属于 AppHost 消费布局，不修改 TekesUI 缺省 Style。两个 splitter 位置独立，拖动只改变相邻两列，无关列的边界和视口保持 fixed；达到最小列宽时停止，不推动另一条 splitter。

原型内容必须使用 TekesUI 的 TextContent / TextFieldContent 文本组件及默认语义角色，平台适配器只接入现有字体/排版/字形服务。不得通过显示专用、缺少 caret 数据的手工正文绕过实际组件路径来宣称文本性能。三列使用不同的中文、英文、Unicode 混排长文，验收检查真实组件重排、裁剪和默认样式；编辑/IME 的完整验收另行完成。

独立进程带来隔离和 CPU 工作并行，不保证 CPU 总负载降低或 GPU 并行收益。减少开销依靠避免像素读回、复用资源、增量更新、合批和隐藏暂停；布局、业务逻辑和提交仍消耗 CPU。60 Hz/120 Hz 的帧间隔约为 16.7/8.3 ms，这些只是测量条件，不是当前实现承诺。

### 7.1 splitter 拖动时的 resize 与帧生产

```text
拖动 splitter
  → Host 即时更新 Panel 几何与原生 toolbar 布局
  → 对尺寸变化的 content 合并并异步发送最新视口
  → 模块重新布局、GPU 绘制并返回带 revision 的帧
  → Host 验证帧与当前呈现目标，合成显示
```

- 只通知视口发生变化的模块；纯位置变化由 Host 更新坐标变换，不要求内容重绘。toolbar 高度变化也纳入 content 视口更新。
- resize 采用有界、最新值优先的待处理状态，可按显示节奏合并，不能为每个鼠标事件无限排队。已经提交的 GPU 工作不假设能取消；过时结果不进入新呈现状态，但仍按协议归还资源。
- 每个视口更新携带单调 revision、逻辑尺寸、像素尺寸/缩放及 live-resize 状态。模块处理最新目标、失效布局并按需生产帧；结束拖动时必须交付最终视口，模块确认应用并输出匹配帧后才视为内容已收敛。
- Host 不等待模块 ACK 或 GPU 帧再移动 splitter。控制消息与输入有明确顺序，只有 resize/允许合并的更新可覆盖；点击、文本提交、捕获释放等事件不可因此丢失或重排。
- 新视口的第一帧未到时，默认保留旧帧在原逻辑比例下裁剪，新增区域显示占位底色，并标明正在调整；可配置的临时缩放策略必须显式声明。旧帧只能作为临时呈现，不伪装成新视口已完成布局。
- Host 只接受与当前目标或明确保留的过渡状态相符的帧；新视口帧提交后，晚到的旧视口帧不能覆盖。缓冲有上限，支持跳过中间尺寸，避免停止拖动后继续播放积压帧。

### 7.2 几何代次与过渡中的输入同步

几何有效性不能只比较视口尺寸。每份可交互呈现使用几何身份 `module_instance + View ID + context_revision + viewport_revision + geometry_environment_revision + layout_revision`：

- `viewport_revision` 由 Host 分配，表示尺寸/缩放等视口目标；相同尺寸下的上下文变化仍有独立 `context_revision`。
- `environment_revision` 标记完整环境；其中字体、语言方向、缩放及其他影响布局的变化，由 Host 同时推进 `geometry_environment_revision`，并按同一更新批次下发。纯颜色变化不推进几何环境代次，不必阻止交互；不能证明不影响布局的变化保守按影响几何处理。
- `layout_revision` 由模块在本实例内分配，表示实际命中布局快照。内部内容重排、滚动或其他改变命中映射的变化也更新该值，不限于 resize；不影响命中的纯绘制动画无需增加布局代次。模块通过布局失效/准备通知、帧和语义快照发布布局代次及其依赖。

Host 分别记录目标几何依赖和当前已显示帧的几何身份/变换。收到 Host 发起的布局变更或模块布局失效通知后，将对应几何标为待就绪；只有目标依赖匹配且帧与必要几何快照共同发布后，才恢复新的命中激活和依赖呈现几何的 toolbar 操作。相同 viewport 下字体/方向变化同样触发该门控，不能等到新帧到达才承认几何失效；已经建立的 capture 会话按下述独立规则继续，不因该门控自动取消。

content 新命中输入关联完整几何身份及已显示帧序号；坐标先变换到该帧对应的逻辑空间。纯 Panel 移位只改变 Host 的窗口坐标变换，不要求模块重排；事件使用采集时的变换映射。模块核对输入携带的布局身份，若已不接受该命中快照，则拒绝执行并请求刷新，不将旧坐标用于新布局；这也覆盖模块内部重排通知尚在 IPC 途中的竞态。有效 capture 下的 move/release 不重新命中，不使用这条规则拒绝已捕获目标。

首版保守策略：目标几何待就绪或与已显示几何不匹配时，不发起新的 content 指针激活或拖动，不缓存点击供新布局完成后重放；Host splitter、窗口操作继续响应。未来若允许旧帧上的新命中持续交互，必须证明模块保留匹配的命中几何与 revision，否则拒绝不匹配输入。这不限制下述已有 capture 会话的持续投递。

#### 新命中与已有 capture 会话

新的 pointer-down/点击/拖动起点必须通过当前呈现几何校验。模块处理一个已接受的起始事件后，可请求捕获该指针；Host 确认原始按压仍活跃、目标和模块实例/上下文仍有效，建立带唯一 `capture_id` 的路由。捕获还绑定下述 `press_id`、View/目标身份、pointer ID、起始事件序号及 context revision；Host 分配的会话 ID 在本实例内不复用，重启时全部失效。建立捕获不能依靠模块自报绕过起始事件校验。

#### 按压身份与迟到 capture 请求

Host 从接收 pointer-down 起记录独立 `press_id`，以 pointer ID、按钮/接触身份及起始事件序号区分每次按压，并记录其是否通过几何校验、接收 View、模块实例和上下文。新的按压不能复用上一轮身份；pointer ID 相同不代表同一手势。

- capture 请求必须引用起始事件及 press_id。grant 只允许对应按压仍活跃且初始 down 已接受的请求；Host 将 grant 与 up/cancel 串行裁定。up/cancel 先发生则拒绝 grant；grant 先发生则后续 up/cancel 终止该 capture，不等待模块收到确认才处理松手。
- capture 尚待确认时，Host 为已接受的按压保留临时路由，把 move/up/cancel 连同 press_id、输入序号和约定的局部坐标按序送到原接收 View；不重新命中或转发给指针经过的其他 Panel。模块以原按压状态处理这些事件，不能把“尚无 capture_id”视为丢弃终止事件的理由。这条临时路由只服务已有按压，不绕过新命中门控。
- grant、拒绝和该按压的输入使用可比较的顺序边界；临时路由升级为 capture 时，尚在途的事件仍按同一 press_id 处理，不重复投递、不重放已处理 move，也不能把先到的终止状态覆盖成活跃。模块收到迟到的 grant 时，若本地该按压已结束，执行幂等释放/确认，不恢复手势。
- up/cancel、原生失焦或输入流失效、上下文切换、隐藏及断连等终止条件，即使发生在 grant 之前，也立即终止临时按压并撤销其路由，记录终止序号。任何不匹配当前活跃 press_id 的后续请求均被拒绝；终止记录可以有界回收，但已结束身份不得再次获准。
- 拒绝/超时通知携带 press_id、请求 ID 和原因，模块清理该次临时按压及待确认捕获；不清理或取消紧接着开始的另一按压。准备捕获有有限期限，模块失联时 Host 独立清理本地状态，不无限保留临时路由；终止与拒绝均不自动回滚已发生的业务动作。

因此“down → up → 迟到 capture 请求 → 下一次 down”与“down → 下一次按压已开始 → 旧 capture 请求”都只能拒绝旧请求，不能建立缺少 release 的捕获或误占用新手势。

捕获坐标空间在建立时声明并确认。首版使用 content 视口局部的逻辑坐标，允许超出视口边界；Host 按采集时窗口到视口的变换传递坐标/序号，不裁剪到命中区域、不转换到不断变化的文档滚动坐标。模块负责当前滚动/缩放与业务拖动的内部换算。capture_id 和坐标空间独立于最新帧的 layout revision，不要求每次 move 等待新画面。

| 情况 | 投递与生命周期规则 |
|---|---|
| 新点击或新拖动 | 通过已显示几何与目标依赖校验；几何待就绪时不启动、不缓存重放 |
| 有效 capture 的 move/release | 按 capture_id、原目标和有序输入流投递，不重新命中；保留布局/帧标签供诊断，不以旧 layout revision 为由拒绝捕获事件 |
| 手势自身引发滚动、画布平移、自动滚动选择或重排 | 模块推进布局代次并可标记来源 capture_id；Host 仅门控新命中，不取消有效捕获，持续 move/release 继续作用于原手势目标 |
| 同一上下文内的其他重排 | 不仅凭 layout revision 改变就取消；模块保证原目标及捕获坐标语义仍有效时继续，否则显式报告 capture 失效 |
| 上下文切换、目标销毁、Panel 隐藏、实例断连/重启或原生系统撤销捕获 | 显式取消原会话，清理路由与按压状态；不把原手势重定向到新目标 |
| 外部 resize、缩放或 Panel 换位等坐标变更 | 若协议不能维持已声明坐标语义，先有序取消；如支持继续，须明确确认新的变换边界并给事件标记其变换代次，不猜测坐标含义 |

只有会话身份/目标/上下文或坐标语义失效才取消，不把“新帧尚未到达”视为捕获失效。模块主动释放捕获和 Host 接收 release/cancel 都进入唯一终态；清理按 capture_id 幂等，迟到事件不能复活旧会话。release/cancel 走终止路径，即使原布局失效也必须清理，不被普通命中门控或 move 合并覆盖。模块无响应/退出时 Host 先撤销本地捕获路由并按故障协议清理，不能等待模块 ACK 而让窗口持续保持捕获。

已聚焦文本的键盘/组字输入不因 resize 被静默丢弃，按同一文本会话和第 8.1 节协议继续；这些文本编辑不依赖指针命中几何，仍须校验焦点、实例和上下文。候选窗只使用与已显示呈现及所查询文本范围匹配的已发布几何；缺失时按第 8.1 节返回允许的无结果。toolbar 纯布局调整不自动使业务命令失效；注册为依赖 content 几何的命令在上述门控期间禁用，不重放为新布局下的操作；上下文切换另遵守第 9.1 节。

splitter 自身顺滑和 content 实时重排顺滑分别测量。跨进程通信、CPU 文本换行/测量、GPU 提交和合成错过显示时机都可能增加延迟，不能因使用 wgpu 就承诺无卡顿。

## 8. 输入、焦点与平台能力

窗口系统事件首先到达 Host。Host 根据 Panel 几何与当前内容视口转换坐标，路由到对应模块；内容内部的命中测试、交互状态和业务处理由模块处理。toolbar 是 Host 原生控件，其交互通过注册 action 回到模块。

Host 是窗口级焦点和输入目标的协调者，模块管理内容内部焦点。指针捕获、拖动、Tab 导航、快捷键冲突与 toolbar/content 焦点转移使用显式协议。换位或隐藏时结束或转移必要输入状态，不把旧坐标事件送入新几何；已有 capture 是否继续按第 7.2 节的会话和坐标规则判断，不能仅因内部重排而取消。

渲染结果之外必须传递输入法与无障碍语义。Host 连接原生文本输入、候选窗位置、选区查询、剪贴板及无障碍入口；模块提供内容语义、文本范围、光标几何和 action。组字不等同于已提交业务文本，不能在换焦点时随意丢弃。

这些桥接属于首版可用性边界，需要真实窗口验证。仅能显示纹理不能视为可用原生 View 已完成。

### 8.1 原生同步查询的本地协议快照

AppKit 的 NSTextInputClient 需要同步返回 selected/marked range、文本片段和 first rect 等查询，不能逐次同步 IPC 等待模块。Host 维护当前焦点文本客户端的有版本协议快照，包含必要文本范围、选区、组字和光标几何，关联模块实例、文本会话、上下文、文本 revision 与视口/几何 revision。

Host 文本代理按原生输入协议更新必要的临时 marked-text 状态，并按序发送编辑意图；同一原生调用链内先设置 marked text 再立即查询时，应能从本地协议状态得到一致回答。模块确认业务修改、发布权威文本投影；Host 按输入序号确认/校正临时协议状态，不维护可独立修改的业务草稿或在每次快照到达时覆盖未确认组字。

文本协议状态与呈现几何使用独立发布通道，不能要求整份快照等待 GPU 帧，也不能让文本确认隐式发布新布局坐标：

| 通道 | 身份与推进条件 | Host 查询规则 |
|---|---|---|
| 文本确认/协议状态 | 模块实例、文本会话、context revision、输入确认序号、text revision；按序确认并叠加尚未确认的临时组字 | selected/marked range 及可用文本片段从一致的本地协议状态同步返回，不因 GPU 帧延迟停住输入确认 |
| 呈现几何 | 完整几何身份、帧序号、对应 text revision、范围/节点 ID 与光标/字符区域 | 先缓存，仅在对应帧和布局提交可见后发布；first rect、点到文本范围及无障碍 bounds 等查询必须校验匹配关系 |

Resize 或环境重排期间，V2 的文本确认可先推进，而 V2 的光标/字符几何必须等匹配画面提交。旧画面的几何只有在仍匹配当前所查询文本范围及其版本时才能使用；文本已变化且没有匹配几何时，返回平台允许的无结果/暂不可用并异步刷新，不能仅通过缩放/平移把新换行几何套到旧画面。过时几何不覆盖已提交的新几何，尚未确认的组字状态也不被旧文本确认覆盖。

上下文、焦点或文本会话变化使两条通道的旧消息同时失效；同一会话内文本确认与几何可以独立推进。第 9.1 节切换所需的“输入语义就绪”包括目标文本客户端身份及明确的几何可用/不可用状态，不要求尚不可用的几何同步查询等待 GPU 或 IPC。无障碍文本/语义与 bounds 同样区分逻辑更新和呈现几何，不发布与可见上下文不符的可操作节点。

焦点交接、content/toolbar 切换、实例重启或上下文失效时，明确完成/取消组字及快照失效顺序。快照不足时使用平台协议允许的无结果/不可用响应并异步请求刷新，不编造范围或阻塞 UI。模块卡顿/崩溃不能让旧文本身份继续接受输入。

无障碍同步查询使用类似的 Host 本地版本化语义投影，区分节点查询与异步 action；语义树、文本与几何在可见上下文切换时一并失效/发布。原型必须覆盖中文组字、候选窗定位、局部文本查询、辅助功能、模块延迟及断连。

### 8.2 主题与平台环境

| 层 | 环境职责 |
|---|---|
| TekesUI | 定义主题 token/语义、环境快照模型和组件响应规则，负责组件样式、重排/重绘与动画降级；不选择产品主题 |
| AppHost | 合并 target app 主题配置、窗口环境和系统设置，发布唯一的有效环境快照，同时应用到原生 shell 并通过模块协议下发 |
| content 原生模块 / 运行器 | 协商环境契约与主题资源版本，消费快照并应用到模块内 TekesUI，确认 revision、使相关布局/缓存失效并输出匹配帧 |

环境至少涵盖主题身份及 token/schema 版本、有效明暗外观、强调色、对比度/辅助功能、减少动态效果、语言/布局方向、字体环境与缩放信息。token 资源可随包固定或按受控数据传递，具体编码待冻结；仅传 theme 名称而不校验 token 兼容性不足以保证一致。环境与视口中重叠的缩放信息通过同一更新批次关联，不能分别采用不兼容值。

Host 为每次有效环境更新分配 `environment_revision`。模块应使用同一 TekesUI 语义环境，不能静默忽略必需字段或另选主题；不支持的必需能力明确报错。业务内容可以使用契约允许的语义扩展，不任意改写 shell 主题。

Host 原生界面可即时应用系统设置；异步 content 在匹配新环境的帧到达前可短暂保留旧画面或显示等待占位，不能将旧环境帧标记为新环境已完成。新环境已提交后拒绝旧 revision 覆盖。视觉环境更新不强制与每个业务状态更新形成事务；语言方向/字体/缩放改变布局时，即使视口尺寸不变也推进几何环境代次，执行第 7.1、7.2 节的几何和输入门控，并按第 8.1 节分别更新文本确认与呈现几何。纯颜色变化不因 environment revision 不同就阻止指针交互。

## 9. 数据所有权与跨 Panel 协调

Host 维护布局、已装配组件的呈现/执行路由和实际内容上下文。公共 ProcessComponent 承担进程与资源生命周期；模块/外部业务提供方维护业务数据、草稿、checked/selection、业务任务及文档。导航模块独占导航历史、游标和 nav.json，Host 不维护可独立修改的副本。Host 可以消费只读投影，不复制可独立修改的业务状态机。

跨 Panel 交互由 target app 声明的业务协调适配器通过通用路由处理，Host 不解析具体业务字段。每项业务状态有一个权威来源；多个模块共享会话时，通过明确的服务/协议共享，而非依赖各自链接进模块进程 的 Kit 单例。

在 [focus/others 范式](#apphost-focus-others) 中，左侧 NavBar 的响应集合由应用绑定稳定容器身份；下述联合切换只覆盖应用选定的参与者。focus/others/List 的呈现位置不能替应用决定订阅或隐式扩大事务范围。

上下文切换使用一致 revision：新上下文未提交前保留一致旧画面或显示等待态，不能用新内容画面配旧上下文命令。toolbar 的 enable/checked、content 操作和异步结果关联同一目标与代次；拒绝晚到事件和陈旧回执。

### 9.0 导航模块与 nav.json 提交

导航模块是无 View 的 ProcessComponent，持久化稳定 route/params、唯一 entry ID、cursor、revision 与提交 ID。空历史 cursor 为 null，非空必须指向有效条目。push 成功截断前进分支再追加，back/forward 成功仅移动游标；失败/取消不改历史。

每个 history 串行处理导航事务：导航模块准备唯一 navigation_id 并记录 pending → AppHost 执行 route 切换 → 目标上下文按 9.1 联合发布规则成功提交后，Host 返回明确成功 → 导航模块原子保存 nav.json 后发布 HistoryCommitted。消息已发送或模块已接受不等于导航成功，Host 执行成功也不等于历史已落盘。

替换前失败与 `replacement-durability-unknown` 必须区分：rename 已发生时不能声称旧 JSON 未变，也不能反向覆盖来伪造未提交。新 Host 必须通过导航模块串行的恢复屏障，先收敛旧提交、接收带 revision 的 RestoreOffer，再完成 RecoveryApplied/Ready 双方确认；基线已变则重新恢复，不能提前开放导航。未 HistoryCommitted/未对账完成的执行 receipt 不因导航模块重启而丢失。

保存失败、未知执行结果、重复/迟到回执和两侧重启的对账规则，以及 UTF-8 nav.json 示例、单写入者和原子替换契约，完整定义在 [ProcessComponent、无 View 导航模块与 nav.json](process-component-navigation.md)。该文是本设计的组成部分；旧原型 V1 尚未实现这些能力。

### 9.1 上下文、toolbar 与内容帧的联合发布

每次业务上下文切换分配 `context_revision`，toolbar 描述/业务状态、内容帧及平台语义投影都标记其目标 revision；协议同时保留独立的 toolbar 更新 revision 和帧序号，不能只依靠消息到达顺序。

Host 准备新上下文时缓存对应输出，在匹配的新内容帧、toolbar 状态和必要输入语义就绪后，于同一呈现提交中切换。等待期间显示一致旧快照或明确等待态，并禁用受切换影响的业务命令/内容激活；未确认的新上下文请求不派发。这样即使模块已经切换内部上下文，也不会让用户看着旧画面操作新目标。

action 携带发起时已提交的 context revision，模块核对目标后处理，旧动作/异步回执不能作用到新目标。跨 Panel 的业务协调器明确哪些 Panel 参与同一切换；Host 在必要参与者就绪前维持等待态，不能由固定路由推断所有 Panel 自动形成事务。

#### 联合切换的终态与收敛

每次切换使用独立 `transition_id`、目标 context revision、参与者集合和准备期限。Host 串行裁定提交与取消的先后；切换状态为 `preparing → committed / cancelled / superseded / failed`，每次只能进入一个终态，终态后晚到消息不能重新提交该切换。

| 情况 | 必须执行的规则 |
|---|---|
| 全部必要参与者就绪 | 复核实例代次、参与者和目标依赖后提交；就绪期间发生 resize/几何环境变化则重新取得匹配输出，过期就绪不批准新布局 |
| 用户取消准备 | 提交前进入 cancelled，通知参与者停止该切换准备并释放暂存资源；模块不必能取消已经提交的 GPU 工作，完成后仍归还资源 |
| 新切换取代旧切换 | 旧切换进入 superseded，清理其订阅/暂存状态，生成新的 ID 与目标代次；不能让旧 ACK 或帧满足新切换 |
| 必要参与者异常、拒绝或超过准备期限 | 由业务协调器在有界时间内选择中止为 failed，或显式批准提交带错误占位的结果；Host 不无限等待、不静默删除必要参与者。协调器无响应时按 profile 的有限期限默认失败 |
| 已经提交后收到取消 | 返回已提交状态；如需返回原上下文，发起新的切换，不将已经可见的提交假报回滚 |

准备取消/失败只终止呈现切换，不承诺回滚业务操作，也不自动丢弃草稿或重放有副作用的请求。模块可能已切到新内部上下文，因此 Host 不能仅把旧截图和旧 toolbar 重新启用就视为恢复：须以新的恢复切换/代次重新绑定权威上下文，并等待模块确认和匹配呈现；失败则显示可恢复错误状态。保留旧图像时受影响命令仍禁用，错误占位仅提供 Host 恢复操作；不依赖失败参与者的其他 Panel 可继续交互。

参与者在 preparing 时声明就绪要求下限，允许因重新显示等原因提高要求，但不能静默降低。可见参与者必须有匹配帧、toolbar 与输入语义；隐藏参与者默认采用逻辑就绪，只准备目标业务绑定、toolbar 投影及不激活的输入语义，允许延后画面。它重新显示时必须先取得匹配当前几何/上下文的首帧再接受几何交互，不能闪现旧上下文。若业务协调器明确要求隐藏参与者也准备画面，则为该切换授予一次受限准备帧，不把暂停普通呈现解释为永不响应准备。

准备期间显隐变化需更新 readiness 校验：隐藏列重新显示时，在未提交切换中提升为可见就绪要求；隐藏行为不自动降低已经声明的必要要求。模块失败或重启使其旧就绪记录失效，仍按失败/取代规则决定终态。这样隐藏、失败和重启均有显式收敛路径。

联合发布门控仅用于语义上下文切换；同一上下文内的普通动画帧、独立 toolbar 状态更新不要求每次都捆绑提交。若旧画面保留策略与新视口或新环境冲突，使用明确等待占位，不绕过视口/环境有效性校验。

## 10. 隐藏、关闭、退出与恢复

导航模块独立于 Panel 显隐。正常退出须先收敛执行中的导航，依据真实执行终态完成/取消并保存已提交历史，再停止导航进程；超时强制退出不得将未知意图写成成功历史，pending 由下次启动按导航恢复契约对账。

隐藏 Panel 仅影响呈现，不保存、放弃或关闭业务会话。正常关闭 View、窗口或 App 时，Host 协调模块及外部状态所有者的关闭准备，再释放实例和进程。关闭能够表达允许、取消、等待与失败；等待时禁止重复请求。

涉及未保存内容时，先由业务提供方完成必要的编辑协调，明确选择保存/放弃/取消。保存回执关联会话、请求、目标和保存版本；保存期间有新修改时仍保留 dirty 状态。关闭批准绑定版本、实例代次和受影响引用，执行前复核，变化则重新准备；取消或保存失败不能提前销毁 View 或删除草稿。

多 View/进程引用同一会话时，按业务会话去重协调，不把进程退出等同于会话关闭。UI 输入缓冲、业务草稿、已提交模型、持久化记录是不同状态，需分别声明支持的恢复层级。

布局记录只存 Panel ID、位置、宽度、显隐和有效恢复引用；不包含业务真值或隐式权限。Host 按当前固定 profile 校验记录，缺失或不匹配模块显示诊断/默认布局，不从布局文件加载任意代码。业务恢复由模块/提供方的持久化能力完成；重新打开已保存文件不能冒充未保存状态恢复。

## 11. 构建、固定装配与发行

每个 target app 的 profile 至少预定：

- 产品身份、名称、图标、Bundle ID、平台资源及权限配置。
- 三个 Panel 的稳定 ID、初始槽位、默认宽度/显隐与 content 模块引用。
- 原生模块 crate 精确版本、锁文件与最终可执行产物摘要、IPC 版本、模块运行器版本及所需平台适配。
- 所需 Host 能力、允许的数据路由和产品业务协调适配器。
- 联合切换的有限准备/决策期限、失败处理默认策略，以及需显式覆盖的隐藏参与者画面准备要求。

构建流程：选择 profile → 校验装配与版本 → 构建共享 Host/运行器及指定模块 → 静态链接选定模块并打包资源 → 按平台要求签名并产出对应 App。模块源码/依赖必须锁定到不可变版本；不能在发行时静默跟随变化的开发目录。

启动流程：读取打包配置 → 验证模块身份/兼容性 → 创建三列与进程 → 按各自固定模块 ID 创建原生实例 → 握手 → content 注册本列 toolbar → 获取内容帧 → 进入可交互状态。单列启动失败不阻塞整个窗口，不将失败列视为 ready。

不同 target apps 可以选择不同用途的模块，但复用同一 Host 和运行器实现；不按 Kit 名称强制划分模块。开发源覆盖必须显式标明；发行依赖使用已发布的精确 pin，并锁定模块摘要和接口版本。

## 12. 最小验证与当前证据

已有位于 `crates/tekes-ui-app-host` 的实验性基础通路与中性原生模块 消费者，验证共享纹理、toolbar 注册和基本原生输入；具体证据及未覆盖项见原型验证记录。下表仍是完整设计的待验收矩阵，基础通路通过不将整项自动标为通过。继续使用中性夹具验证 Host，不将具体业务模块设计混入框架。

| 验证 | 通过条件 | 当前状态 |
|---|---|---|
| 两个 target apps | 同一 Host 源码、不同固定 profile 构建并运行，无产品条件分支 | 待实现/待测 |
| 进程边界 | 每个 Panel 独立 PID/runtime/模块实例；单列异常不停止其他列 | 待实现/待测 |
| 三列布局 | toolbar、Panel、splitter 均为 TekesUI 组件；每列 toolbar + content；换位保持进程、状态与注册；可独立隐藏/恢复/缩放 | 待实现/待测 |
| toolbar 注册 | content 注册、更新、接收 action；失效代次不生效；失败无半注册状态 | 待实现/待测 |
| toolbar / 图标规则 | TekesUI 分组、位置意图、溢出、图标资源与无障碍统一；换位不改变模块归属 | 待实现/待测 |
| 画面通路 | 模块进程绘制、Host 合成；正确同步/归还资源，排查 CPU 像素读回 | 待实现/待测 |
| GPU 兼容 | 验证 adapter 身份与实际导入；不匹配明确失败；设备变化撤销旧资源 | 待实现/待测 |
| 原生资源寿命 | Rust/OS 资源所有权明确；IPC 映射、断连与 GPU 在途资源安全释放 | 待实现/待测 |
| 输入与语义 | 真实窗口下中文组字、候选窗、焦点、捕获、坐标、快捷键与无障碍正确 | 待实现/待测 |
| 同步输入查询 | 同一调用链设置组字后立即查询、本地快照确认/校正、焦点转移和模块卡顿/崩溃正确 | 待实现/待测 |
| 文本/几何独立发布 | 新文本确认先于新画面到达时，组字确认继续；新几何延迟到匹配帧，缺失几何返回允许的无结果；旧确认不覆盖新组字 | 待实现/待测 |
| 联合发布 | 人为延迟/乱序 toolbar 与内容帧，切换时不出现旧画面操作新目标，旧语义同步失效 | 待实现/待测 |
| 切换终态 | 覆盖准备取消、新切换取代、必要模块崩溃/超时、隐藏逻辑就绪/一次准备帧、显隐变更；无无限等待或晚到提交 | 待实现/待测 |
| 环境更新 | 原生 shell 与 content 消费同一环境契约；版本确认、旧帧拒绝、字体/方向变化输入几何同步 | 待实现/待测 |
| 几何代次 | 固定视口下切换字体/RTL 也门控指针和几何命令；纯颜色不阻止交互；模块内部重排拒绝旧命中，release/cancel 仍完成清理 | 待实现/待测 |
| 持续捕获手势 | 拖动滚动条、画布平移、自动滚动文本选择自身推进 layout revision 后，原 capture 仍持续接收 move/release；即使帧延迟且新点击被门控，也不取消原手势或重新命中其他目标 | 待实现/待测 |
| capture 建立竞态 | 快速 down/up、延迟请求/grant、随后同 pointer ID 的下一次按压，以及 grant 前失焦/断连：结束按压不获准捕获，临时 move/up 按序且不丢失，旧请求不干扰新手势 | 待实现/待测 |
| capture 终止与坐标 | 覆盖超出视口、外部 resize/换位的显式继续或取消、上下文切换、目标销毁、隐藏、断连/重启；release/cancel 不被合并丢弃，重复清理幂等且旧事件不复活会话 | 待实现/待测 |
| live resize | 连续拖动 splitter 无无限积压；最终尺寸收敛；旧帧、旧坐标和捕获释放正确，其他列不被阻塞 | 待实现/待测 |
| 数据边界 | 业务状态单一来源，跨 Panel 上下文一致，旧回执不覆盖新状态 | 待实现/待测 |
| 故障与重启 | 超时/崩溃/设备丢失可诊断；有限重启；旧资源和路由安全释放 | 待实现/待测 |
| 关闭与恢复 | 取消/保存失败保留状态；旧保存不批准新版本关闭；恢复按声明层级验证 | 待实现/待测 |
| 性能 | 指定硬件、尺寸、刷新率和负载，测输入到画面延迟及尾延迟、掉帧、各进程 CPU/GPU/内存 | 待实现/待测 |
| 跨仓复用 | 验证 TekesUI/相关共享包、Host 与第二个公共 API 消费者；涉及 Agent UI 时运行所有权检查 | 待实现/待测 |

### 12.1 前置技术验证与完整原型

三列 View 完整原型必须是三个 Panel、三个独立 View 模块进程、三个预定的中性原生 content 模块；加上 AppHost 主进程，共四个应用进程。包含导航的新装配另加一个无 View 导航 ProcessComponent，总共五个应用进程；不得拿其中一个内容进程替代导航进程或添加第四个内容容器。三个原生模块由固定 profile 装配，可以复用夹具源码，但必须在三个独立进程中分别创建实例，不能在一个进程中模拟三列完成验收。不在本文指定业务模块。

| 阶段 | 规模 | 必须证明 |
|---|---|---|
| 前置技术验证 | 一列、一个中性原生模块、两个真实进程（Host + 一个模块进程） | 实际运行原生模块/TekesUI/GPU 绘制、共享纹理与资源同步，跑通 toolbar action 和基础输入；只排除基础技术风险，不代表完整原型完成 |
| 三列 View 完整原型 | 三个 Panel、三个独立模块进程、三个原生模块同时运行 | 三列 toolbar/content 正确归属；换位保留进程/状态；splitter 连续 resize、跨列焦点与输入、上下文联合发布、环境更新及资源背压正确；单模块卡顿/崩溃不阻塞其他列，测量并发负载与延迟 |
| 导航扩展装配 | 三个 View ProcessComponent + 一个无 View 导航 ProcessComponent + Host | 无 View 路径零 GPU/纹理分配；导航 toolbar 不依赖 frame；Host 成功执行后原子提交 nav.json；失败/取消、重复回执、崩溃与恢复不重复/提前提交 |
| 固定装配复用验证 | 同一 Host 源码构建两个 target apps | 分别按固定 profile 链接指定原生模块并打包资源，验证共享框架没有被复制或嵌入产品条件分支 |

先完成单列前置技术验证，再在完整三列中人为延迟上下文/帧消息验证联合发布；在真实窗口验证中文组字及同步查询、持续捕获和迟到 capture 请求；连续拖动 splitter 验证输入/帧代次、积压和最终收敛；模拟单模块退出及 resize 中的资源回收。所有完整原型要求均需三列同时运行时验证，不能用三个独立单列演示替代。性能与兼容性未验证前，不把原型画面、离屏测试或接口定义视为生产验收。

基本交互夹具还需记录 capture_id、输入序号和布局代次：连续拖动引发多次滚动/重排，同时延迟帧返回，验证只有一个捕获会话、move 持续投递且 release 后终止；同一期间的新点击被门控。另模拟上下文切换/崩溃、迟到 move/release 与重复 cancel，确认不会卡住按压、错误重定向或复活已结束手势。

捕获建立夹具另外记录 press_id、请求/grant 序号与终止序号：注入 down 后立即 up、连续两次同 pointer ID 按压、延迟 capture 请求及 grant 通知，分别覆盖 grant 与 up 的两种先后顺序；验证零悬挂捕获、终止不丢失、清理幂等且第二次按压不会收到第一次的清理动作。

live resize 的测试分别记录 splitter 自身帧表现、content 尺寸跟随延迟及尾延迟、掉帧、停止拖动至最终匹配帧的时间、队列/资源上限和各进程 CPU/GPU/内存。分别覆盖轻重布局、不同宽度/缩放和声明支持的刷新率；通过阈值在原型前按硬件与负载设定，不在设计阶段凭架构宣称无卡顿。

## 13. 实现前待确定

- 原生模块生命周期 trait、模块工厂及编译期 profile 的正式 API；当前接口仍为实验性。
- 模块进程内的完整文本、输入与平台服务适配；native Rust API 不替代跨进程语义契约。
- IPC 编码、资源拥有/借用映射、背压、超时和关闭门控机制。
- 同 GPU 选择、实际导入、跨进程共享纹理与同步的 macOS 原型，以及后续平台适配。
- 各 Container 的共享 Toolbar 接入、按钮保留属性和公开兼容迁移；窗口只提供布局/平台区域与目标位置，不重建全窗口 ToolbarHost。窄宽溢出和快捷键复用原能力并按影响验收。
- 三列最小宽度、窄窗口策略、live-resize 消息格式和临时呈现反馈。
- press/capture 建立/确认/终止消息、临时路由与 grant 顺序边界、有限期限、坐标与变换代次，以及外部几何变化时允许继续的协商边界；保留第 7.2 节的捕获活性与清理约束。
- 原生文本/无障碍快照字段、输入确认/校正及焦点与组字移交的可行性。
- context/viewport/environment/geometry-environment/layout revision 的编码与提交门控，以及文本确认/几何双通道的乱序验收夹具。
- 联合切换终态、准备期限、隐藏参与者 readiness 和恢复切换的具体消息格式及 profile 策略。
- TekesUI 环境 schema、主题资源兼容规则及模块更新确认接口。
- profile 格式、模块产物锁定、签名/能力声明与构建工具选型。

## 14. 参考与设计沿革

本设计承接 TekesUI 中的共享主程序与编译期装配提案，曾将框架归属迁入独立仓库；2026-10-21 决定与 TekesUI 同仓分层维护。当前设计以固定原生 ProcessComponent 装配、三个带 View 的独立模块进程、模块 toolbar 声明及无 View 导航/nav.json 契约为目标；既有 View 运行证据与新增 [服务侧最小导航原型](navigation-prototype-20261021.md) 的真实子进程/持久化故障证据分别记录；后者已验证窗口作用域 toolbar 注册和三组恢复故障，但不能视为五进程窗口装配、真实三 View 联合发布或物理导航输入已验收。原有 WASM 交付方案已由本次原生 Rust 固定装配替代；Rust View 工厂只在对应模块子进程内执行。

以下资料说明技术机制，不表示本项目已接入：

- [Metal 跨进程共享纹理](https://developer.apple.com/documentation/metal/mtldevice/makesharedtexture(descriptor:))
- [wgpu Texture 底层接口](https://docs.rs/wgpu/latest/wgpu/struct.Texture.html#method.as_hal)
- [AppKit NSTextInputClient](https://developer.apple.com/documentation/appkit/nstextinputclient)
- [Apple HIG Toolbar 指南](https://developer.apple.com/design/human-interface-guidelines/toolbars)
- [AppKit toolbar 可见优先级](https://developer.apple.com/documentation/appkit/nstoolbaritem/visibilitypriority-swift.property)

### Embedded application windows (#229, v0.2.1)

`AppWindow::open(&Window, WindowConfig)` creates a window in the caller's existing native application loop. It uses the same initial container mount, settings provider and Host event/render routes as `run`; it does not re-enter `App::run`. Process-backed consumers must retain the executable's existing worker dispatch. Gallery uses only local containers. Engine menu contributions are installed for the actual key window on focus; standard App/Window menu objects and AppKit's window list remain intact. A queued stale focus event cannot replace another key window's menus. Window closure relies on native key-window changes to restore the next window's contributions.

### Local text and native window movement (#247, v0.3.2)

The local AppHost text service reuses exact paragraph results within one draw only. The key includes full text, semantic role, type scale and exact width; the next `begin_frame` or a type-size change clears it. Draw position, color, clip, glyph raster scale and Scene construction remain per-call. Calls without an explicit frame lifecycle retain no paragraph table. Editable TextSurface paths are unchanged.

After a successful native titlebar handoff, uncaptured left-drag samples belong to the native window move and must not invalidate ContentHost geometry or schedule body paints. This does not suppress explicit redraws, content updates, animation, resize, hover enter/leave, or real component captures. New down/up/move and focus loss terminate that ownership, including AppKit moves that omit mouse-up.

See [the scoped native review](../../design/reviews/apphost-flicker-20261108/README.md) for measured improvements and remaining frame-interval variation; these rules do not promise every frame at 120Hz.


### Initial Theme inheritance for embedded windows — 2026-11-09

Refs #245 / #229. `AppWindow::theme(Theme)` selects the initial appearance and
contrast using the existing shared Theme type; all constructors retain LIGHT as
their default. Embedded and standalone run paths carry this same value into the
original Host. Its window ground, local content, mounted Toolbars, Others Tabs,
presentation chrome and render clear consume one matching Palette. An AppHost
Settings child inherits its parent's selected Theme through its original cold
assembly path.

The Gallery launcher passes its effective `gallery.theme()` when opening the
AppHost item, including increased-contrast modes. Select Appearance → Dark in
Gallery (Control-Command-3), then activate AppHost. The new window snapshots that
selection; this change adds no live synchronization, system listener, runtime
Theme registry or environment override. Process-rendered module interiors retain
their existing protocol and are not claimed to inherit new Theme data.

Host-owned monochrome toolbar artwork uses the existing Button state foreground,
IconContent and SvgIconCache pipeline; arbitrary ImageId content keeps its original
colors. Toolbar measurement and final paint use the same Palette so measurement
does not reset a dark button's motion channels. The original two collapsed-panel
SVGs are copied unchanged into the canonical icon catalog; the historical #57
artwork and coverage remain immutable. Settings Back uses the same semantic icon
path. The Toolbar text adapter forwards the existing icon and motion services.

Verification: 96 AppHost tests, 10 Toolbar tests (including four-mode semantic
icon state coverage), 4 Settings tests, 2 icon tests and strict clippy passed.
The three unchanged Gallery document/navigation tests are reused from the first
candidate. Native CU2 standard-dark review on a real 2x external screen verified
Toolbar rest/hover/held press/leave, actual Focus/Others exchange, both sidebars'
collapse/expand artwork, selected Tabs, the filled Draft menu and attached Field
focus. Evidence is recorded in Issue #245; this is not an all-component or
all-theme native Gold7 certification. The process exited and input was released.

### 2026-11-09 原生标题栏分配与正文起点

原生窗口验证发现缩窗时 AppKit 总标题栏 allocation 从51增到66pt，而实际 Toolbar paint 仍是50.4pt；旧实现只对含红绿灯的Nav把总allocation加进正文起点，造成三列不齐。本批将 AppKit 总allocation与内容安全边界分开：有Toolbar的顶行共享 max(paint_end, 已对齐原生控件中心的两倍)；极小字体仍以真实原生按钮高度为安全下限。无Toolbar的容器仅在真实原生控件相交时避让。原生按钮位置、尺寸与系统总allocation不改；正文的单层Spacing::Loose16不变，不添加负margin或额外padding补偿。此项仍须修复后原生缩窗验证。

### 2026-11-09：Gallery 模板 Main／Settings 原框切换

Gallery AppHost 的 Settings 使用既有 Exclusive 模式：先在 Main 的实际原生 frame 完成 Settings 呈现，再无动画隐藏 Main；返回时恢复原 Main 实例、文档和容器布局。两种画面的窗口尺寸与位置一致，不能以居中独立小窗替代。消费者仍可显式选择 Independent，未删除原能力。Settings 导航首项为与导航内容等宽的“〈 返回”（共享 Back 图标加文字），独占首行，Search 保留下一行；原焦点/组字/无效草稿离开门禁继续适用。

若在 Settings 中移动或缩放窗口，返回时原 Main 采用 Settings 当前的原生 frame；其他被暂时隐藏的窗口保留各自原 frame。沿既有非动画窗口组先显示目标再隐藏遮盖层，不销毁/重建 Main 内容。

## 2026-11-09 后续内容交互与 Settings 一致性

用户在 v0.3.5 实际使用后要求：Focus 正文须可选择/复制并保持只读，复用原文本服务与 selection owner，不能用静态 TextContent 代替；整段跨段选择沿同一只读 TextSurface，不新建文本选择引擎。AppHost 导航 List 原 scrollbar=true，指示条按真实溢出显示；需核实短窗与 Dark 的实际绘制，不为凑出滚动条增加伪条目或强造溢出。

Settings 内容边界复用主 AppHost 的可见且避让原生控件的 Toolbar 边界；正文与导航使用同一现有 spacing token 单一留白 owner，移除完整 titlebar allocation 带来的额外顶部空区。导航项使用共享 navigation List 行高/组距和填充选中态；Back/Search保持原稳定实例与全宽，Back仍为第一项。Theme三个Radio保持单一原RadioGroup model/view，在右侧同一行排布；默认竖排API保持兼容，新增原Radio横排布局，不拆成三个状态源。横排 Radio 按原文字渲染器测得的自然宽度借用行内剩余空间，左侧标题至少保留其可用的自然宽度，保持整组右对齐；测量和布局使用同一列宽，避免正常宽度下因旧固定值列上限而拆词。物理空间不足的过窄窗口保持同一行的几何/命中/AX一致，文字可在各自项内换行。

字号NumberField两按钮与编辑器之间留同一spacing token的空隙；测量、绘制、命中与AX共用一份布局。单行TextField水平滚动条缺省关闭、提供显式开关，滚动/reveal/IME继续沿原策略，不能把隐藏指示条等同禁用滚动；多行默认行为保留。

本批分别验收源码、受影响自动回归、原生选择/复制和输入/布局、公开发布及Gallery Dock采用；原通过证据按不变条件复用。v0.3.5和其原程序证据保留，后续改动不改签。

### 2026-11-10：统一工具栏拖窗与 Gallery 滚动条回归

Window host 的统一工具栏空白区域属于原生窗口 chrome。原控件、分割条及正文先处理输入，剩余空白区域再进入原生拖动／双击处理；不能仅以内嵌 Container toolbar 的 frame 判定。Nav／Others 显隐和 Focus 交换保持既有命令与实例身份，不因 host 迁移改变布局语义。

Gallery 原左侧导航显式启用纵向滚动条，按真实溢出显示，保留共享 ScrollArea 的开关及无条滚动能力。AppHost 和 Settings 使用 ListContent::scrollable 的入口保持原行为。源码检查不替代准确打包程序的拖窗、布局按钮和滚动条原生验收。

## 2026-11-12: navigation visits and compact Settings chrome

Gallery document identity remains the stable gallery-document route and index. Navigation History entry identity represents an individual visit: returning to a previously viewed document must create a distinct visit ID without changing the document or Container identity. Back/Forward reuses stored entries and resolves the same document. The human-readable title remains separate from the visit ID.

Settings keeps Back and Search in one sidebar header row. Back artwork starts at the same leading boundary as the sidebar controls while preserving its original action hit height. The search icon expands the original text editor in that row; committed text matches existing group/item labels and field labels/descriptions to the original item routes. Empty search restores the rows; Escape closes search, while an active marked composition keeps its original IME cancellation path. This follows the Swift Tekes compact-search interaction (ContentSidebarChrome and ClientSettingsController); it does not introduce a second editor or navigation model. Field highlighting and direct field scrolling are outside this bounded search change.

Number steppers retain their original hit targets, bounds and keyboard/AX behavior. Their minus/plus actions paint only the existing action-quiet foreground states (secondary at rest, primary on hover/press, disabled foreground at limits), without rectangular hover backgrounds. Theme values are not overridden in Settings.
## 2026-11-12：消费者以选择事件和稳定 item ID 导航（#229）

AppHost/Gallery 的业务导航消费共享 List 的 `ListIntent`，以稳定 `ListItemKey` 表示业务项；`selected_key()` 是选择状态的公开读取入口。消费者不得在动作到达后回读可变的显示索引、按分组偏移计算业务项，或用显示标题恢复身份。现有统一 action 通道承担事件传递，不增加全局通知、第二个选择 reducer 或框架对 Gallery 文档的依赖。

`SelectionChanged` 表示用户改变选择，`Activated` 表示显式激活；消费端根据意图携带的 key 映射自身导航目标。前进/后退或 Inspector 更新导致的程序化选择同步通过原 `select_key`/`clear_selection` 完成，不回发用户导航事件。文档 item ID、每次历史访问 ID 和 Container ID 分别保持原职责；重排、过滤、改名及相同标题不改变业务实体身份。

有限检查覆盖 AppHost、Gallery 的实际选择消费者，并包含 AppHost 使用的 Settings sidebar。固定数组展示、控件内部合法索引运算和兼容 API 不因存在 index 就判错；只有把展示位置当业务身份或回读丢失事件身份的接入需要修正。保留原样式、滚动、IME 拒绝、历史和内容更新语义。实现、受影响验证和交付分别记录在 #229；本节定义接入契约，不代表验证已通过。

## 2026-11-12：Others 文字 Tabs 与横向 ScrollArea（#229 / T64）

用户本次将 Others 切换的类型图标组调整为文字 Tabs。当前目标布局为 `[[title ×][title ×][+]] <flex space> [expand/collapse][Others on/off]`：左侧 Tabs 与末尾创建入口在真实横向 ScrollArea 中，右侧两项保持可见且不随 Tabs 内容滚动。旧独立 Close 由各 Tab 内透明的关闭按钮替代；创建入口使用原 `+` 命令，不再附在 on/off 按钮旁。

每个 Tab 以文本自然宽度、关闭按钮及统一 token 间距测量；标题有最大宽度，超限用省略号，完整标题保留为辅助文本。宽度不足时滚动这一个 Tabs 区域，不能以压缩所有 Tab、图标 family 下拉或隐藏菜单代替；当前选中项应能被 reveal。测量、绘制、clip、鼠标/滚轮/键盘与可访问性几何共用真实滚动偏移。

默认标题区最大宽度复用现有 TabsView 的 160 点上限，关闭按钮和间距另计；实现时由共享组件统一提供，不在消费者重复该值。Tab/List 聚合保持原语义：聚合 List 是一个不可关闭的文字 Tab，不新增关闭全部动作；单独 Container Tab 的 × 只关闭自身。Others 关闭时 Tabs/+ 随区域隐藏，固定 on/off 保留恢复入口；重新打开后按原状态显示选中项或空态创建目录。

Tab/Close/创建仍对应原 Container 稳定身份：相同标题及同类型多个实例不合并，点击关闭只关闭其目标，不额外激活它。现有 Tab/List 呈现声明互斥、聚合 List 的成员与关闭规则保持，不借此改为双重呈现；Focus/Others 交换、隐藏保留、展开覆盖 Focus、空态创建目录及恢复入口沿用原状态机。Container 的业务 Toolbar.host、Window 三列分区、Nav 前后命令、标题栏拖动与输入清理不因切换器换外观而改变。

实现优先复用共享 Tabs 和 ScrollArea。标题上限与间距优先采用既有组件配置/token；若缺少所需配置，只补适用于组件的最小入口，不在 Gallery 私自覆盖样式。验证本批改变的横向溢出、选中 reveal、精确关闭/创建、窄宽固定按钮可达性与输入身份；既有未受影响证据复用。源码、原生验收、公开交付与 Gallery/Dock 采用在 #229/T64 分别记录，本节不是通过声明。

ScrollContent 的原纵向默认布局保持；显式横向模式按子内容自然宽度计算 document 宽度，而非强制等于 viewport，继续使用原 ScrollArea 的滚动、裁剪、输入和 reveal。独立 tab strip 不能要求消费者挂载伪造内容面板，也不能在 AppHost 复制一套 Tabs 选择状态机；具体组件接线以实际共享实现为准。

普通滚轮的非精确 delta 是行量，精确 delta 是点量；共享 ScrollContent 在消费时沿用 TextSurface 的 `size-row-height` token 换算一次。平台事件与直接消费空间位移的 ScrollView 不重复换算。Others 聚合 List 的独立滚轮入口采用同一策略；Gallery 侧栏向 ScrollContent 传原始行量，其直接 ScrollArea 分支保持原换算。原生诊断以收到的 delta、precision 和累计 offset 为准，工具请求数字不等于实际点数；不声明与 AppKit 水平滚动绝对距离相等。
普通 Tab/Shift+Tab 必须接入现有窗口工具栏遍历：进入当前 Tab 标题，进入其可用的关闭按钮，继续到末尾 `+`，再离开条带；反向按同一真实焦点顺序返回。TabStrip 复用 TabsView 的 focus/focus_close 与按键状态机；Space 按下仅准备动作，实际绘制后松开才关闭，隐藏或关闭需清理按压和焦点。ToolbarHost 以 `set_focus_wrap(false)` 将边界交回外层，未设置的消费者维持原默认；不能以 AX 直接聚焦替代普通键盘可达性验证。
