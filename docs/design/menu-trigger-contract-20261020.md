# MenuContent 触发器可用性及卸载契约

这是第六批菜单／浮层组合整改的有界修复；其余批次、Popover 接入及最终统一验收仍未完成。

根因属于公共组件适配契约。`MenuContent` 已使用共享 `ButtonContent`，但菜单的 AX 动作、子项语义及直接 overlay 事件没有完整遵守该触发器的 disabled/loading 状态；宿主仅取消输入并不能代替组件卸载时清理绘制缓存。

共享修复位于 `crates/tekes-ui-controls/src/composition/adapters.rs`：

- 复用 `trigger.focusable()` 判断可用性，复用 `trigger.semantics()` 发布 disabled/loading 元数据。
- 不可用时通过既有 `MenuView.deactivate()` 关闭并取消旧行按下，通过既有 Button cancel 清掉触发器的旧按下。事件与 AX 路径在下一次绘制之前也检查当前模型。
- 逻辑关闭后立即移除菜单子项和 expanded 状态；普通关闭保留退出绘制。
- 卸载调用已有 `clear_presentation()`，丢弃 popup 命中布局，重新挂载不重播旧退出。业务选择值保留。

适用于任意公共 Content 树中的所有 `MenuContent`；异构 List 是首批真实接入样例。合理例外：独立使用的 `MenuView` 继续由调用方使用其显式可用性 API 管理；本次没有覆盖该设置、给语义菜单加入任意 children，或增加状态、token、协议层。

分层证据：

- 公共 API：4 个测试先真实失败后通过，覆盖禁用／加载元数据和 AX、绘制前取消旧行按下、新意图恢复以及卸载清理。controls 总计 363 项通过，严格 Clippy 通过。最初测试编译错误单独保存，不算失败回归证据。
- 真实消费者：既有消费者 152 项通过；新增真实 CoreText 异构 List 回归跨 4 主题、3 品牌、两种宽度、disabled/loading，共 48 组。草稿、选择和无关行隔离均核对。
- GPU：48 个实际 GPU 场景、96 帧。错误参考由独立 MenuView 显式重建“不可用触发器下仍有 popup”，不是旧二进制截图。
- 原生：U2790B @2，19 个实际活动状态检查点，包含 AX 触发与选项选择、Enter 打开／提交、Escape 关闭、disabled/loading 隔离、真实移除重挂及新激活。保留原始事件、序列状态及截图；连续接受区间为 17–113。未将工具操作声称为物理按住输入或 VoiceOver 听读。
- Gold7：按既有菜单作用域、焦点、关闭和新意图规则检查实际渲染和交互；不推导全页面或全组件成熟度。
- Token：新增实际 MenuContent 生产绘制探针，240 个有界状态组合观察到 20 个 lookup ID；捕获只包围目标组件绘制，未把异构 List 的其他子项读数归给菜单。加上刷新后的既有 9 份探针，本轮 10 份共 4656 个有界样例，均 `complete=false`；不声称完整 alias／依赖闭包。
- 门禁：45 组件组合审计、8 项组合门禁测试、42 项能力治理测试和记录时的全库能力清单最终检查通过。其他并行 Scene3D WIP 保留，未纳入本轮验证。

新增探针入口为 `scripts/audit-menu-content-tokens.py --output <全新证据路径>`，使用既有共享缓存和构建锁运行；历史输出不覆盖。机器可读入口：`design/reviews/menu-trigger-contract-20261020/acceptance.json`。

小宿主已关闭，确认进程与文件未使用后清理本轮运行包、测试可执行文件和小宿主包产物；源码、证据、公共库及 registry 缓存保留。VoiceOver 保持关闭。未提交、推送、打 tag 或关闭 Issue。
