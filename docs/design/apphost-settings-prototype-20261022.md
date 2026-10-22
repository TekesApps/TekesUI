# AppHost Settings 最小原型（2026-10-22）

实现范围：消费者固定 TOML → 共享 Group/Item/Field 模型 → TekesUI 共享目录/表单 → AppWindow 两列 SettingsWindow → 本地 Provider 的自动保存和独立进程 Undo/Redo。完整设计见 [Settings 契约](apphost-settings.md)。

- `crates/tekes-ui-settings`：独立于 AppHost、Agent、平台/GPU 的解析、校验、草稿、共享表单与目录，以及可替换的本地文件适配；控件复用 TextFieldContent、SwitchContent、MenuContent、ButtonContent、ScrollContent，以及标准 List/ListView 的 SelectableListContent 适配；目录不定义选择颜色或其他按钮样式。
- `crates/tekes-ui-app-host`：AppWindow 装配及两列 SettingsWindow，菜单/⌘, 入口；复用原有 navigation ProcessComponent、toolbar、leading 显隐图标/Gold7 motion 和 splitter。主消费 App 的三列及模块进程继续保留。
- `examples/app-host-demo/settings.schema.toml`：以 ClientKit General 为默认参考，只声明外观和文字缩放百分比；完整字段类型/多 Item 导航测试使用独立的 `tests/fixtures/layout.schema.toml`，不再让默认内容受测试夹具约束。
- `examples/settings-consumer`：不依赖 AppHost 的第二个公共 API 消费者，验证定义、form/sidebar 和草稿机制；其 CLI 检查不代表 native/GPU 验收。

`AppWindow::new(profile).navigation(...).settings(SettingsWindow::from_toml(...)?).run()` 固定装配 Main 与 Settings。模块数为两列或三列；单列继续供技术验证。SettingsWindow 的 DisplayMode 默认 Exclusive，可选 Independent；消费示例提供 `two-panels`、`three-panels`、`independent-settings` 构建 feature，均不做运行时模块发现。

左侧 Group/Item 来自 TOML，右侧仅展示当前 Item 的 Fields。左栏无 toolbar title；右侧提供 Group 祖先路径与 Item 组成的面包屑标题，使用共享 ToolbarContent/ToolbarGroup 的 Navigation placement；前进/后退保留在左侧 navbar，仍由原 navigation ProcessComponent 控制。Main 的当前文档标题也归入该导航组，不再由 Host 另画一个居中标题。切换 Item 保留 namespace 编辑并按批次自动保存；底栏整体移除。关闭/Escape 等待有效编辑保存成功，不再丢弃未保存值。Undo/Redo 在左侧 navbar 注册，由独立无 View ProcessComponent 维护已保存配置历史。Settings 的 Item 导航历史使用独立目录，leading 显隐及字段变化不进导航历史。

表单当前只调整布局：每项左侧仅为可换行名称，右侧为原有配置控件；TOML description 使用原控件和名称上的 TekesUI hover hint，并由同一 help 声明提供无障碍描述，不再常驻占行；各行共用同一列宽规则，控件右边缘对齐并在行内垂直居中。重复页面/分组标题不显示。布局组件本身不绘制底板、边框或颜色，其他 TekesUI 默认控件样式保持；按用户要求，Settings TextField 使用无边框 Content presentation 与 trailing 对齐，复用原 TextSurface 的绘制/光标/命中/选择/IME。字段仍保留原 mount 和输入服务，字段名称通过行上下文及原控件 help 提供给无障碍。

本地存储为 `~/Library/Application Support/TekesUI AppHost Prototype/settings/general/settings.toml`，仅保存覆盖值及 `_tekes` 命名空间/revision/last_commit_id 元数据。使用 App 生命周期内独占文件锁、revision 检查、临时文件写入/fsync、rename 和目录同步；未知配置键保留，Undo 可恢复不含覆盖的默认状态。文件读取与保存经后台工作线程/WindowWake，UI 不等待写入；保存回执只清理对应批次，较新编辑仍 dirty。确定的替换前失败与替换后的 unknown 分开处理，unknown 只能查询原请求状态。

