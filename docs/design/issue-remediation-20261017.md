# #13：Issue整改盘点与执行记录（2026-10-17）

基线是活跃 `gallery-design-baseline/TekesUI` 的全部内容，包括未提交WIP；不是仅使用 `bc3de79`。已核对GitHub开放Issue、全部状态PR、Issue评论和cross-reference：14个开放Issue、0个PR；评论无新验收记录。完整快照位于 `design/reviews/issues-20261017/github-*.json`。本轮没有GitHub写入。

主checkout的21个WIP文件字节未变；活跃checkout原1203个WIP文件无丢失，原有改动完整备份于 `/tmp/tekesui-handoff-20261017/{gallery,main}-wip.tar.gz`。在既有WIP上继续修改的文件仅为effects.rs、workbench.rs及重新生成tokens.json，其余原WIP字节未变。

## 依赖与顺序

1. #2/#6恢复设计真源/完整性门禁，本轮已实施；远端CI须用户提交推送后才能验证。
2. #14先审计容器与共享架构；当前矩阵见[架构审计](container-composition-audit-20261017.md)。公共核心保持一个实现负责人。
3. #1从共享语义与绘制推进，#3/#4/#9复用既有焦点、输入、浮层能力修缺口；独立门禁与能力清单可并行。
4. #7消费者品牌入口、#8完整能力映射、#10阅读controller、#11草稿协议沿现有实现补公共边界，不重做WIP。
5. 组合基础后做语义内容槽及窗口Toolbar；#5语义树/平台桥与控件树绑定，不能用演示模型冒充。
6. #12统一验收所有适用组件与页面。性能按用户要求暂不测；不扩大新组件范围。

## 当前状态

