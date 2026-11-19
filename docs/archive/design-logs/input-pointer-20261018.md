# TextField / TextArea / SearchField：hover 与 pressed 共享契约

2026-10-18，活跃 `gallery-design-baseline/TekesUI`，`bc3de79466834e99062a1b4ef0872323e2860001` 加全部 WIP。GitHub 当前仍有14个开放 Issue、全部状态0个PR，#14正文更新时间为 `2026-10-17T08:15:16Z`。本批继续原 #1/#8/#12；不新增组件、不扩大组合迁移。证据入口：`design/reviews/input-pointer-20261018/receipt.json`。

## 根因和共享契约

根因属于共享设计语义和公共组件契约：input state profile 已列 hover/pressed，但当前补充契约没有解释输入框的按压语义；TextSurface 已有命中与选区/滚动捕获，却没有公共状态快照。Gallery 因而过滤掉两个状态。这个缺口无需新建状态机或引入按钮呈现。

唯一作者输入为 `design/source/interaction-contracts.json` 的 `text_surface`，按既有工具生成 `design/tokens.json`。原导入包保持不可变，既有 token 值完全未变。补充规则如下：

| 状态 | 语义与反馈 | 结束与例外 |
| --- | --- | --- |
| hover | 当前指针命中可见 frame，包括输入内边距、受祖先 clip 限制；不改变编辑内容、选区或 IME | 离开或无指针位置即 false；disabled 为 false；read-only 允许选择，因此可 hover |
| pressed | 接受主指针 down 后，既有选区/光标或滚动条捕获生效；即使拖出 frame 仍 pressed | up/cancel/blur/失活/卸载/禁用时释放捕获；disabled 状态投影立即 false；read-only 仍可捕获选择 |
| presentation | hover/pressed 保持输入底色、必要边界、圆角和尺寸；使用现有 caret、selection、scrollbar 与独立焦点提供定位反馈 | 不使用 Button wash、品牌 tint、缩放、按压动画或 release activation；错误边界不被 hover/pressed 覆盖 |

`TextSurface::pointer_state(Option<Point>) -> ControlState` 复用现有类型，只从可见几何、TextInput.disabled/error 和原 pointer_dragging 导出快照；没有新增存储字段、token、协议层或编辑状态机。None 表示不 hover，捕获仍可 pressed。焦点仍由宿主传给 draw；宿主更新 disabled 等生命周期时调用原 cancel_interaction。现有 Gallery/组合宿主已有 blur/cancel 路径，未另建生命周期实现。

适用于三个输入变体及使用 TextSurface 的现有 TextFieldContent。TextArea 保留多行/纵向滚动，另两个保留现有单行路径；read-only 允许选区和复制、禁止编辑，disabled 禁止新编辑/捕获。普通 Text/阅读场景保留自身职责，不因此新增激活状态或扩入迁移。原有 IME 行为保留：hover 不触碰 marked text；accepted down 沿用原 unmark 事务；cancel 释放捕获但不回滚已可见的草稿或选区。宿主光标形状与物理键鼠体验未在本批自动宣告通过。

## 公共 API 与真实组件接入

库外 `input_contract.rs` 两项测试通过。矩阵扩为612例：3个输入别名×四主题×三个既有品牌×17个状态/组合；包括 hover/pressed、read-only、disabled、error 和 composing 的交叉。验证公开快照、hover 不变更 IME/选区、禁用投影、release/cancel 不触发按钮动作或改变输入皮肤，并保留原有编辑/撤销回归。另一个用例覆盖祖先裁剪、拖出边界仍捕获、释放后选区保留及禁用/取消释放。公共探针使用真实 CoreText shaping，glyph painter 为空；像素验证单列。

独立 composition-consumer 在已有 Rich Dialog 的真实 TextFieldContent 上执行 PointerMove/Down/Up/Cancel；覆盖单行/多行、可编辑/只读，核对越界捕获、焦点与草稿/选区保持。该消费者共16项通过（1 lib + 15 scenarios）。没有新增 Search 专属适配器或复制编辑器；SearchField 沿用已登记的单行别名。

