# Separator：复用装饰边界规则传递真实倍率

根因属于公共组件契约与宿主输入适配。JSON 已规定普通装饰线使用 `1 / backing_scale`，增强对比度使用 `stroke-rule`；`paint::decorative_border` 已实现该规则。Separator 固定画1pt，SeparatorView 未接收倍率，导致普通 @2x 的装饰线仍占2个物理像素。原始144个真实GPU场景中36个与已有规则不符，保留在 separator-scale-diagnostic-20261021；不是新造一套视觉标准。

共享修复位于 `extended::separator_at_scale` 和 `SeparatorView::render_at_scale`，直接复用现有装饰线解析。Gallery 的普通展示与变体展示、横线与竖线均转发 TextEngine 的实际 backing scale。没有新 token、全局状态、协议或抽象层，布局框、祖先裁剪与命中范围不变。

旧 `separator` / `SeparatorView::render` 保留1x默认值，兼容不知道倍率的既有调用方。知道真实倍率的宿主须采用 at-scale 入口；这是传递宿主输入，不是手工覆盖颜色或线宽。例如：

```rust
separator.render_at_scale(scene, frame, Axis::Horizontal, clip, palette, text.backing_scale());
```

适用于独立 Separator 及使用公开 painter 的装饰分隔。合理例外：增强对比度继续原 `stroke-rule`；SplitView 拖动分隔条承担独立输入／几何职责，保留既有路径，不自动调细；其布局与线宽有兼容回归。必要控件边界、焦点和选中提示不使用该装饰规则。

验证分层：

- 公共 API：4项新回归，覆盖真实倍率及无效倍率退回、1x兼容入口、布局与颜色角色、祖先／不可用裁剪及SplitView例外。416项controls测试和严格Clippy通过。
- 真实接入：实际Gallery普通展示和variants在4主题、@1/@2共16个接入场景通过；Gallery严格Clippy及168项独立消费者测试／严格Clippy通过。
- 真实GPU：4主题、3品牌、两轴、@1/@2及三个逻辑相位偏移共144场景／288帧。当前画面全部与既有共享规则参考一致；36个普通@2场景改变，其余108场景与修复前逐像素相同。相位偏移单位是逻辑pt，实际物理相位随倍率换算，不宣称穷尽所有像素相位。
- token：14份真实重跑观测共5832有界场景，包含144个Separator观测；complete=false。44项治理、8项组合及3项生成绑定检查通过。保留原历史状态要求，没有用新profile覆盖旧要求。
- 原生：单组件小宿主在U2790B @2实际激活，四主题各自的scene线宽、事件、源码／二进制身份及原生PNG通过核对。普通主题右侧真实scale路径为0.5pt，左侧兼容1x参考为1pt；IC两侧均1pt。窗口已关闭，清理另留证。

Gold7：G3的有界规则、真实接入、GPU及外接屏原生渲染分别留证；截图不代替主观物理清晰度、跨显示倍率、完整页面或ClientKit匹配验收。G7有公开API、兼容说明和可重建小宿主；整体成熟度仍为experimental。

证据目录 `design/reviews/separator-scale-20261021/`，入口 progress.json、native-acceptance.json、comparison.html。观察输入身份 `f07266652b0a55118d6e3377fb80de5037b7df0a59e67f0cc9bcd5fb47b3692f`；owned-production.patch仅相对本轮WIP前态，不混入其他整改。

集合候选154份原始输入已另存为collection-final-native-20261021/verified-source，原生验证器绑定该冻结输入并明确列出后续WIP变化。其重新激活检查仍待完成；不把本轮改动倒灌为已有通过证据。AgentKit冻结包不变。不测性能，不提交、推送或关闭Issue。

## 2026-10-28：普通 Content 的独立接入（#54）

上文统计与原生观察保留2026-10-21来源，不转签本轮。新增公开 `composition::SeparatorContent::new(Axis)`（默认横向），通过原 `SeparatorView::render_at_scale` 消费同一 Palette/backing-scale 装饰规则。父约束提供有限长轴，短轴按原 stroke-rule 钳制；只有实际装饰墨迹溢出短轴时才加叶子裁剪，正常非整数位置保留原 View 的祖先 device-scissor，避免旧 a1 的64例丢线。没有第二 painter、token 或业务状态机。

该被动叶子不进入语义树、Tab、点击、滚轮、文本服务或焦点；它按当前原事务接口声明 StableHooks/有限 local prepared effects。实际 checked pane 新增/退休与原 reading、Composer marked/binding/selection/焦点及 Enter 门禁组合已验。曲线子树保留默认 Unsupported，本轮不新增曲线能力。

最小公开消费样例见 `examples/composition-consumer/tests/support/separator_fixture.rs`，普通 Text→Separator→Text 树与9项 public 测试在独立消费者工作区执行。原 scale、当前 API binding/治理与strict通过；原 a2 完整8帧256例在当前 main 重跑，192正常全ROI与原View零像素差、所有clip外零像素，root已查看8图。两个测试lint修正分开留有失败与重跑身份；其余未变输入沿用原检查。详细当前程序/源码/图像收据在SSD `artifacts/issue-54/separator-content-main8de6c24-candidate/main-intake/`。

这只完成#54本仓库组件与前三项实际接收；不可变公开交付和AgentKit精确pin接入仍依第4项分别完成。原生窗口、物理IME、VoiceOver、完整Gold7和性能未在本轮执行，不追加为本票前提。
