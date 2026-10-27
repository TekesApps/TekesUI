# TekesUI AppHost 框架设计

日期：2026-10-21

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

原失败保留：旧 Main23 `exclusive-round03` 50秒超时，无首帧/READY；本 scope 的 Main resize/panel-frame 为 not_run，不转签。scope04 异步字段未就绪的驱动误报已修；Independent 首次在关闭前失活，焦点终态失败，同程序重放成功不抹除该失败，也不推定具体失活来源。Main 首帧现象仍须归因到实际启动/传输/导入边界，#70继续开放。

原 #70 AC3 的最终 PR 独立 checkout、精确公开依赖和同源 CI 尚未完成。AC4 要求关联 #48/Toolbar 剩余项并明确本 PR 不承诺的一般远端、多窗口与 VoiceOver 范围；物理 IME/完整 VoiceOver 并非 #70 明文规定的独立全量关闭门禁。本批只证明程序驱动输入与语义 AX，未宣称物理输入/IME/VoiceOver通过；#48 等原能力目标继续保留，不因本次范围说明被删除。完整既有Settings设计和历史原型保留在Gallery Root的 `docs/design/apphost-settings.md` 与 `apphost-settings-prototype-20261022.md`，后续接收沿原契约，不把历史实现转签为当前main完成。


平台Settings前置接收（2026-10-27，#70）：真实WindowHandle、普通/Exclusive子窗口、可见窗口恢复、key-window菜单/焦点分发和dirty退出通知已接入；重复present保留首次恢复名单与key，按身份去重。原47平台契约、strict及AppHost接口检查通过；定向真实AppKit回归验证实际激活/key前提、四窗口重复present、关闭恢复两可见窗口且不显示原隐藏者、恢复原焦点。证据SSD `artifacts/issue-136/settings70-window-main3e37ff7-intake/round02` 的exclusive-native及平台各round；早期harness缺正常生命周期而失败，记录保留。此有限原生结果不是完整Settings自动保存/Independent/物理IME/VoiceOver验收。


## 1. 目标与范围

维护一套共享 App Host 源码，按构建 profile 预定模块组合、平台能力和发行资源，构建并打包成不同 target apps。Host 的 toolbar、Panel 容器和 splitter 均使用 native TekesUI 组件，由 Host 组合并管理其布局状态；每个 Panel 通过 ProcessContainer 承载独立的带 View ProcessComponent，模块声明 content/toolbar 并输出画面、事件和请求。另可固定装配无 View 的导航 ProcessComponent：它独占 nav.json 和导航历史，Host 只执行导航，成功后回报，由导航模块提交历史。

这是一套公共框架与共享启动入口，不是生成多份 App 源码的代码模板。消费产品通过装配配置及必要的 host adapter 接入，不复制 Host 的状态机和 View。

AppHost、模块运行器及其接口在 TekesUI 仓库内分层维护；仓库共享不表示底层 controls/text/scene/render 反向依赖 Host。消费者提供用途型原生模块、固定装配配置和产品资源/能力适配，由共享构建工具产出目标 App。

本规范只讨论通用 Host、模块边界、三列布局、进程管理、渲染合成和输入输出契约；不列举具体业务模块，不定义业务 Kit 的内部结构或业务模型。

首版不提供用户安装插件、运行时发现任意模块、动态库加载、产品切换、模块热替换或热卸载。消费者通过普通 Rust crate 和模块工厂在编译期固定装配；首版将 Host、runtime 和选定模块链接为同一目标 App 可执行文件。Host 启动同一可执行文件的独立子进程，通过固定模块 ID 选择工厂；每个子进程只创建自己的模块实例，Host 不调用业务 View 工厂。崩溃后重新启动同一预定模块不属于动态插件替换。

去掉 WASM 后不再有虚拟机、线性内存、fuel 或 guest/host ABI。原生模块是可信的随 App 发布代码；独立进程提供故障边界，不自动提供文件、网络或凭证的安全沙箱。现有原型继承父进程权限，不声称已实现能力隔离或 CPU/内存配额。未来权限限制由平台进程沙箱及 Host adapter 实现并单独验收。

## 2. 分层与所有权

