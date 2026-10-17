# 附件 DESIGN.md 对齐核对与补充

核对日期：2026-10-16。范围：Gallery 工作树中的现行设计输入、品牌编译器、Rust token、控件模型与 Gallery 注册；主目录的早期文档不代表当前实现。

附件来源与 SHA256 记录在 [机器契约](../../design/behavior/reference-alignment.json)。附件是比较资料，其中的 MUST、发布门槛和工具命令不会自动覆盖项目约定。按本次用户要求补缺，颜色比较语义、生成关系、主题方向和对比度，不逐项复制 HEX。此前提案的显式选择保留；新的视觉提案不能冒充已批准默认值。

## 颜色规律结论

**语义结构大体一致，计算机制与部分设计选择不同。** 附件没有定义 OKLCH 品牌生成算法，不能据此声称两套算法相同。

| 附件规则 | 当前输入/计算与运行路径 | 核对结果 |
| --- | --- | --- |
| 基础值→语义→组件别名 | `source/base.tokens.json`、品牌生成 primitive、`c-*` 别名→合并投影→Rust `proposal` | 一致；新增同义颜色表反而会分裂真源 |
| 四层暖中性表面；浅色高层可同色 | `surface-0..3`；四主题明度非递减，Light 的 2/3 同色 | 一致；同色须由位置或轮廓区分，不能宣称存在额外明度层 |
| 品牌只占少量操作/选中/反馈 | seed→OKLCH→限制 C→色域映射→搜索满足约束的 L→8-bit sRGB；正文/表面/状态等独立 | 目的相容；附件固定 Clay，我们支持显式消费者品牌，不强制 Clay |
| Light hover 变深、Dark hover 变亮 | `policy.targets.*` 的 action/link/subtle 状态方向；最终量化颜色再次验证顺序及 ΔOK≥0.012 | 一致；同时检查 pressed，不能仅比较 seed |
| 浅品牌底降低强调强度 | `C_subtle=min(C_seed×0.35,0.055)`，四主题独立 L 目标，输出不透明色 | 意图一致、算法不同；附件暗色以透明 Clay 叠色，背景变化会改变最终颜色 |
| Clay 上默认深字 | Light 用较暗品牌填充+白字，Dark 用较亮品牌填充+黑字，按 4.5/7 验证 | 有意差异；`brand-seed`/`brand-decoration` 不可直接当按钮背景，也不可拿链接 accent 代替 action 色 |
| 普通字≥4.5、必要边界≥3 | 编译器检查声明的实际色对；增强对比度文本提高至 7；透明覆盖先合成再算亮度 | 一致且增强主题更严格；只证明声明的色对，不证明任意嵌套或原生像素 |
| 弱文本可读，禁用与错误区分 | `tertiary-text` 与 `disabled-text` 分开；status 不随品牌变 | 一致；Dark 弱文本不一定严格比 secondary 更暗，不能把“层级”误写成机械灰阶排序 |
| 焦点采用品牌色；弱/强边框分开 | 焦点中性；`separator` 装饰、`border` 必要边界；IC 加强 | 焦点色有意不同；功能边界不能直接套附件低 Alpha 装饰边 |
| 选中/hover 使用透明底 | 持久行选中为中性色；可选品牌 marker；`hover-wash`/scrim 保留运行时 Alpha | 语义相容、配方不同；不可统一改成品牌色或把所有 opaque token 当最终像素 |
| 状态不只靠色彩，成功/警告/错误/信息独立 | status/warning/diff/syntax/chart 不进入品牌依赖闭包 | 色彩隔离一致；文本、图标、AX 通知须在消费组件验收 |

颜色核对入口为 `scripts/check-reference-alignment.py`。它实际重编 neutral/blue/clay，检查投影新鲜度、声明色对、表面阶梯、状态方向/差异、非品牌角色独立和下表角色映射；输出中的原生结果仍为未测。源上 sRGB source-over 为 `C_out=αC_fg+(1−α)C_bg`，对比度则对合成结果做 sRGB 线性化再算 `(L_max+.05)/(L_min+.05)`。OKLCH 推导、sRGB 混合与线性亮度是三个不同步骤。

## 组件覆盖：15 组逐项核对

“有规范”表示导入 registry/recipe 已定义；“有模型”表示现行 Rust 文件有对应模型，不等于全状态、VoiceOver、真设备通过。

