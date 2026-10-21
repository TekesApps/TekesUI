# Gallery 反馈组件真实动作与焦点接入

本轮基于当前全部 WIP，保留 AgentKit 已验收的 4310937 冻结输入。修复尚未提交或推送。

根因属于真实消费者接入：Popover、Toast、Banner、EmptyState 的公共 View 已有操作按钮、裁剪几何、焦点和按键能力，但 Gallery 没有将这些组件自身的按钮发布到原生可访问性树。Popover 的宿主 Tab 分支还会先关闭浮层，导致无法进入 Dismiss。公开契约无需新增抽象或 token。

共享修复位置是 Gallery 的 feedback_adapter、workbench 焦点路由和 accessibility 生产者。使用实际渲染文案和公共 View 布局；正文 Group 与按钮父子关系一致。AX Press 复用既有组件按键 down/up，关闭后拒绝迟到动作。Popover Tab/Shift+Tab 在既有焦点顺序内移动，Dismiss 的 Enter 完整按下/释放关闭后恢复仍挂载的调用按钮。

适用组件为 Popover、Toast、Banner、EmptyState 的真实组件拥有动作。Tooltip 不含对应操作按钮，Progress/StatusBadge 无可激活动作，保留各自语义；没有把无操作的内容伪装为按钮。Toast 的现有计时器示例按钮保留宿主语义。

验证证据在 design/reviews/issues1-14-closeout-20261019：feedback-ax-red-runtime.log 记录四个实际缺失；feedback-ax-green.log 和 feedback-family-final.log 验证真实发布、执行、关闭后撤销；popover-tab-final.log 验证 Tab、Shift+Tab、真实非编辑控件 Enter down/up 和焦点恢复。最初将 Enter 模拟为编辑器 TextCommand 的失败记录保留，修正为真实控件输入路径后通过；未添加不必要的编辑器命令适配。

公共 API 签名未改变，复用已有 controls。首轮真实原生检查完成后，用户指出 Banner/Toast 图标与文字未对齐，首轮 Gold7 已明确记为失败，未用此前截图提升接受。

共享根因是两个通知 View 将状态标记放在固定 Y，而文字由真实 shaping 决定首行高度。现在复用 ButtonTextRenderer 已有 first_line_height，让两类标记按真实首行居中；多行仍对齐第一行，保持原有正文、动作和状态语义。不新增 token、状态或公共协议；独立 StatusBadge 不拥有文字，不套用这个契约。

后台原生启动还复现 Banner 的失活绘制 panic：绘制错误地依赖仅在活动窗口存在的逻辑焦点目标。现在使用已有绘制几何，保留失活时的被动呈现，继续撤销焦点目标并拒绝操作。单元红/绿和新原生后台启动已留证。

最终证据见 ../../design/reviews/notice-alignment-20261019/acceptance.json：controls 320 项（237 单元加 83 集成/文档），Gallery 416 通过、1 项既有忽略，独立消费者 133 通过；严格 Clippy/fmt 通过。独立 CoreText 消费者覆盖 96 个通知条件；64 帧真实 GPU 每帧包含两个通知，覆盖四主题、@1x/@2x、100%/150%、220/640pt 与单行/混排换行。

最终 U2790B @2x 原生复验包含四主题通知、420pt/150% 预览、两类 Dismiss、EmptyState 创建及节点更新、Popover Tab/Shift+Tab、Enter/AX/Escape/outside 关闭与焦点恢复、Reduced Motion 终态及窗口关闭后 0 节点。当前仅提升该反馈修复范围的 Gold7；未开启 VoiceOver，不声称全库或完整辅助技术验收。

旧失败 App 和更早 Menu/Popover App 已按精确归属/哈希和占用检查清理，源码与原始证据保留。最终 App 在用户请求后重新打开供查看，暂不清理正在使用的文件；查看结束再回收。不测性能、不提交推送或关闭关联 Issue。


## 用户复核发现的按钮默认对齐遗漏