本轮仅实现串行本地 Provider：最近 commit ID 可查询/去重并覆盖 Provider 重开、丢回执场景；已为本地配置与 undo.json 实现 prepare/confirm 和重启对账；尚未实现远端 Provider 恢复屏障、凭证复合 Apply、业务 action/draft patch、动态选项、数组/map/custom editor 或完整 ClientKit 能力等价。这些仍按完整设计的后续阶段及故障验收执行，不用一次本地保存冒充其验收。

## 验证

构建/测试及 native 证据保存在 `design/reviews/apphost-settings-prototype-20261022/`。以该目录的最终 ACCEPTANCE.md 和 identity.json 为准；早期整页单窗截图及中间构建不代表最终的两列/展示模式验收。

使用 Cargo 1.95.0、共享 `build_lock()` / `environment()` / `target/review-cache`，不清理缓存、不切换工作树、不提交或推送。未改 ClientKit、Kernel 业务实现；所有 unrelated WIP 保留。


## ClientKit General 默认配置

参考 ClientKit 的 `ClientGeneralSettingsConfiguration`、`ClientGeneralSettingsStore`，AppHost 公共默认仅保留 appearance=system 和 text_size_percent=100（范围 50...300，步长 10，显示 %）。按用户确认，思考内容显示和 Running HUD 均从默认定义移除；消费者需要时自行注册。

Feedback 是代码 action，不是持久化字段，当前最小原型尚未实现该 action，不展示无效按钮。Markdown Format/About 属于其他 Item，未将它们冒充为 General 字段。本轮仅对齐默认定义/默认值；外观 segmented 及配置应用到窗口的业务绑定仍需对应消费者能力；百分比可编辑数值控件已接入共享 NumberFieldContent，不声称当前本地 Provider 已应用这些业务效果。

当前 demo 的配置及 Settings 导航使用独立的 `settings/general-percent/` 目录和 `apphost-demo.general-percent` namespace；旧 `settings/settings.toml` 与其 navigation 保留，不覆盖旧字段和历史，也不把旧路由恢复到不兼容的新 schema。

## 自动保存与配置 Undo/Redo

