# #14 最小场景第三批：Rich Dialog 过渡、IME 与原生语义桥

2026-10-17。基于 `bc3de79466834e99062a1b4ef0872323e2860001` 和活跃 checkout 的全部未提交 WIP，接续 [第二批记录](composition-list-dialog-20261017.md)。重新读取 GitHub #14（更新时间 `2026-10-17T08:15:16Z`）及 #12；二者仍开放。本批不更新远端、不扩大组件迁移、不做性能验收，组件继续 experimental。冻结源身份、构建、测试和图像见 `design/reviews/composition-presence-20261017/receipt.json`。

## 根因与共享修复

| 根因归属 | 修复位置与复用 | 适用组件及合理例外 |
| --- | --- | --- |
| 容器组合能力：关闭模型后，组合树立即失活，原有 OverlayPresence 未承接完整子树 | `composition::DialogContent` 持有现有 `control_motion::OverlayPresence`；`ContentHost` 分开保存语义 modal 与仅用于绘制的 presented_dialog。每帧重新构造 Dialog 子树 Scene，再交给原过渡实现；不缓存跨帧字形图集引用 | 所有接入 DialogContent 的 Header/Body/Footer 内容，包括真实文本编辑、Switch、嵌套布局和浮层。未另建动画协议、token 或曲线；原 DialogView、Menu 等消费者保持各自既有接入。本批不迁移它们 |
| 公共组件契约：关闭期间的绘制存活不能延长事件/焦点/语义存活 | `ContentHost::close_dialog` 立即结束 capture/modal，恢复稳定 invoker；focus 与 focused_text 检查整条 active 祖先链。关闭子树仍可绘制，但拒绝重新获取焦点或处理事件 | 所有组合 Dialog 子控件；背景在语义关闭后按普通页面路由，不继续被退出画面锁住。重开同一个 Dialog 复用原进度，切换绘制 owner、卸载或资源失效会清除旧 presentation |
| 基础输入适配：公共 Key::Escape 与原生 cancelOperation 路径对组字的优先级不一致 | `TextFieldContent::event` 使用现有 TextInput 取消 marked composition；第一次 Escape 交给编辑器，第二次才关闭 Dialog | 所有 TextFieldContent；没有新增编辑器状态机。更完整的编辑器命令、物理 IME 候选窗与日文转换矩阵仍不由此推定通过 |
| 公共语义契约与平台适配：组合语义未输出原生 AX 节点，也无公共动作回路 | `composition::{SemanticPart, AccessibilityAction}`、`ContentHost::accessibility_action`；平台公开 AccessibilityNode 快照、Window setter 和 Event::Accessibility；原生消费者负责映射，不依赖 Gallery | Button/CompoundButton、Switch、TextField、Menu 及消费者 ActionRow 复用原模型操作。Menu 项是已有模型的语义 parts，不包装成伪 Content 子组件；生产 MenuView 的 focus_item/activate_item 复用自身运行时与 reveal/activate 路径。静态文字、容器没有虚构 Press |

平台桥按稳定 `(component, part)` 缓存 NSAccessibilityElement，更新原生屏幕坐标、父子关系、名称、值、帮助、焦点、选中与展开状态；Switch 映射 AXCheckBox，Dialog 使用 Group + Dialog subrole。请求回到 Host 后重新检查 active/modal/disabled，原生过期对象也不能操作已移除节点。重复键、缺父节点、循环或非有限/无效几何的快照 fail closed。相同快照不重复发布 layout notification。此次公开最小协议增加了可选 trait 方法与语义字段，尚不构成全库 API 兼容性或版本发布批准。

## 三类验证证据

### 公共 API

- 工作区 release 测试：888 passed、0 failed、1 原有 ignored；显式过滤两项 CPU/GPU 计时测试。独立消费者 workspace：10 passed。
- 关闭测试验证完整退出帧会再次调用子内容绘制，scrim alpha 随原曲线变化，焦点只恢复一次；关闭子内容不能夺焦点、不能激活。反向重开保持进度，Reduced Motion 立即完成，主题/资源失效清理 presentation。
- 独立场景验证真实 TextSurface 的 marked 中文/日文文本优先取消、Dialog 第二次 Escape 关闭；AX 写值、Switch Press、Menu part 激活均进入原模型，modal 背景、disabled 与已关闭 part 请求被拒绝。
- fmt、workspace release all-targets Clippy、独立消费者 all-targets Clippy、能力清单一致性、纯 Rust、DS、Gold7 mechanical、motion 与 reference alignment 检查通过。机械门禁不算体验验收。

