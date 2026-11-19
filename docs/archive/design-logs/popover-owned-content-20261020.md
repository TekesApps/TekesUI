# Popover 公共组合最小消费者

## 当前 Main 修正验收（2026-10-29，基线4c9405b＋本批差量）

首次接收提交4f5b314的完整本地验收判断已撤回：原公开 `scenarios` 的非模态外部新焦点断言仍失败。失败收据和此前有限通过保持原身份。本批沿既有 focus/IME/epoch/reveal 准入，先验证实际关闭后的作用域及当前 Popover session，再原子关闭实际 Popover并安装新焦点；不允许 Tab、AX、pointer 穿过背景隔离或模态边界。原 Scroll body 的纯布局资格沿既有 Dialog 方式声明已安排的内容框，不重建布局规划器。

最终 main-cpu03 的 controls/consumer 严格检查、原 controls/IME/reveal 拒绝回归、完整原公开 scenarios 和 Popover/嵌套阅读场景均通过，原第822行断言保持。最终两份实际原生程序分别通过 Popover 的同 owner 编辑、组字/query、Escape、Undo、Tab、关闭/恢复，以及嵌套阅读的真实 Scroll、非零位移、Append、resize、原 Composer 状态保持。退出码不单独作为行为证据。实体 IME、VoiceOver、性能未执行；未作外部业务接入声明。

证据：SSD `artifacts/issue-49/maine435eb8-outside-focus-candidate-20261029-01/root-intake01/{main-cpu03,native-popover01,native-nested01}`。当前仍待本批提交、必要 CI、公开不可变 SHA 和远端 main 交付；#49 保持开放。主题、Scene、Render、生成器及 cfg 输入未变化，旧主题/GPU证据不改签、不无理由重跑。

## 历史：首次 Main 接收（2026-10-29，基线391f4ba＋差量）

当前库提供 `PopoverContent` 与原 ContentHost 的 show/close 公共入口，保留原文本编辑器、Switch、Cancel/Save 和真实下层动作。复用原 PopoverView、OverlayScope、motion、Palette/Surface 与原 mount；不增加第二个 host。配置 Dismiss 可隐藏且不再占据内容量度、绘制、命中或语义；默认旧 View 行为保持。共享 View 的 Surface 使用现有 canonical packet 与 popover elevation，未新增生成器或主题规则。

原组件/public consumer 行为、主题映射及必要 strict 已通过。接收中发现的缺失实际 View 依赖、重复 overlay paint、Dialog 未退休外部 popup、Cancel 后残留 scope 均已在不删断言的条件下修正。真实 AppKit 窗口已观测原 owner 的 UTF16 preedit/query、Escape 优先取消组字而保持表单、选择/编辑/Undo、Tab、外点关闭、同 mount 重开、原 Save AX selector、失活关闭与恢复不重开。实体键盘/物理 IME、VoiceOver、全库原生和性能没有执行，不作完成声明，也不是本票之外追加的门禁。

最终原生证据为 SSD `artifacts/issue-49/main6e8792c-popover-intake-candidate-20261029-01/root-intake01/native04/native-verified.json`，绑定 final-check07 的精确程序。native03 的通过保持其原身份；初始背景 token 与 AX 目标身份错误导致的 abort 保留在 native01/02。最后 consumer 入口将错误显式失败而非忽略，受影响 strict、公共行为/主题与最终原生程序已补验。独立公开40位 SHA／必要CI／远端 main 交付尚待，#49保持开放。

## 历史：consumer 包装器阶段（原证据身份保留）

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

### 2026-10-29：Main 6e8792c 初始私有候选（历史）

本候选把原 Root `PopoverContent` adapter 接入 Main 原 `ContentHost`，复用 PopoverView、OverlayScope、文本/AX owner 和现有 focus/capture 清理路径；它不创建第二个 host。公开 `ContentHost::pointer_capture()` 继续返回 `(ComponentId, mount)`；内部只记录该次 capture 来自普通正文还是 overlay，以保持同一原入口分发 Move/Up。

独立 consumer 保留 TextField 草稿、Switch、Cancel/Save 和真实下层 Action；Dismiss 可单独关闭，业务 completion 使用 opening `PopoverSession`，陈旧 session 不能关闭后续打开。关闭期间 shell 可退出绘制，原 body 不再进入 hit/focus/AX；anchor mount 失效与窗口失活会关闭 transient。Main consumer 原有手写 Card consumer 被公共 adapter 场景替换，测试源与历史证据身份保留；Root 收据不改签。

本节保留最初交付时的候选状态，已由上方当前接收状态替代。当时只完成静态 source/API、逐文件 rustfmt、Cargo 元数据解析、patch/path/hash 与 apply-check；不表示 Main 构建、GPU、AppKit、IME、VoiceOver 或完整 Issue #49 验收通过。原验收中 GPU、原生窗口、物理输入法与 VoiceOver 仍按各自源码和实体身份逐层验证；Root source receipt 对应不同 checkout，不能替代 Main 的验证。
