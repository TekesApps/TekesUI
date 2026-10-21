# #23 无表单装饰文本内容（进行中）

根因涉及公共呈现契约和基础交互，不是一个页面的边框。TextFieldContent 固定输出 Field label/message，TextSurface 固定输出输入背景、边界、内距和按钮最小高度；其裁剪后的 inner 同时决定换行。禁用双轴的文本仍吞精确滚轮，ContentHost 又把长正文整体作为焦点 reveal 目标。

当前复用既有 TextFieldContent / TextSurface，新增 opt-in TextSurfacePresentation::Content 与 read_only_content(name, value, TextRole)。默认 Field 保留。内容模式使用同一行指标、测量/布局/选择/绘制几何，不输出表单装饰；父 Surface/ScrollContent 决定背景、padding 与唯一文档滚动。共享焦点/选区墨迹是合理保留，焦点并非输入背景。禁用双轴的所有 TextSurface 都把滚轮交回祖先；Content 焦点/只读插入点导航复用既有祖先 reveal，避免选中首行却显示整个长块，也不把手动滚动拉回旧插入点。

适用：用户显式选择的只读、多行、可选中文本块；不自动迁移普通表单、已限制视口的编辑器或整个组件族。没有新增组件/路由，registry 仍为45项。没有复制编辑、布局、焦点或滚动状态机，没有新 token。语义 name 和 TextRole 由调用者提供，既有受控值/版本拒绝、UTF-16、指针与 AX 继续复用。

独立消费方以既有 SurfaceView + StackContent 包裹正文、按钮和 footnote，外层 ScrollContent 拥有滚动，并保留普通 TextField 作对照。调用者拥有 DraftRevision 并通过原事务追加。原生小宿主 plain 场景沿用原 CoreText/Metal/平台桥；复制只桥接既有 selected_text 与 Window clipboard。该测试入口使用平台已有的进程私有 pasteboard，不读取/覆盖用户总剪贴板。

证据目录 design/reviews/plain-text-content-20261019/ 保留新增API与真实消费者测试、首行聚焦失败回归及修复、48个内容模式token案例和七组实际重跑。首次夹具类型/方法名编译错误与审计刷新拒绝记录保留，未算成产品通过。当前版本 Gallery 集成417项通过、1项既有忽略。外接屏真实GPU已核对默认/窄窗正文、普通Field对照、换行与父级裁剪；父偏移0→619pt，正文内部偏移始终0，换行宽度/段落高度在滚动前后保持303/912pt。活动窗口下的选区/复制、追加动作与Gold7仍待验，未把失活时被拒绝的AX操作算作通过；#23尚不可关闭。较早batch2/3独立包是另一冻结身份，不宣称含本轮新增模式。性能不测，不提交/推送/关闭Issue。


## 公共消费者接入

实际可构建样例：`examples/composition-consumer/src/plain_text.rs`，公共 API 回归：
`crates/tekes-ui-controls/tests/plain_text_content.rs` 与
`examples/composition-consumer/tests/plain_text.rs`。原生入口为 `native plain`；依现有小宿主流程构建，
不要求消费者依赖 Gallery。这里没有单独的 Agent 模型，也没有强制业务气泡样式。

```rust
use tekes_ui_controls::composition::TextFieldContent;
use tekes_ui_tokens::proposal::TextRole;

let body = TextFieldContent::read_only_content(
    "Selectable content", // 可访问性名称，不额外绘制一行 label
    "中文😀 e\u{301} content",
    TextRole::UiBody,
);
```

把同一个 adapter 作为现有 Content 树的叶节点；普通 Surface 决定背景和内距，Stack 组合按钮/footnote，
ScrollContent 决定有限视口与唯一文档滚动。不要再为这个 Grow 正文包裹独立内部滚动区域，也不要
把它的布局宽度替换成祖先裁剪矩形宽度。保留原 ContentHost 的事件、焦点和语义路由。
新模式的构造器将只读、多行、Grow、双轴不滚动设置为一组一致配置；普通
`TextFieldContent::new` 的默认 Field 外观和编辑方式不变。

数据身份和版本继续由消费方持有 DraftRevision。追加或替换调用原
`surface.apply_external_value(baseline, next, ExternalSelection::Preserve, ExternalUndo::Preserve)`；
异步工作必须携带发起时的 baseline，不能在迟到回调时取一个新 baseline 掩盖冲突。检查返回结果，
只在 Applied 时推进调用方权威版本；不得通过直接重写 text/selection 绕过版本或区间规则。
样例的 Append 按钮通过既有 typed action drain 触发同一受控更新，非独立文本编辑实现。

原生宿主继续使用已有 prepare_text_input、AX UTF-16 range 与平台文本服务。
复制命令从既有 focused TextSurface 读取 selected_text，交给原 Window clipboard adapter；
不重写选区算法，也不把 readonly 当作 disabled。当前验收 App 使用已有进程私有 pasteboard，
总剪贴板不读取、不覆盖。活动窗口与组字/禁用保护继续生效；文本 AX 节点存在不等于 VoiceOver 已验。

交付身份：本轮全部 WIP/二进制输入见
`design/reviews/component-runs/plain-text-native-20261019/receipt.json`；
逐条原条件映射见 `design/reviews/plain-text-content-20261019/acceptance-matrix.json`。
较早 `4bb69cb08ff8…` 独立源码包不包含此模式，消费方不能用该包或此前正式 pin 推断 #23 已实现。


## 最终本地增量验收与移交

新增GPU回归 `independent_plain_content_has_no_form_pixels_and_keeps_real_selection_in_four_themes`
复用既有Gallery后端和独立消费者。空内容逐像素等于clear帧（语义name不绘制），普通Field仍绘制；
非空中文/emoji选区在实际shaping范围内产生像素变化，四主题×两倍率×两宽度×两字号×三阶段共96帧。
隔离重放96帧逐张一致，严格Clippy、公开API3项、核心238项、消费者139项和38/8项治理测试通过。
首轮夹具Option类型错误与后续并行WIP引发的工作树门禁失败均保留，不混成产品通过。

活动外接屏原生已通过：UTF16[8,12]选择“中文😀”并经私有剪贴板复制；实际按钮追加到version2后
Shift+Tab回正文保留选区；窄窗宽度303pt、父滚动635pt且内部偏移0；Cmd上下到文末1790/文首0；
聚焦后的只读字符输入不改文本/选区、不建立marked。文首父偏移16pt仍完整显示首行；不要求显示外层装饰padding。
初次字符探针没有焦点，未算通过，立即聚焦后的单独复验通过。VoiceOver/物理IME/性能未测。
最终小宿主 `plain-text-active-final-20261019` 已接受并关闭清理；62个独立测试产物约108MB另行回收。

期间检测到并行Scene3D Gallery改动，全部保护，没有纳入本轮、没有通过改测试计数或盲重绑来掩盖。
复验/归档使用精确声明来源的隔离源码身份
`2f0902d9f90fb069696c52edc38d9b7e7f8f0d0c4a3f0141e21fe1ac3f3183d0`，
不是对整个最新工作树的统一通过结论。入口为
`design/reviews/plain-text-isolated-20261019/HANDOFF.md`、`acceptance.json`。
#23本地分层条件已覆盖，已移交AgentKit独立复现/消费者适配复审；仍待其结论与明确授权下的公开交付。
较早段落保留了当时pending状态，以此最终增量为当前本地结论；未关闭Issue。