| 层 | 负责 | 边界 |
|---|---|---|
| TekesUI | 通用组件、布局、文本、输入、焦点、无障碍、动画与绘制；ProcessComponent 基础进程能力及带 View 的 ProcessContainer 适配 | 不依赖 TekesAppHost、业务类型或模块目录；无 View 不启动 GPU/共享纹理 |
| TekesAppHost | 固定装配、窗口、三列布局、原生 toolbar 呈现、焦点/输入协调、执行导航与结果回报 | 不实现业务 View、复制进程机制或维护第二份导航历史；不直接写 nav.json |
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

## 3. 三列布局

```text
App window / workspace (TekesUI + native macOS chrome)
┌───────────────────────┬───────────────────┬───────────────────┐
│ ● ● ●  left toolbar   │ middle toolbar    │ right toolbar     │
├───────────────────────┼───────────────────┼───────────────────┤
│ left content          │ middle content    │ right content     │
│ native module process │ native process    │ native process    │
└───────────────────────┴───────────────────┴───────────────────┘
```

macOS 上三列 toolbar 与原生标题栏、红黄绿窗口按钮处于**同一条顶部栏**，不能在普通标题栏下再放一排独立 toolbar。AppKit 管理真实窗口按钮、统一窗口 chrome 和原生窗口拖动；TekesUI 呈现模块注册的 toolbar 控件。左侧窗口按钮属于窗口而非某个 Panel，Panel 换位时它们不移动、不随模块卸载。

- 顶部栏中的三段 toolbar 与各列 content 的横向边界保持一致；splitter 改变列宽时同帧更新 Host 的 toolbar 布局，模块只接收 content 的视口。
- Host 为原生窗口按钮、安全区以及系统附加指示器预留空间；这些保留区不参与模块按钮命中，不传给模块。换位后按**物理槽位**重新计算保留区，不把左列偏移绑定到模块身份。
- toolbar 控件优先接收输入；可拖动的空白区域交由 AppKit 拖动窗口，splitter 命中区仍归 splitter。模块的 content capture 不得占用原生窗口控件。
- 标题栏高度、安全区随平台、窗口模式变化时，Host 重新计算 toolbar/content 几何，按现有 viewport/几何代次契约发布；不得在全尺寸 content view 上重复扣除普通标题栏高度。
- 单列与三列原型均验收：无额外标题行、窗口按钮与 toolbar 同排、左列无重叠、三列 toolbar 可点击、splitter 调宽后 toolbar 与内容对齐。全屏、不同系统样式与辅助功能作为后续平台覆盖验收。

`left / middle / right` 是物理槽位，不是导航、编辑器或检查器等固定业务角色。Panel 是有稳定身份的整体，包含 toolbar、content 和模块进程绑定；列指 Panel 当前占据的槽位。

- 三个 Panel 可以交换位置，整列 toolbar 与 content 一起移动；不销毁模块实例、不重启进程、不迁移业务状态。
- 每个 Panel 可以独立调整宽度、隐藏和重新打开；middle 没有必须可见的特殊规则。
- 隐藏不关闭会话或卸载模块，默认保留进程及模块状态，暂停不必要的内容帧生产；业务任务是否继续由业务所有者决定。
- 三列全部隐藏后，仍通过窗口菜单或快捷键恢复，入口不能仅存在于列内 toolbar。
- 记录 Panel ID、槽位顺序、显隐意图、有效宽度及可恢复的焦点引用。窄窗口的临时折叠不覆盖用户显隐意图。
- Host 使用 TekesUI 的 Panel、toolbar 和 splitter 组件管理分列、分隔线及 toolbar 与 content 的几何关系；向模块传入扣除 toolbar 后的 content 视口。
- 多窗口中，即使引用相同模块定义，每个 Panel 实例仍有独立进程和输入/呈现状态，不共享一个可变 UI 单例。

### 顶部栏测量与容器边界

每个 Panel 的模块声明是否注册 toolbar 以及动作、可用状态和命令 ID；消费 App 与 AppHost 不补造业务 toolbar。TekesUI Toolbar 组件拥有固有尺寸测量、单行排布、溢出触发器/菜单、禁用状态与输入交互。宽度不足转入溢出菜单，不换行改变顶部栏高度。模块标题中的换行按空白处理。

