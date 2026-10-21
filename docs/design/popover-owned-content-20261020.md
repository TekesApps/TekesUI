# Popover 公共组合最小消费者

本轮完成第六批的有界 Popover 场景，不代表第六批、整个组件或八批工作已经完成。没有增加内置 `PopoverContent`，也没有扩大组件迁移。

根因是公共组合能力存在接入缺口：`PopoverView` 已有内容回调、放置、测量、事件和进入／退出过渡，但 `Content` 的子树绘制由宿主私有遍历接管；消费者无法把同一棵已挂载子树放入该回调，只能另做绘制或另建宿主。调用者的挂载身份校验也只在 Dialog 内部可用。

共享修复复用原有机制：

- `Component::paints_children()` 默认关闭；选择开启的消费者使用公开的既有 `Content::paint/paint_overlays`，宿主不再重复画它的 children。布局、生命周期、事件、焦点、文本查询和语义仍属于原来的 ContentHost。
- `ContentHost::capture_focus/restore_focus` 复用已有 `(ComponentId, mount)`，Dialog 也调用同一实现。它是进程内恢复意图，不是可持久化 key；替换、卸载重挂及当前不可用目标不能继承旧意图。
- `PopoverView::arrange` 复用生产 render 的同一几何计算，不绘制或推进过渡。关闭期间不再发布 Dismiss 逻辑焦点。
- 只绘制退出内容时仍检查实际文本后端；缺失时返回原有 MissingTextServices，而不悄悄缺字。

真实消费者位于 `examples/composition-consumer/src/popover_content.rs`。消费者定义的 Card 通过公共 Component 承载 Caption、真实 TextFieldContent 和 SwitchContent；没有第二个 ContentHost。打开／关闭使用已有 Popover/PopoverView，触发器使用现有 typed actions。

验证过程中修正了三项消费者适配问题：子树使用实际浮层祖先 clip，保留完整焦点环；Dismiss 消费空格按下和松开；普通 Cancel 只取消手势，显式窗口失活才关闭并清掉过渡及恢复意图。非模态关闭保留用户已经移到外部的新焦点。失败截图、确切旧源码、原生事件及失败回归分别归档，未计入最终通过。

适用范围是需要将已挂载 Content 放入绘制回调的消费者组件，Popover 是首批样例。默认自动绘制保持兼容。语义 Menu 的 item 限制仍保留；本轮没有把任意 children 塞进 Menu，没有另建状态机、协议层或 token，也没有实现 Window Toolbar 注册。

最终证据：

- 公共 API：367 项 controls 测试、4 个新增公共契约回归及严格 Clippy；Dialog 原有挂载身份回归继续通过。
- 真实消费者：154 项测试及严格 Clippy；24 组真实 CoreText 场景覆盖子控件输入、草稿／选择保留、组字 Escape 归属、外部新焦点、调用者替换、卸载重挂和显式失活。普通 Dismiss 的退出过渡另有失败再通过的回归。
- GPU：48 个场景、192 帧，4 主题、3 品牌，360@1 与 720@2，字号倍率 1 与 1.5。错误 clip 对照是显式重建，不冒充旧 GPU 二进制；旧版原生截图及确切源码保留。
- 原生：外接 U2790B @2，28 个最终签名二进制的实际事件检查点。普通空格关闭在序列 27 保留退出，Reduced Motion 的序列 103 立即结束，真实最小化／失活／激活为 108／110／113。AX Dismiss 为 119。正文编辑后跨关闭、重挂载和窗口恢复保持。完整焦点环和前后截图分别保存。
- Gold7：本轮核对焦点、非模态事件归属、正常与 Reduced Motion 关闭、新意图和保留状态；最终整页和全组件体验仍需统一验收。
- Token：11 份实际探针、5040 个有界样例，均 complete=false。Popover shell 的 192 个样例观察到 18 个 lookup ID；另 192 个 owned-tree 样例含子组件，观察到 25 个，未归给 shell。常量、完整 alias／依赖闭包及未执行路径不在证明范围。
- 门禁：45 组件组合审计、8 项组合门禁测试及 42 项能力治理测试通过；全库最终能力清单检查结果单独记录。

原生证据来自工具驱动的实际 AppKit 键盘、菜单、AX 和窗口生命周期。未声称物理拼音或 VoiceOver 听读通过，VoiceOver 全程保持关闭。Reduced Motion 是宿主提供的上下文，未修改系统设置。性能没有测量。

机器入口为 `design/reviews/popover-owned-content-20261020/acceptance.json`。小宿主关闭、进程退出并检查无使用后，已清理运行包、测试可执行文件及宿主包构建；代码、失败／通过证据、截图和公共库缓存保留。`source-final` 只是本轮所属文件快照，不是完整可移植依赖闭包。未提交、推送、打 tag 或关闭 Issue。