用户随后指出 Banner 的 Dismiss 仍未居中。此前将反馈修复 Gold7 记为通过的结论已撤回，原记录备份保留在 feedback-button-centering-20261019。根因是组件调用显式传 Leading，绕过了已有 ButtonTextAlign::default() 与 JSON button_content.text_alignment=center。

本轮不新增样式层：ToolbarView 既有动作按钮、Toast/Banner Dismiss、EmptyState action 全部改用已有默认值；Popover Dismiss 也使用同一默认入口。NumberField 步进按钮原已读取 BUTTON_CONTENT，无需重做。消息、说明文字和导航/菜单语义不套用按钮居中。

增加覆盖五种真实 View 的公共回归，修复前明确报告五个 Leading 覆盖；独立 CoreText 消费者同时检查默认对齐。真实 GPU 回归读取实际栅格字形 ink bounds，修复前复现 Toast 向左偏移，修复后四主题、@1x/@2x、窄宽、100%/150% 共 64 帧通过，保留首行图标对齐检查。证据目录为 ../../design/reviews/feedback-button-centering-20261019/。

修复版 Gallery Feedback Centered 已在外接屏打开并保存实际 Banner 原生截图，供用户继续查看。本轮不重新宣称第一批整体接受：后续原生场景和本次源码改动后的门禁登记仍须收完，再继续第二批。当前小宿主保持打开，不删除正在使用的产物。


## EmptyState 自然尺寸补齐

随后原生截图复核发现，窄预览／150% 字号时 Create document 换行后按钮底部被裁剪；此前的按钮居中修复仍有效，但不能因此提升整批视觉接受。新增回归在修复前实际复现：按钮完整高度 56pt，宿主只留下 50pt。

根因同时属于公共组件测量契约缺口与 Gallery 接入：EmptyStateView 已按真实文本测量绘制，却没有公开自然尺寸入口；Gallery Playground、矩阵和变体重复手算高度，并使用与 SurfaceView 不一致的留白。现在新增 EmptyStateView::measure，和 render 共用现有标题／说明／Button 测量规则。Gallery 三处均按现有 space-loose 的 Surface 留白组合自然尺寸；独立组合消费者的空状态也使用公共测量。没有新增 token、通用容器或其他抽象。

适用范围是现有 EmptyState 的自然内容宿主。显式受限的调用者仍可裁剪；原有紧框测试保留，动作不得为挤入框而覆盖正文。标题和说明继续左对齐，拥有的动作继续复用 Button 默认居中。其他反馈组件不套用 EmptyState 的布局规则。

本次独立证据入口为 ../../design/reviews/empty-state-measure-20261019/。公共 API、真实消费者、真实 GPU 与最终原生／Gold7 分别登记，未完成的原生项不从离屏结果推导。旧失败图和全部历史证据保留。


本轮最终结果：controls 321、独立消费者 134、Gallery 417 通过／1 既有忽略，严格 Clippy 与原生构建通过；真实 GPU 24 个 EmptyState 条件及 64 帧通知回归通过。外接 U2790B @2x 的四主题、420pt／150% EmptyState、AX／键盘创建、矩阵／变体及通知对齐、Dismiss 和 Popover 焦点／关闭回归完成。原生几何显示动作完整高度 56pt，修复前仅 50pt 的裁剪已消除。首次 AX 打开后直接 Tab 未建立键盘焦点的探索记录保留并排除；最终键盘回归使用实际 Tab 到调用者后打开的路径。

本轮按限定范围登记 Gold7 通过，不提升全库成熟度。VoiceOver 未开启；关闭后接受节点为 0，原生宿主清理约 32MB，Gallery 自身可重建产物清理 135.9MiB，公共依赖缓存、代码与全部原始证据保留。七项实际 token 探针已再次按当前共享源码运行；能力清单与组合矩阵已复核刷新。第一批结束，进入 #13/#8 账本与 #6/#14 审计门禁交付；未提交、推送或关闭 Issue。