| Issue | 状态 | 剩余边界 |
| --- | --- | --- |
| [#1](https://github.com/TekesApps/TekesUI/issues/1) | 部分完成 | 45组件角色表已记录；完整组件语义解耦、全部状态/品牌/@1x/@2x及人工视觉验收仍待完成。Search保留必要输入边界，未以局部改色处理。 |
| [#2](https://github.com/TekesApps/TekesUI/issues/2) | 已实现待验收 | 本地完整检查结果见verification；未推送，远端CI仍为旧基线失败，不宣称远端已恢复。 |
| [#3](https://github.com/TekesApps/TekesUI/issues/3) | 已实现待验收 | 后续WIP已实现；真实物理键盘、CJK IME/多行策略仍待验收。 |
| [#4](https://github.com/TekesApps/TekesUI/issues/4) | 部分完成 | 本轮修复CommandPalette关闭与迟到文本回归；共享库级浮层焦点协议、其他浮层全路径和真实IME验收未闭合。 |
| [#5](https://github.com/TekesApps/TekesUI/issues/5) | 未实现 | 先定义共享语义树、真实控件行为action与生命周期，再接macOS AX桥并做VoiceOver。 |
| [#6](https://github.com/TekesApps/TekesUI/issues/6) | 已实现待验收 | 本轮建立构建绑定/严格验证及负例，CI脚本接入；远端运行须在用户提交推送之后。 |
| [#7](https://github.com/TekesApps/TekesUI/issues/7) | 部分完成 | 现有Gallery品牌不能证明外部consumer可接入；交付策略、双消费者独立构建与CI仍缺。 |
| [#8](https://github.com/TekesApps/TekesUI/issues/8) | 部分完成 | 当前清单可生成并校验；缺真实公开re-export编译绑定、完整token映射与历史40→45语义映射。 |
| [#9](https://github.com/TekesApps/TekesUI/issues/9) | 已实现待验收 | WIP已有真实production动画及GPU帧；仍需当前源下完整原生中断/失活/Reduced Motion人工验收。 |
| [#10](https://github.com/TekesApps/TekesUI/issues/10) | 部分完成 | WIP的Gallery阅读适配器保留；公共可组合阅读controller未抽出，不重复做已有append/reflow。 |
| [#11](https://github.com/TekesApps/TekesUI/issues/11) | 部分完成 | WIP的真实原子文件适配器保留；缺消费者可实现的公开协议和独立consumer/network乱序示例。 |
| [#12](https://github.com/TekesApps/TekesUI/issues/12) | 部分完成 | GPU与AppKit脚本证据独立记录；全页面人工/物理键盘/IME/VoiceOver未通过。性能按本轮用户要求排除，不列为失败也不冒充已测。 |
| [#13](https://github.com/TekesApps/TekesUI/issues/13) | 部分完成 | 总览；以当前未提交工作树逐项核验，不按bc3de79旧报告关闭。 |
| [#14](https://github.com/TekesApps/TekesUI/issues/14) | 部分完成 | 容器矩阵完成；通用组件树协议、语义解耦和窗口Toolbar尚未实现，先公共组合基础后消费者A-F。 |

每条原验收条件及其五态判断见 `design/reviews/issues-20261017/issue-status.json`。未满足条件继续归原Issue，不创建重复缺陷或自动关闭。

## 本轮修复的根因与共享边界

### #2

质量门禁：当前设计真源与历史原包完整性被同一断言混淆。

test-merge-design-system.py分离当前作者输入验证与原ZIP逐字节完整性；保留原包检查。

本地完整检查结果见verification；未推送，远端CI仍为旧基线失败，不宣称远端已恢复。

### #6

质量门禁：生成投影内容未绑定，输入hash清单不完整仍可消费。

复用tokens merge receipt，将brands.json及generator纳入必需哈希；构建前严格品牌校验；CI离线重生成比较。

本轮建立构建绑定/严格验证及负例，CI脚本接入；远端运行须在用户提交推送之后。

### #1

设计语义：现有必要/装饰/焦点边界已有，但动作角色分配缺失。

layout.json#/control_emphasis → design-export effects → proposal::control_boundary → paint::button_variant_with_interaction / TextSurface::draw；复用现有token、线宽、状态与焦点，不新增颜色token。

必要输入边界、错误、选中提示保留；Primary填充强调不弱化；Panel装饰/elevation、Checkbox/Switch/Slider自身几何按原语义；IC不凭空给optional action加框。

### #4

公共组件契约的宿主接入：关闭模型状态与文本焦点生命周期不同步。

复用ComboBox/CommandPalette模型、TextDemo焦点和现有Menu取消；集中StoryState open/close宿主接线，不新建输入状态机。

IME marked text优先；关闭的面板不发布输入状态；其他浮层及库级契约仍需逐项核验。

### #8

公共组件契约与治理：规范、公开API、registry和证据视图脱节。

从现行workbench、源声明和Gold7记录生成capabilities；schema引用当前入口，历史来源保持原样。

当前清单可生成并校验；缺真实公开re-export编译绑定、完整token映射与历史40→45语义映射。

#2的实际差异包括历史包之后有意调整的border-focused、status foreground、shadow-ink、checkbox radius/size/label-gap和focus尺寸。当前作者JSON仍为真源，原包134个文件与ZIP逐字节完整性检查继续保留，不回退当前设计以迎合旧输出。

#1新增的是原有边界类别到动作角色的映射：现有定义能表达必要/装饰/焦点，却无法表达“Quiet是optional enclosing boundary”。没有新颜色token、新状态机或通用框架。全部45组件的适用角色与例外见 `visual-component-coverage.json`；这是一张源码审查/迁移表，不是全部组件体验通过声明。

公共API、真实组件接入、最终体验分开记录。验证命令、退出状态、GPU/native范围和限制见 `verification.json`，源文件与产物哈希见 `source-manifest.json`。原包、历史golden及旧验收收据不被本轮结果覆盖。

## 验收边界

本轮保留520张整改前整页画面；最终候选保存1646张真实GPU画面，其中1104张覆盖38个组件的46条状态/变体路由、四主题、两种窗口/字号和top/mid/bottom位置；其余包括520张代表性整页画面及组件特定回归图。逐路由清单见 `page-validation.json` 和 `visual-after-final/state-pages/manifest.json`。未覆盖的页面继续待验收，不把有非背景像素等同于组件体验正确。仍不是完整45组件所有状态与物理@1x/@2x验收。真实AppKit扩展脚本另留34张窗口截图、4项文本断言通过；脚本的Click/Key动作不能代替物理键盘、真实CJK输入法候选窗或VoiceOver。

抽查Light Settings整页确认轻动作边框减少而输入/主要操作仍明确；抽查Dark窄窗口与IC Button状态，确认必要输入边界与独立焦点仍可见。这是有限视觉审查，不把Gold7 experience统一改成pass。

未自动提交、推送、打tag、关闭Issue或发布。性能未测；原生AX桥缺失使VoiceOver条款依赖#5。

最终源冻结后：867项Rust测试通过、0失败、1项既有可选忽略、2项性能测试按范围过滤；fmt、严格Clippy、19项离线设计/完整性检查和Rust投影CLI生成/检查通过。远端CI仍是bc3de79上的既有失败，本轮不推送，不能宣称远端已经恢复。

原生脚本曾在ComboBox提交后报告text=None，失败完整保存在 `native-failure.log`。工作台完整Preedit→Commit→moveDown:→insertNewline:→重绘回归通过；清理本仓旧debug构建产物、按同一源码重建后完整脚本通过（`native-clean-check.log`）。没有据此盲改共享路由或隐藏失败；完整原生矩阵仍待验收。最终源码仅比该原生候选增加这一自动回归，生产逻辑未变；两份源清单/快照均保留。