共享 Host 仅将窗口级贡献与三个 Panel 已注册 toolbar 的所需高度和平台当前窗口 chrome 安全区取最大值，并按 backing scale 对齐，统一三列顶部栏。左右按钮安全区来自平台实际窗口控件几何。高度变化与分栏变化一样更新 content viewport；绘制裁剪和指针 down/move/up 坐标使用同一份 content 几何，旧帧仍受 viewport 输入门控。溢出菜单作为 toolbar 自己的 overlay 最后合成，Host 只转发原命令 ID。

普通 UI Container 保持布局/组合语义。本轮不为所有 Container 增加 `subprocess=true`。可复用的进程容器应是显式模块边界（ProcessContainer），声明固定模块入口并承载进程、IPC、共享画面和失败处理；任意现有组件子树不能靠布尔值自动移到新进程。ProcessComponent 是公共进程基础，View 可选；ProcessContainer 仅适配有 View 的模块。无 View 导航模块不创建容器、renderer 或共享纹理，toolbar 注册独立于内容帧。当前公共抽取与无 View 接口仍待实现，不得把旧 PanelModule 原型当作已经支持。详见 [ProcessComponent 与导航契约](process-component-navigation.md)。

## 4. content 注册本列 toolbar

每列 toolbar 的内容由该列 content 原生模块注册。无 View 的导航 ProcessComponent 也可注册常驻窗口级 toolbar 贡献，其纯图标前进/后退不依赖 Panel/View 或 frame-ready。Host 负责呈现并按所有者转发动作；模块负责命令语义、描述及状态。leading/trailing 显隐仍属于 Host 布局操作，不写入导航历史。

```text
content 原生模块 → 注册 toolbar 描述 → 模块进程 / IPC → Host 原生 toolbar
content 原生模块 ← toolbar action ← 模块进程 / IPC ← 用户操作
content 原生模块 → 新状态 / 描述更新 → Host 更新同一 toolbar
```

职责级契约包括：

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
| AppHost | 使用 TekesUI 组件挂载 View/窗口作用域 toolbar，提供可用空间，按作用域管理挂载/注册寿命、Panel 换位和命令路由；不另写布局算法 |
| 有/无 View 的 ProcessComponent | 在获准的 View/窗口作用域注册 item/command ID、分组、建议位置、图标、标题/提示、可见性及 enabled/checked 等业务投影 |

TekesUI 的通用行为参考 Apple HIG 与 AppKit 的 toolbar 分组、优先级和空间分配约定，不直接把 NSToolbar 对象或平台私有行为暴露为跨平台协议。View 与窗口级 toolbar 的挂载、模块归属、作用域及消息寿命属于 Host/公共注册契约，不由 AppKit 自动提供。

位置使用 leading/center/trailing、组内顺序、间隔/伸缩和溢出优先级等布局意图，由 TekesUI 根据可用空间与布局方向求解；模块不指定按钮的屏幕绝对坐标。具体控件集合、分组是否整体溢出、中心区避让及最小宽度待 TekesUI 接口设计时确定。

图标使用命名空间化资源引用或受控资源句柄，声明尺寸、模板着色或多色等呈现意图，由 TekesUI 统一解释。Host/运行器负责校验引用、解析随包资源并按资源代次缓存；缺失资源使用可诊断的回退，不允许模块借图标引用任意读取文件。平台图标映射、缩放及辅助功能标签遵循同一通用规则。

“跟随”分别约定：整列 toolbar 跟随 Panel 换位由 Host 管理；跟随上下文/选区变化由模块提供投影；随滚动固定、隐藏或浮动等通用呈现行为由 TekesUI 提供，Host 按声明策略配置。首版默认 toolbar 位于本列 content 上方，不跟随 content 滚出；其他行为不视为已经支持。

toolbar 的通用布局规则不能替代第 9.1 节的上下文联合发布门控。

## 5. 每个 Panel 的独立模块进程

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
- TekesUI toolbar 控件/分组/图标接口、窄列溢出和快捷键冲突规则；Host 仅定义注册/挂载适配。
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
