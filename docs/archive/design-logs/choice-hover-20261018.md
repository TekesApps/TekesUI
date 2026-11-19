# 选择族：CommandPalette 触发器状态与焦点移交

基于 HEAD 加全部 WIP，继续原 #3／#8／#12／#14。GitHub 当前仍为 14 个开放 Issue、0 个全状态 PR，原 Issue 更新时间未变。上一批弹层几何证据保持冻结。

## 根因与共享修复

| 根因层级 | 修复与适用范围 |
|---|---|
| 公共 View 交互契约 | CommandPaletteView 的 pointer_move 只转发候选菜单，遗漏私有触发器。现在先核对现有可用性，再把移动转发给原 Button；所有该 View 的消费者获得 hover、拖出撤销 pressed 和移回行为，不新增状态机或样式。Select 已有触发器转发；ComboBox 的输入由宿主 TextSurface 持有，不能套用按钮契约。 |
| 公共组件焦点契约 | 现有整体 blur 会关闭查询，不能表达触发器到查询框的焦点转交。新增精确的 blur_trigger，内部仅复用 Button.blur；整体 blur 仍关闭弹层。独立消费者验证 open、查询和候选布局保持，并验证已失焦触发器不会通过 Enter 清空查询。 |
| 真实宿主接入 | Gallery 的统一 owned focus 原先没有同步 CommandPalette；现在同步冷启动 Tab 焦点、打开后的查询接管，以及关闭后的恢复。拖出释放原先只在命中触发器时交给 View，导致移回残留 pressed；现有 ExtendedStory 释放分支现在先结束原触发器捕获，再处理候选。 |

没有新 token、状态层、协议类型或组件迁移。样式仍来自既有 Button 原语及 JSON 定义；本批修复事件和焦点接入，不以局部传色修饰状态。触发器、查询框、候选菜单分别保留既有所有权，这是合理差异，不能从一个触发器的状态推定整个选择组件已完成。

## 公共 API、真实接入与体验

- 公共 API：hover／拖出回归在修复前失败；当前公开测试验证原始 Scene 中的状态变化、拖出释放、新移动与禁用，以及焦点转交保留查询。另以 84 个有界案例验证五项精确触发器状态和两种取消场景，覆盖四主题／三品牌；期望 token 读取在捕获窗口外，不向生产读取记录注入依赖。
- 真实接入：控件、token、design-export 三包启用 token-audit 共 336 项测试通过，其中默认控件包 293 项、开启观测的控件包 294 项；独立消费者 31 项通过。外部 crate 使用 CoreText 测量与原 View，但文字测试后端不绘字，不将其称为像素验收。严格 Clippy 通过。
- Gallery：冷启动键盘焦点与实际指针事件各有红绿回归；后者在公共 hover 修好后继续暴露宿主 outside-up 遗漏，已在原接入层修复。CommandPalette 7 项相关测试及导航 States 2 项通过，覆盖首次 Enter／Space、查询接管、Escape 恢复、hover／pressed、拖出释放、移回与新点击。
- GPU：现有 CommandPalette States 增加原 View 的 closed hover／pressed／keyboard focus，open 样例只让查询框显示焦点。四主题、三品牌、1×/2×、两种宽度／字号和三个滚动位置共 144 张最终整页图；与上一批同名 144 张均不同，主要原因是新增行及焦点归属修正，不把差异数当作质量分数。另有包含所有样例的 360×2000 实测绘制图及其他导航回归图。抽查文字、焦点环、层级和既有边界关系；未完成所有状态组合的最终体验验收。
- 原生：前后应用均校验签名后的二进制身份。旧版首次 Tab 到触发器没有焦点环，Space 不打开；新版首次 Tab 焦点可见，Space 打开，查询接管焦点，Escape 恢复。Gallery 内 Reduced Motion On 的全新会话也确认 Enter 打开及 Escape 恢复；没有进入系统设置。

原生工具的 drag 会附带 mouse-up，旧版也能在释放时得到 hover，因此这组原生截图不证明纯移动修复。精确的 Moved／outside-up／Moved 时序由公开方法及真实 Workbench 事件测试证明；原生截图仅证明已经实际完成的有界操作。首次 Reduced Motion 尝试焦点落在其他页签，未接受为通过；随后从新会话确认触发器焦点再重验，接受的文件名为 `reduced-confirmed-*`。

Gold7 分层记录：G1–G3 核对原有文字、层级、边界、焦点和裁剪关系；G4 有 Reduced Motion 的原生功能证据，未宣称全动效验收；G5–G6 验证查询保留、键盘接管及取消路径；G7 有公共 API、独立消费者、真实 Gallery 和文档。机械检查通过不等于完整体验通过。原生 AX 仍只暴露窗口与菜单，控件语义／VoiceOver／物理 IME 未因此晋级。

## 状态与真实依赖

仅 CommandPalette 触发器的 rest、hover、pressed、focus、disabled 五项增加 source-reviewed／public-api-verified，experience 全保留 unverified。公共状态累计 100，夹具待核验 156，269 项原要求不变。选择族仍余 25 项，下一步继续 Select／ComboBox 的真实状态，再逐项核验 selected／open／empty 和调用方 loading／error 组合，不新建状态字段来掩盖宿主组合缺口。

84 项触发器观测使用现有 action_state 断言路径，明确 part=trigger、complete=false。共享源码变化后，动作 576、输入 612、选择 2016、导航＋内容 408、几何 192 项均重新执行并校验源散列，历史观测不覆盖；仍不宣称完整 token 依赖已经提取完。

代码、红绿日志、GPU／原生截图、GitHub 快照及 WIP 保护记录位于 `design/reviews/choice-hover-20261018/receipt.json`。所有未满足的原验收条件继续留在原 Issue；未测性能，未提交、推送、打 tag、关闭 Issue 或发布。
