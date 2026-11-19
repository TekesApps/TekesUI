# 混合 Content 阅读事务（#22）

本轮补的是多 Content 的阅读位置契约；已交付的单 TextSurface `ReadingController` 不重复实现。根因是普通 `ScrollContent` 只有像素 offset，没有文档附件、稳定子项与阅读意图，前插或上方高度变化会使相同内容移出原位置。消费者自行估算行高补偿会形成第二套滚动实现。

共享实现位于 `controls/src/composition/content_reading.rs`，沿用 `ScrollArea`、`ContentHost`、稳定 `ComponentId` 和现有 `ReadingDocumentVersion`。普通 ScrollContent 默认不启用；消费者只提交文档 key、版本、稳定 item 顺序和实际树变更。布局内部测量并恢复，在最终绘制前完成，不增加中间帧。

## 公共接入流程

完整可运行的公共消费者是 `examples/composition-consumer/src/scroll_updates.rs` 的 `Scenario::reading_document()`，回归在 `tests/scenarios.rs`，真实 CoreText/GPU 接入在本轮 `host/tests/reading_gpu.rs`。它混合 Text、嵌套 Stack 面板、Button、Switch 和行内 TextField，另有独立 composer。

1. 构造实际 Content 树。稳定 item key 使用既有 ComponentId；item 列表按文档顺序给出，可包含尚未挂载项，不得重复。
2. 使用 `host.set_scroll_reading_document(owner, key, revision, items, callback)` 附加文档。callback 用既有 insert/remove/reorder/get_mut 更新树。返回的 ReadingDocumentVersion 是本次附件的请求身份。
3. 在发出异步请求时保存这个 token。结果到达时使用 `host.update_scroll_reading_document(owner, &expected, next_revision, new_items, callback)`。错误身份、旧版本及非法输入在 callback 执行前拒绝；不要把旧请求 token 换成到达时的当前 token。
4. 仍用 `host.render(scene, text_backend, frame, clip, palette)` 测量和绘制。此后从 `ScrollContent::reading_result()` 取得该版本的 Preserved/Fallback/FollowingLatest、clamped 和 user_input_won。不可见 viewport 暂缓恢复，不伪报成功。
5. 原生输入照常送入 `host.event(&ContentEvent::Scroll { .. })`，不增加消费者 scroll_by 补偿。数据更新不取消同一文档的有效输入；输入发生后按新的阅读位置恢复。跨文档、卸载、显式取消及显式回到最新会隔离旧动量尾包。
6. 回到最新使用现有 ButtonContent 和 `ContentActionKind::Activate`，调用 `ScrollContent::follow_latest()`。数据到达、布局变化和接近底部都不会自行恢复跟随。必要时显式 `stop_following()`。

“回到最新”是作用于阅读位置的辅助动作，鼠标或 AXPress 不应抢 composer 的键盘／组字所有权。本轮新增实例配置 `Content::preserving_focus()`，仍使用原按钮契约和绘制。Tab、显式 AXFocus 保持可用。已统一应用到结构化消费者、reading_prepend、reading_stream 的同类动作；普通按钮默认行为不变，不新增包装组件或 token。

## 身份、保留与合理边界

- 同 item 的局部逻辑点为结构化锚点。固定字体／宽度的前插误差要求 ≤1pt；任意自定义组件没有通用词／UTF-16 接口，因此不声称字号改变时仍指向同一个字。单 TextSurface 的语义锚点继续用 ReadingController。
- A→B→A 每次取得新附件 token，每文档分别保留阅读／跟随意图。旧附件、旧 revision 与卸载前 token 均不能修改当前树；同文档 revision 不允许倒退。
- item 删除或变为零尺寸时，依次选择旧顺序中存活后继、前驱、当前首个已布局 item；无可用项则 Fallback(None)。内容边界无法满足原位置时明确报告 clamped。
- 消费者应保留存活 Content 对象及身份；不要为更新整树重建编辑器。本轮真实行内 TextField 重排测试核验焦点、选区和 preedit 不变。
- callback 是同步更新边界，不是任意应用失败的回滚事务。不要在 callback 中替换其 reading owner；callback 内 render 拒绝为 `CompositionError::ReadingTransactionInProgress`。panic 后清理事务标记并继续传播，已执行的应用修改不自动撤销。
- CompositionError 新增枚举项；下游穷举匹配需处理它。未给公开 ButtonContent 增加字段，未引入新 trait／包装层。

## 分层验证与剩余项

### #48 跨进程重排修正（2026-10-28）

前插后仍可能露出前一项的一像素边缘。没有新用户滚动时，自动重排继续保持原稳定 item，不以该边缘替换锚点；推进文档版本前保存现有身份。新的用户输入或实际滚动偏移变化重新选择当前可见项，显式 latest、删除回退和 clamp 规则保持不变。复用既有 result/pending/capture 状态，没有消费者 offset 补偿或第二套阅读引擎。

公共原组件红例从错误锚点900恢复到原项100；独立三 IPC consumer 的 `--reading-anchor-only` 保留原混合 Reader，通过原业务消息/API 前插、改变宽度与字体、恢复及显式 latest。原 CoreText Code/mono 字体下，页高实际为1314→2178→3915，原项始终 y=5，误差0≤1逻辑点，组字和 item mount 保持。此处是结构化 item 局部锚点，不声称字号变化后仍指向同一个字。实际命令、旧864点失败、红例及必要原生回归分别保留在 SSD `artifacts/issue-48/main33d35b3-reading-anchor-20261028-01/`。同源 CI 增加该原 consumer 模式；完整 #48 原 AC 仍按其 Issue 收口。

本轮 `design/reviews/content-reading-anchor-20261020/` 保留公开 API 的失败／通过回归、真实公共消费者、GPU、源码输入及 token 记录。命令与结果以该目录 receipt 为准。GPU 图片只证明本实现更新前后的连续性，不能证明 ClientKit 视觉一致。

原生物理动量、原生 IME／AX 在最终候选中的组合复验，以及相同场景／主题／宽度／倍率下的 ClientKit 对照仍未完成。当前压力布局不作为视觉达标依据。不得把离屏像素相等或已有旧版本的原生验收升级为这些项目通过，也不得据此关闭 #22。性能按用户要求延期。冻结 AgentKit 输入保持不动，待独立交付候选明确后再消费。