### 真实组件和后端接入

Gallery 仅在测试里嵌入独立消费者 fixture，使用生产 TextEngine/CoreText 与 GPU Renderer，未迁移生产页面。新过渡测试输出 80 张 PNG：4 主题 × @1x/@2x × 10 个状态，覆盖 enter-start/mid/settled、exit-start/mid/finished、reverse-start/mid、reduce-open/exit。每帧清空 glyph atlas，验证退出期间使用当帧重建内容。完整工作区测试同时回归已有组件接入。

独立原生例子仍只使用公开 controls/platform/text/render API。普通模式与 `TEKES_COMPOSITION_REDUCED_MOTION=1` 各通过 8 项 AppKit NSTextInputClient 回调检查：精确 UTF-16 replacement range、代理对边界 actualRange、marked selection、组字期间 Tab、提交、撤销、首次 Escape 取消、关闭解除 text owner、重开和 unmark；同时检查候选定位矩形有限且非空、点到 UTF-16 索引。覆盖 `A😀B`、`A拼音B`、`A你😀B`、`你😀B`。这是调用真实 AppKit 回调的合成脚本，不能代替系统输入法和候选窗实验。

原生窗口 AX 实测通过：List 的 AXSetValue 写入中文/emoji，Switch AXPress，Menu AXPress 展开及选择 Team；Dialog 的模态树隐藏背景、空值 Save 触发生产 Field.error 并聚焦输入、有效值 Save 关闭后焦点恢复 invoker、重开保留 `共享空间😀`。AX 树与窗口截图独立保存；首次自动操作因重新读取完整树后仍使用旧索引而被工具拒绝，刷新索引后重试成功，未发生组件错误。关闭日志明确出现 `open=false presented=true`，随后 `presented=false`，Reduced Motion 关闭立即结束。

### Gold7 核对

| 项目 | 契约与接入 | 体验及边界 |
| --- | --- | --- |
| G1 / G2 / G3 | 沿用 JSON Palette、共享 Dialog 表面、scrim、角色圆角和既有焦点 gutter | 浅色原生窗口与四主题部分 GPU 样图抽查通过；没有重设视觉方向。退出采用现有逐 primitive opacity 混合，半透明期可见背景，未引入独立离屏组透明方案。全部显示器与品牌体验未测 |
| G4 | Rich Dialog 已接现有 OverlayPresence；开启、退出、打断重开、Reduced Motion 的公共测试及真实 GPU 帧通过 | 原生日志证实持续请求退出帧且最终停止；静态样图确认阶段变化。物理显示帧节奏、动画主观流畅度未验收，性能本批排除 |
| G5 | TextInput/组字单一 owner；原生 UTF-16、replacement/candidate-rect 查询和首次 Escape 路径通过 | 物理中日文 IME、系统候选窗、跨窗口/屏幕定位未测；不可把回调脚本写成物理 IME 通过 |
| G6 | 公共语义快照、原生 AX 树和原模型动作回路已实现；模态范围/恢复原生观察通过 | AX 基本操作通过；VoiceOver 朗读、导航及编辑选区矩阵仍未验证，AXSelectedTextRange/富文本 TextMarker 等更完整文本辅助协议未实现 |
| G7 | 独立 workspace、真实生产控件与公共平台接入、可运行预览保持成立 | 本批最小场景已实现待完整验收，不升级全部 registry 或授权扩大迁移 |

## 原任务状态与下一步

| 原验收项 | 当前状态 |
| --- | --- |
| #14 Rich Dialog 进入/退出、退出帧绘制、事件隔离、焦点恢复、Reduced Motion | 已实现待验收：列出的 API/GPU/原生局部证据通过，完整体验仍有上表边界 |
| #14 / #12 IME 原生接入 | 部分完成：回调范围矩阵通过；物理输入法/候选窗未验证 |
| #5 / #14 原生可访问性 | 部分完成：最小场景原生树和动作通过；VoiceOver 与文本辅助矩阵未验证/部分缺失 |
| #12 全组件逐页统一验收 | 部分完成：本批仅增量最小场景证据，其他组件/页面不据此晋级 |

后续先执行物理 IME/候选窗和 VoiceOver 最小场景验收，根据具体失败修正既有共享能力，再决定更大迁移。上述未满足条件继续归属原 Issue，不创建重复任务。性能继续暂缓。没有自动提交、推送、打 tag、关闭 Issue 或发布；本批未操作原 Gallery 应用或其他 checkout。