Gallery 状态模板对三个输入变体恢复 hover/pressed；pressed 通过同一个 TextSurface.pointer_down 建立捕获，并展示定位光标与独立焦点，hover 保持未聚焦。屏外比较夹具先在完整 frame 建立捕获，再恢复真实祖先 clip，等价于捕获后移出视口；它仍是比较夹具，不冒充原生输入验收。新增实际状态夹具回归及既有 pointer/disabled/read-only、失活、IME/选区夹具回归均通过。

## 真实 token 依赖核验

复用已有显式 token-audit 工具，实际重跑612个输入场景和2016个选择族场景，在新目录保存源码绑定观测，并更新当前清单引用。旧两批证据未覆盖，也没有只改旧观测的哈希。

三个输入别名各204例、各12个实际查询 ID。逐组件/主题/品牌对照，hover 与 rest、pressed 与 focus 的查询集合共72组完全相同，没有查询 Button token。`token-state-comparison.json` 保存结果。这确认本批复用输入边界与独立焦点，没有将 Button 外观带进输入族。

这仍是有界运行时 accessor 观测，`complete=false`；编译常量、已展开 alias、TextRole style、宿主字体内部及未执行分支仍不在观测范围。完整依赖状态继续 runtime-unverified、tokens=[]。颜色/尺寸值未改变。源哈希、真实 pointer flags、四主题和未知 token 门禁继续生效，32项治理测试通过；仅填写状态名不能将 hover/pressed 升级为已核验。

当前45组件、269项要求不变：68项夹具已源码核对，201项待核对；49项公共API状态已核验，其中输入27项、选择族22项。所有 experience 仍 unverified，没有提升组件成熟度或按局部结果关闭原 Issue。

## Gold7 及证据边界

| 核对项 | 结果 |
| --- | --- |
| G1/G2/G3 | 保持中性输入层级和必要边界，不新增按钮填色或强调。三页四主题、两窗口/字号、top/mid/bottom、@1x/@2x共288张新整页GPU图。前态288张复用上一批 after 图；开始时核验其580项源码清单与当前基线完全一致，并保存本批基线哈希，所以前态没有重新渲染。抽查浅色TextField完整矩阵前后、深色IC大字号TextArea和浅色IC大字号SearchField。 |
| G3/G4 | 复用原文本像素用例，新增 hover/pressed 路径，16个主题/单多行/倍率组合各两种状态，共32组真实GPU逐像素相等断言，比较状态结束前后且保持同样焦点/选区。实际字形图共96张，含既有64张光标/选区/禁用/只读回归；未新增动画。抽查单行pressed@2x和多行hover。 |
| G5/G6 | 光标、选区、错误和中性焦点仍独立；hover不改IME，捕获取消不丢草稿。公共模型、独立消费者和实际Gallery分别留证；没有把offscreen渲染当物理键鼠、系统候选窗、跨屏或VoiceOver通过。 |
| G7 | 共享JSON、公开快照、实际夹具和源码绑定token证据一致；状态要求没有删除或用例外静默豁免。 |

控件/tokens/design-export默认295项测试、独立消费者16项、四项针对性Gallery状态/捕获/失活回归、两个倍率整页GPU测试及一个文本GPU测试通过。workspace与token-audit严格Clippy、fmt、设计生成、DS、Gold7机械检查、reference alignment、纯Rust和能力生成检查通过。未重跑全workspace测试，未运行性能基准或采集性能验收数据。

开始时10230项文件记录哈希；本批有意修改13项既有文件，其余10217项逐项保持。源码、前态副本、截图配对和收据单独归档。没有提交、推送、tag、创建/关闭Issue或发布。

后续继续核验其余组件族的状态与真实token依赖；输入族物理hover/拖选/失活、IME候选窗与VoiceOver仍须分别原生验收。原 #1/#8/#12 的其余条件保留在原 Issue。
