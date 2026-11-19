# SegmentedControl 整体可用性

基于全部 WIP，沿原 #8/#3 处理；不关闭 Issue。

根因是公共组合控件缺少整体可用性入口，Gallery 用逐项 set_enabled 代替，触发单项禁用的合法选中回退，还在重新启用时清掉调用者的单项禁用。真实 Gallery 回归先失败：selected 从0变为2。

共享修复位于 layout_view.rs 的 SegmentedControlView：复用现有 cancel/blur，增加 set_disabled(bool)。整体禁用取消指针/Space、清焦点，并阻断用户选择，不改 model.selected 或 per-item enabled。重新启用不复活旧按下、不自动恢复焦点。Gallery Playground 和已有全部禁用状态样例统一使用它。

适用于全部 SegmentedControlView 消费者。单项 set_enabled 继续使用原有回退策略，是有意保留的模型契约；消费者显式改值不等同于用户输入。没有新增 token、协议层或组件，也没有扩展其他组件迁移。

公共 API：包含无绘制间隔的禁用、禁用期间绘制、重新启用、旧指针/Space release 和新点击；所有5项 action 契约通过。共享 controls268项及 tokens/design-export42项，共310项通过。

真实接入：Gallery 实际 Workbench 构建回归先红后绿，保留选中值与单项可用性；CoreText 独立消费者24项通过，包含整体禁用/重启用场景。工作区、独立消费者、token-audit 探针严格 Clippy 通过。

Gold7/GPU：已有 SegmentedControl States 整页重新捕获，三品牌、四主题、@1x/@2x、两窗口/文字倍率、三个滚动位置，共144张；与前批对应144张逐字节一致。查看普通浅色整页可见全部禁用、单项禁用、选中、焦点、hover/pressed。此证据验证绘制保持和真实矩阵接入，不证明原生输入或 VoiceOver。Gold7 机械门禁通过。

三个运行时 token 探针基于最新源码重新执行：action576、input612、selection2016；覆盖仍有界且 complete=false。45项当前组合审计重新逐项保留原内容容量结论并绑定新增方法后的源码位置；34项能力门禁与6项组合审计测试通过。

本批未操作原生窗口/系统设置、未测性能、未提交推送或关闭 Issue。后续先核实 IconButton 既有 token 的默认几何依赖；全量184项夹具及 #12 原生/可访问性验收仍开放。

证据：design/reviews/segment-availability-20261018/receipt.json；历史 action-family 批次保持原样。
