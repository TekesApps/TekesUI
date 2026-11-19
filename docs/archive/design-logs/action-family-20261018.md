# Action family：共享状态与真实 token 核验

基于 HEAD bc3de79 加全部 WIP；范围为已有 Button、IconButton、Link、Checkbox、SegmentedControl。原 #8 仍为部分完成，#12 原生与无障碍验收仍开放。

## 根因及共享修复

- IconButtonView / LinkView 曾在 render 之间的事件中使用过期的可用性，并把过期状态写回调用者模型。现在事件入口同步最新 disabled/loading，复用既有 cancel；禁用或加载中拒绝激活，也不抢焦点。适用于全部这些公开 View 的消费者。Link 不具 loading 契约，Space 继续由页面处理。
- SegmentedControlView 曾保留已禁用项的 Space arm，重新启用后可能误选。绘制和禁用键盘事件现在复用 cancel，并修复失效焦点目标。受影响范围为全部公开分段视图；不改变模型的单项 set_enabled 选择策略。
- LinkView 查询不存在的 spacing:space-button-body-inset 并用数值 8 回退。现在直接读取已有 spacing:space-regular；不新增 token，不套用按钮样式。
- 真实 Gallery 复用现有矩阵，补入非默认选中和全部禁用样例；没有新建状态抽象。

## 分层验证

公共 API：新增 3 项红绿回归和 1 项状态矩阵测试。31 个必需状态与 17 个登记组合，覆盖四主题、三既有品牌，共 576 个执行案例。共享包 309 项通过。公共状态核验累计 72 项，尚有 184 项夹具待核验；不能将登记完整解释为实现或体验完整。

真实接入：独立 composition-consumer 使用 CoreText 文字后端和实际 IconButton/Link/SegmentedControl View，验证调用者在主机事件间变更可用性；23 项通过。Gallery 状态捕获复用真实生产绘制路径。消费者及工作区严格 Clippy 通过。

运行时 token：最终 action 576、input 612、selection 2016 案例均重新采集，观测完整性仍为 false。Checkbox 保留 selection 的更广覆盖。观测不能证明编译常量、字体后端及未执行路径的完整依赖。

Gold7/GPU：三品牌、四主题、两倍率、两窗口/文字倍率与三个滚动位置，五组件已有 States 和 Button Variants 共 864 张。此批在可用性修复后、Link token 绑定修复前生成；随后用最终 Link 源码补拍 144 张，144 对 PNG 逐字节相同。共生成 1008 张，有效矩阵为 864 张，不重复计算案例。捕获时能力元数据是前态，后续元数据更新仅影响 Docs 摘要，未改变这些 States/Variants 绘制路线。已查看分段状态整页及最终 Link 深色 IC 大字页面；机械 Gold7/reference 门禁通过，不代表全部体验通过。

治理：34 项能力门禁、6 项组合审计门禁通过；45 组件 / 184 类型分类保留，当前组合审计已按实际改动重新绑定。纯 Rust、JSON DS 与设计门禁通过。没有运行性能测试。本批没有新增原生鼠标/键盘、系统 AX 或 VoiceOver 验收结论。

## 后续与合理例外

先核实 Gallery 整体禁用分段控件是否因逐项 set_enabled 改写选中值；如属共性缺口，补现有 View 的整体可用性契约。随后核实 IconButton 的默认命中尺寸、图标尺寸和语义圆角是否真正依赖已有 token，保留消费者显式 frame 的合理覆盖。不得据这五个组件的改善认定系统状态问题完成，不自动扩大组件迁移范围。

证据索引：design/reviews/action-family-20261018/receipt.json；当前源码快照与修改前备份、红绿日志、分阶段 GPU、最终 token 观测分别保存。