| 附件组件 | 项目对应 | 当前覆盖与本次补充 |
| --- | --- | --- |
| Button | `tui.button` / `widgets::Button` | 有模型及 Gallery；防 loading 重触发已有；保宽、全部 variants 和 AX 仍需分别验收 |
| SegmentedControl | `tui.segmented-control` / `extended::SegmentedControl` | 有模型；值选择 radio 与面板导航 Tabs 已区分；辅助技术语义未测 |
| StatusIndicator | `tui.status-badge` + `tui.progress` | 有展示/模型，但不是完整异步状态业务组件；本次补点+文字、idle/running/success/error、节制播报契约 |
| Dialog | `tui.dialog` / `widgets::Dialog` | 有焦点循环/返回触发点模型；危险默认安全动作与 IME/Escape 路由由宿主验证 |
| Attachment | `agent.attachment` | 已有 planned recipe；补移除焦点恢复、重试入口、未就绪阻止发送 |
| UserMessage | `agent.message`（user） | 已有 planned recipe；补 sending/sent/failed 子状态和附件/长内容边界 |
| AssistantContent | `agent.message`（assistant） | 已有 planned recipe；补 cancelled 与稳定流式/复制行为 |
| ThinkingDisclosure | `agent.thinking` + Disclosure | 有 planned recipe/基础 Disclosure；补长过程不可抢阅读位置 |
| Composer | `agent.composer` | 已有 planned recipe；TextInput 的 marked text 不是完整发送路由；补菜单优先级与 sending 草稿确认条件 |
| ModelSelector | `agent.model-usage` | 已有 planned recipe；补独立推理档、换模型保草稿、不可用说明 |
| ToolDecisionCard | `agent.tool-decision` | 已有 planned recipe；补 Retry-After/抖动/最多3次、取消终止、禁止暗中重放 |
| CodeBlock / Diff | `tui.code-block` / `tui.diff-view` | 有模型及 Gallery；复制/换行/行号与实际原生文本的完整组合验收未测 |
| MediaViewer | 新增本地 `agent.media-viewer` 契约 | 原 artifact-preview 只规定产物宿主，不能替代锚点缩放、平移、复位与退出契约；本次补齐接口、公式、状态、键盘、焦点及验收 |
| SplitPane | `tui.split-view` / `extended::SplitView` | 有拖动/取消/键盘/受限尺寸模型；通用最小值不是业务280pt；业务配方补双栏 minima 与嵌入网页捕获清理 |
| Sidebar | `agent.session-row` + List/Tree | 有基础模型/业务 planned recipe；补重排稳定 ID、焦点与选中分离及恢复 |

以上业务 recipes 不因本次补规范变成 Rust 已实现。本次没有把计划项放进可操作 Gallery 冒充实现，也没有新建网络/持久化/授权后端。

## 缺失契约补充

完整可枚举接口、状态、行为、色彩角色见机器契约的 `components`。补充规则适用于后续实现及验收：

1. **媒体查看器**：输入 `resource, viewport, loading_state, return_focus_id`，输出 `close, retry, transform_changed`；loading/ready/error 下始终能关闭。缩放以指针或触摸中心 O 为锚，`T_next=O−(O−T_prev)×S_next/S_prev`，X/Y 分别计算。比例定义为相对 fit-to-viewport 的 1–8 倍；先按锚点求解，再约束平移，允许边界钳制导致锚点偏移。Reset 重新 fit 并居中；容器变化重新约束。提供 +/−、复位、关闭按钮及键盘入口；拖动结束、取消、丢捕获和失焦都清理捕获；退出返回触发点。未钳制时锚点漂移≤2逻辑pt；原生目标另测1x/2x。媒体必须有替代说明。
2. **状态与消息**：运行才允许循环动画；静态名称+图标是必要表达，状态从真实执行来源读取。UserMessage 保留发送中、已发送、失败差异；AssistantContent 有 complete/error/cancelled，停止后不可伪装 complete。流式不逐 token 播报。长内容不能溢出；复制不得携带按钮或行号装饰。
3. **输入与附件**：IME→建议菜单→编辑器→弹窗→全局；Enter 先确认候选/建议，Escape 只消费当前层。空白且无有效附件不可发送；未完成/失败附件必须有解释。sending 前保留不可变草稿，durable ack 只清对应版本；旧请求不得清除新草稿。移除附件后焦点到相邻附件或编辑器。切换模型/推理档不丢草稿。
4. **工具与恢复**：同调用 ID 关联 pending_approval/running/success/error/denied/cancelled；授权过期独立处理。运行后移除重复授权入口。仅幂等或明确未执行的请求可重试；退避加抖动，尊重 Retry-After，默认最多自动重试3次；取消后不再调度。后端无范围持久化和撤销能力时，不提供“始终允许”。断网恢复不得暗中重放。离线持久化只有真正完成存储/恢复/去重后才能承诺。
5. **分屏与侧栏**：业务双栏最低各280pt；空间不足切为可恢复的单栏。通用 SplitView 仍可被别的场景配置，不把280写成通用控件限制。拖动阈值按已有 ≥4pt（附件写“超过4”），保留边界差异；必须测3/4/5pt。宿主捕获跨 WebView/iframe 指针或使用真正拦截事件的遮罩；所有取消路径恢复。分割线1pt、桌面命中≥12pt、触控≥44pt；键盘16pt步进、Home/End、均分/折叠入口。重排保持稳定ID及焦点，独立于选中值。
6. **音频与渐隐（可选消费能力）**：手势后初始化音频、可静音、与视觉状态同步；0.08门限/0.7压缩仅是需实测初值。渐隐采用附件8个采样点和24pt初值，到边界取消相应渐隐，不遮焦点；减少动态效果取消过渡。当前未实现，不纳入基础控件完成声明。
7. **宿主隔离与主题**：主题初值可取系统偏好，显式选择须由消费者持久化；框架不能隐式写设置。原生窗口材质可选且需不透明回退。Web/Electron 的拖窗口区与按钮分离；预览隔离容器、origin/source/结构/范围校验属于有嵌入网页的消费者必需项，纯原生 Gallery 无 iframe 不套浏览器属性。

