# Select／ComboBox：实际状态、关闭点击与确认键

基于 `bc3de79` 加全部 WIP，继续原 #3／#8／#12／#14。刷新 GitHub 仍为 14 个开放 Issue、0 个全状态 PR，更新时间与账本一致。前批证据不覆盖，不关闭原 Issue。

## 根因、共享修复与合理差异

| 根因层级 | 修复位置及影响范围 |
|---|---|
| Select 公共组件交互契约 | SelectView 在关闭已打开菜单的 pointer-down 后，又给触发器重新按下，导致同一次释放重新打开。现在复用既有 Menu 的 Dismissed 结果和 Button.cancel，消费整个关闭点击；新点击仍可打开。所有原 SelectView 消费者受益，不新增开关状态机。 |
| 真实宿主接入 | Gallery 额外调用 trigger blur 掩盖了公共 API 缺口。共享修复后，点击触发器的路径不再依赖该补救；点击其他宿主目标时仍由既有焦点同步释放旧组件。 |
| 宿主基础事件适配 | 原生实际操作中，Down 移动候选而 Enter 未确认。完整 Gallery 回归区分硬件键码 36 与 76，后者被遗漏；现有独立原生消费者已经同时映射两者。现在 Gallery 统一输入入口把 76 交给既有 Enter 路径，所有 Gallery 控件与焦点区域复用；平台层仍保留原始硬件事件。 |
| 状态证据与实际测量 | Select 旧固定尺寸样例被同一测量／裁剪模块中的原 View 样例替代，标题、触发器和菜单都用现有测量。ComboBox 补充已有 TextSurface 指针夹具。没有增加颜色、token、状态层、公共类型或新 UI 框架。 |

Select 固定选项触发器、ComboBox 编辑输入与候选菜单分别保留原所有权。ComboBox 的 hover／pressed 复用 TextSurface：输入底色和边界不变，按压处理文字选区并取得输入焦点；不使用 Button 的背景或按压外观。Select 的 selected 由真实选项值及标题表达，不伪造 Button.selected 外观。编辑输入点击与固定选项触发器关闭点击具有不同语义，不强制统一切换行为。

## 公共 API 与真实消费者

- 新增关闭点击红绿回归：修复前失败，修复后点击打开／点击关闭／新点击打开、外部点击关闭、选中值不变通过。独立消费者用 CoreText 和原 SelectView 完成相同流程，不调用宿主 blur 补救，再用 Down／Enter 选择第二项。
- 180 个精确状态案例覆盖四主题、三品牌：Select 的 rest／hover／pressed／focus／disabled／selected／open 七项，以及 ComboBox 的上述七项和 empty。断言原绘制图元、实际标题／查询、选中值、菜单可见性、空过滤无幽灵表面、禁用拒绝编辑。期望颜色读取在捕获窗口之外；输入闭合状态的观测明确不含 Button 外观依赖。
- 公共测试使用 CoreText 排版；字形绘制后端为空，不称为像素验收。独立消费者共 32 项通过；控件、token、design-export 启用 token-audit 共 338 项通过。严格 Clippy 通过。
- 真实 Workbench 回归验证 Select 打开／关闭／新键盘选择，以及两种字号下 ComboBox 的输入底色、草稿和焦点。新增完整 Gallery 回归实际经过页面、点击、绘制和键盘入口，36 与 76 均确认第二项；76 的修复前失败单独留证。它补充了原来只在 Workbench 内验证的边界。

`choice_state` 只用于验收记录，复用现有精确状态证据检查，附显式 `state_asserted=true`；没有新增运行时状态或设计 token。37 项能力门禁包含缺少断言、错误布尔值和合并状态标签不得晋级的负例。

## GPU、原生与 Gold7

两组件 States 在四主题、三品牌、1×/2×、两种宽度／字号和三个滚动位置生成 288 张最终整页图。较早 `popup-geometry-20261018` 冻结的同组件 288 张作为已注明来源的视觉参考；全部变化主要来自补充状态、实际测量与编排，不把变化数量当作质量结论。该参考不是本批开始时重新执行的原始源快照。本批另有 Select 360×1300、ComboBox 360×2000 及其余两个导航组件的整幅绘制图。

原生发现确认键缺口后，重新构建并重跑同一 288 张：与确认键修复前的本批图逐张字节一致。说明统一输入修复没有改变这些既有状态图像；交互是否修好仍由事件回归和原生操作证明。抽查文字、输入与触发器区别、焦点留白、圆角、候选边界和空反馈；没有因此把所有状态组合的体验记为通过。

原生前后应用均校验签名后的二进制身份，仅操作 Gallery：

- Select：原宿主已有 blur 补救，所以原生旧版关闭点击本来能关闭；不以这一画面冒充公共缺口的原生红绿。新的共享实现不再依赖补救，原生打开／关闭保持正常。实际 Down＋Return 最初不确认，Space 能确认；统一映射后 Down＋Return 选择 Compact、关闭菜单，触发器标题和事件日志均更新。
- ComboBox：`Com` 过滤，Down＋Enter 选择 Comfortable，查询保持输入焦点且菜单关闭；改为无匹配查询后反馈可见、无菜单；禁用后点击／输入不改变查询。
- Select 的禁用尝试中，键盘焦点仍在宿主 Disabled 按钮，Enter 因而切换了这个宿主动作；不计为对禁用 Select 的键盘验收。禁用拒绝激活由公共 API 验证，原生该特定路径仍未验证。
- 当前系统 AX 仍只有窗口和菜单；控件级 AX、VoiceOver、物理 IME 未晋级。没有修改系统设置。本批 GPU 夹具使用 Reduced Motion，未新做完整动效体验验收。

Gold7 分层：G1–G3 核对现有关系与真实组件图，不改变共享样式；G4 不从静态状态图推断动效通过；G5–G6 以查询／值保留、关闭点击和确认路径为有界证据；G7 有公开 API、独立消费者、真实 Gallery 与文档。机械门禁通过不等于整体验收通过。

## 状态、依赖和下一项

新增十五项 source-reviewed／public-api-verified；experience 仍全部 unverified。公共状态累计 115，夹具待核验 141，269 项原要求不变。选择族还余十项：Select 的 empty／loading／error，ComboBox 的 loading／error，CommandPalette 的 selected／open／empty／loading／error。

下一步先判断错误、空和加载分别由组件本身还是调用方内容状态负责，复用 TextSurface、既有 Button 状态和公共反馈能力；只在确实无法表达时补公共输入。不得用普通触发器的改善替代这些验收，也不因为本地查询没有异步工作就静默删除原要求。

180 项新观测之外，触发器 84、几何 192、动作 576、输入 612、选择 2016、导航＋内容 408 项均按当前共享源码重跑并核对散列。全部为 `complete=false` 的有界实际读取，完整 token 依赖仍未完成。

代码增量、红绿日志、当前能力／组合审计、GitHub 快照、GPU／原生证据及全部 WIP 保护记录见 `design/reviews/choice-selection-20261018/receipt.json`。未测性能，未提交、推送、打 tag、关闭 Issue 或发布。