最新约定见 [保存策略](apphost-settings.md#当前-apphost-保存策略2026-10-22-更新)。`settings-undo` 通过原 ProcessComponent 启动为独立 headless 进程，窗口绑定 toolbar，仅维护本地配置历史，不创建容器或 GPU View。原 Provider 仍是唯一配置写入者。

当前输入合并窗口 300 ms，失焦/关闭立即 flush；IME marked text 和非法中间输入不保存。无固定底栏，只在校验或持久化失败时显示错误/Retry。原文本编辑的撤销与已保存配置历史分开；本地历史跨 Settings 重开/进程重启恢复。历史确认前不移动 cursor，确认丢失后以相同提交身份对账；不同配置 revision/内容导致 fail closed。

`examples/app-host-demo --verify-settings-undo` 使用真实 child ProcessComponent 验证未写入 prepare 的恢复、已写入丢确认后的子进程重启、Provider 回执、Undo/Redo 和 toolbar 注册。单元测试另覆盖 redo 分支、错误回执、外部配置冲突、未知 datetime 往返及原控件 mount 保留。native 验证另见 `autosave-undo/`，CLI 结果不替代窗口验证。

## Description 提示

所有字段的 `description` 通过原 Content 的 `with_help` 接入 TekesUI 提示机制，名称和配置控件均可触发；空说明不创建提示。shared help 同时投影到无障碍 description，不增加提示按钮或复制控件。校验/保存错误仍保留可见的错误文字和 Retry，不改为仅悬停可见。

## Toolbar 分组与覆盖切换（2026-10-22）

Main 是 `examples/app-host-demo` 消费示例，负责固定模块、路由和 Settings 数据装配；公共窗口行为由 AppWindow 的同一 Host 提供。Main 与 Settings 复用导航组 Navigation、显隐组 Navigation 和 center title 组 Navigation 的声明。消费者只提供标题/面包屑文本、导航和命令注册，不复制分组行为。

Settings 的独立 `settings-undo` ProcessComponent 贡献独立 SecondaryAction 组，在 Settings toolbar 右端呈现；不再合并进入 Back/Forward 的 ToolbarControl。它保留自己的原 ToolbarControl、ContentHost、focus、hover、More 和 AX 身份，动作只分发给配置 Undo 模块；原导航进程仍只收到导航命令。Settings 没有 trailing 容器和开关。

Exclusive 通过平台 `open_exclusive_window` 在首次显示前设置最终 frame、完成初始化和初始绘制，禁用系统窗口出现/消失动画；先显示 Settings，再在同一无动画 AppKit group 中隐藏 Main 及其余先前可见窗口，拖动 Settings 不会露出 Main。关闭时先在 Settings 下方无动画恢复原窗口，再移除 Settings 并恢复原焦点。Independent 保留普通新窗口行为。这个覆盖策略不改变保存前的关闭保护，也不宣称跨显示帧的呈现时延已经测量。

Exclusive 模式在左侧导航 List 上方显示标准 `Back` Button，位于 List 滚动区域外、固定在原位；Independent 不创建该按钮。Back 不注册导航历史命令、不改变 nav.json，而是走 Settings 的现有 Close/自动保存屏障，保存或校验未完成时仍留在 Settings。leading 收起和过渡期间不接受该按钮的输入，重开 leading 后恢复。

Back 呈现为 TekesUI 标准 CompoundButton：工具库 `tekes-back` 图标与原 TextContent 组合成单一按钮，保留 Back 可访问名称，并用 with_help 声明“返回主窗口”；图标和文字不拆成两个交互目标。hover overlay 由原 ContentHost 绘制。Settings 两侧顶部 inset 使用 space-regular（默认 8pt），替代 24pt；按钮与目录间距也为 8pt，水平及底部留白保留。间距随文字缩放。按钮仍在 List 滚动区域外。

## 固定容量的百分比输入（2026-10-22）

控件、scene、tokens、text、motion、render、icons、protocol 和 module-runtime 的 git 依赖统一使用 tag `controls-number-field-20261022`，lockfile 固定 commit `50ec49f3446924fdbc66b3cff22cd8cfe150fe7c`。AppHost、Settings 与现有多窗口平台适配仍保留本地源码；该 tag 未含多窗口 API，不将其冒称为全部平台能力的发布 pin。dependency-sources.json 记录实际依赖来源。

共享表单按声明的 default/current value、min、max、step、suffix 构建 NumberFieldContent，使用 DecimalNumberFormat，并调用 set_input_digits(3)。三位是布局容量，非输入长度或数值范围限制。控件自然宽度在 SettingsRow 的 trailing 侧挂载，不扩展到父容器宽度。原 TextSurface 处理文本/IME、控件处理提交、取消和步进；表单只 drain_value_changes 后交给既有 Draft/Provider。Enter/blur 提交，Escape 先取消数值草稿，不直接关闭 Settings；非法草稿不进入配置历史。配置 Undo/Redo 仍由 AppHost 的 headless 服务维护，数值编辑或保存期间不接受配置历史操作。

旧 general/text_size_step 一次性按 ClientKit 原 ramp 转成 general-percent/text_size_percent，保留 appearance/未知配置。旧 settings/general 目录及 nav.json、undo.json 不改写，新 namespace 独立建立历史。没有旧配置时默认 100%。

固定 tag 尚未导出 Settings 原先使用的 List Content 适配和 deferred-help API：Settings 使用薄 Content 适配调用该 tag 的原 List/ListView，不定义选中样式或输入状态机；hint 使用原 ContentHost 自动绘制，将完整 toolbar scene 在内容之后合成，避免被内容覆盖。数值 TextField 的 AX role 不因其 value_range 被错误改成 Slider。

验证证据：design/reviews/apphost-number-field-20261022。AppHost 40 项、Settings 18 项、迁移 1 项测试，以及 Clippy、sample 构建和第二公共 API 消费者通过。原生 .32 验证输入/步进/边界/取消/持久化和 Undo/Redo；最终 .33 增加数值草稿与配置 Undo 的屏障，已打开并验证 200% 未提交时禁用配置 Undo、Escape 回到保存值 110%。