## 有意差异与待评审提案

| 领域 | 当前约定 | 附件差异与采用边界 |
| --- | --- | --- |
| 阴影/材质 | `source/layout.json: shadow=none`；motion 禁 vibrancy | 附件双层阴影与材质不直接覆盖；当前用轮廓/位置分层 |
| 动效 | press_scale=1，hover_translation=0；现有时长及原生 motion presets | 附件0.98按压、位移Popover不启用；弹簧k/c/m与我们的Hz/阻尼比表达可换算，但不因此等效所有实际配方 |
| 密度 | compact/normal/comfortable 行高；Composer 单一20pt角色 | 附件 conversation/workbench 是整套配置，不能拿三个行高冒充。补为待评审 profile：conversation=20pt，workbench=12pt；尚未接入运行时，不改变已用token |
| Composer尺寸 | editor min44、max240、expanded384；外壳min72 | 附件 min48、max min(384,40dvh)，部件层级和上限不同。采用现有范围与可用高度约束，待做可视候选，不混入两组数 |
| 排版 | chat14/20、doc14/22、UI13/16等16样式 | 附件15/24正文、38/46问候语等并非当前样式。大展示标题/serif 是可选产品提案；无选定字体与实图，不向通用DS随意加字号 |
| 容器阈值 | 576/480/384/320已定义 | 用容器测量，不只看窗口；动作降级按现有 source.layout 规则，关键动作保持可达。阈值两侧仍需实际渲染 |
| 跨平台/网页 | 当前原生macOS Rust Gallery | 浏览器三引擎、forced-colors、Web Audio、iframe 仅在相应消费者纳入范围时适用；VoiceOver、IME、真实指针仍须测 |

## 验收矩阵

| 附件ID | 本次结果 | 证据/下一层验收 |
| --- | --- | --- |
| V01–V03 | 静态规律核对；原生未测 | `design/reviews/reference-alignment-20261016/report.json`；后续四主题、倍率、边界截图 |
| V04 | 规范差异已记录；运行未测 | 当前 motion 与源规范分开，不能用参数表证明动效一致 |
| V05 / I01–I05 | 已有契约+本次细化；端到端未测 | 真实中日IME、菜单/弹窗路由、延迟发送、上翻后图片加载 |
| V06 / I06–I08 | 分屏有模型，媒体缺实现；原生未测 | 3/4/5pt、跨嵌入网页、取消/失焦、缩放与边缘钳制 |
| V07 | 补齐规范入口；业务组合实现仍缺 | 上表15组映射；文档不是组件运行证据 |
| I09–I10 | 契约齐；真实服务/离线未测 | 不编造授权、持久化和去重支持 |
| I11–I13 | 真实键盘/VoiceOver/异常全矩阵未测 | 模型单测不可替代辅助技术或设备测试 |
| §7.4 | 未测 | 帧间隔p95/100ms停顿、1万/5万字符、音频及麦克风；既有性能报告不可当本次全量验收 |

本次补的是 Design System 的比较证据与缺失设计契约。实际业务组合、媒体渲染、应用持久化和平台验收仍按上表独立推进；不得写成“附件全部实现/验收通过”。

本次验证：颜色规律与2,880项声明色对检查通过，负例守卫通过；原始包/本地引用/历史golden/合并投影检查通过；`tekes-ui-tokens` 18项与 `tekes-ui-controls` 47项测试通过。没有运行GUI、真实IME、VoiceOver或性能测试。改动未提交、未发布。验证收据见同证据目录的 `verification.json`。
