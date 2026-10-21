> 此入口的表格/行号属于2026-10-20审计时点；后续当前状态、已补证项及精确替代关系见 [2026-10-21入口](composition-audit-current-20261021.md)。旧收据保留原身份。

# #14 当前组合审计入口（2026-10-20）

工作树 HEAD `b8999fe006adfcd70587f07af50ef9e240422788` 加全部 WIP。HEAD 仅为提交来源，当前字节身份以 [机器矩阵](../../design/remediation/composition-audit.json) 的完整源码 SHA-256 和精确声明行为准；[此前清单](../../design/reviews/composition-audit-followup-20261020/manifest.json) 固定其审计时点身份，后续集合修复另留证据。本轮复核现有矩阵的新鲜度、#22 契约变化及兼容边界，不冒充重新完成全库原生或视觉验收。

45 项登记组件：公共组合协议 **10 已实现、6 部分实现、29 缺失**。内容能力另计，叶子无须 children；这些统计既不是控件整体可用性，也不是原生体验或成熟度。全部 source-audit native_acceptance 仍为未验证。

最新 #14 继续限定为审计／任务规划。Window Toolbar 注册、条目组、placement、overflow、命令同步与多窗口生命周期是后续能力，不能以 ToolbarView 的内容行或当前缺口为由直接扩大实施。

## 当前矩阵投影

以下投影来自本轮机器矩阵；完整 state ownership、生命周期、AX、兼容性及所有源证据见上面的 JSON。历史数值／行号不再作为当前结论。

| 组件 | 维度 | 协议 | 内容能力 | 槽位 | 首条源证据 |
|---|---|---|---|---|---|
| Stack | slots | 已实现 | 已实现 | children* | `crates/tekes-ui-controls/src/extended.rs:20` |
| Grid | slots | 已实现 | 已实现 | cells* | `crates/tekes-ui-controls/src/extended.rs:61` |
| Separator | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/extended.rs:103` |
| SplitView | slots | 缺失 | 部分实现 | 无 | `crates/tekes-ui-controls/src/extended.rs:113` |
| ScrollArea | slots | 已实现 | 已实现 | children* | `crates/tekes-ui-controls/src/scroll_view.rs:27` |
| Disclosure | slots | 缺失 | 部分实现 | 无 | `crates/tekes-ui-controls/src/extended.rs:508` |
| Surface / Panel | slots | 缺失 | 部分实现 | 无 | `crates/tekes-ui-controls/src/feedback_view.rs:176` |
| Toolbar | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/widgets.rs:2585` |
| Text | leaf | 部分实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/text_surface.rs:81` |
| Icon | leaf | 部分实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/layout_view.rs:117` |
| Button | leaf-and-slots | 已实现 | 已实现 | content1 | `crates/tekes-ui-controls/src/widgets.rs:298` |
| IconButton | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/extended.rs:530` |
| Link | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/extended.rs:559` |
| TextField | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/text_surface.rs:81` |
| TextArea | leaf | 部分实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/text_surface.rs:81` |
| SearchField | leaf | 部分实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/text_surface.rs:81` |
| ComboBox | leaf-and-slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/extended.rs:1246` |
| Slider | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/selection.rs:34` |
| Checkbox | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/widgets.rs:803` |
| RadioGroup | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/selection.rs:288` |
| Switch | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/selection.rs:638` |
| SegmentedControl | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/extended.rs:808` |
| Field | slots | 部分实现 | 部分实现 | 无 | `crates/tekes-ui-controls/src/widgets.rs:1233` |
| Select | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/widgets.rs:1168` |
| NumberField | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/widgets.rs:1189` |
| Tabs | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/widgets.rs:1068` |
| Breadcrumb | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/extended.rs:873` |
| CommandPalette | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/extended.rs:1312` |
| List | slots | 已实现 | 已实现 | rows* | `crates/tekes-ui-controls/src/collections_view.rs:230` |
| Tree | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/collections_view.rs:527` |
| DataTable | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/collections_view.rs:981` |
| Chart | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/chart.rs:33` |
| Menu | leaf-and-slots | 已实现 | 部分实现 | 无 | `crates/tekes-ui-controls/src/widgets.rs:1098` |
| Tooltip | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/extended.rs:1118` |
| Popover | slots | 部分实现 | 部分实现 | one content callback; real minimum consumer owns one Content subtree | `crates/tekes-ui-controls/src/extended.rs:1123` |
| Dialog | slots | 已实现 | 已实现 | header1, body1, footer1 | `crates/tekes-ui-controls/src/widgets.rs:1142` |
| Toast | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/extended.rs:1145` |
| Banner | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/extended.rs:1191` |
| StatusBadge | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/extended.rs:1207` |
| Progress | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/extended.rs:1214` |
| EmptyState | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/extended.rs:1239` |
| Skeleton | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/feedback_view.rs:2214` |
| Markdown | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/collections_view.rs:1718` |
| CodeBlock | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/collections_view.rs:1828` |
| DiffView | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/collections_view.rs:1934` |

## 已落实的公共能力与合理边界

- Component/Content/ContentHost 的既有核心承担测量、布局、绘制、裁剪、路由、取消、焦点与动态树生命周期。已接入的适配器复用它，未接入的组件不因拥有 model/View 就升级为通用 Content。
- 复合 Button、异构 List 与 Rich Dialog 有独立消费者和分层历史证据；后续共享修复仍须按其精确候选复验，不能把三者的局部成功推广成全库通过。
- #22 新增的是 ScrollContent 的可选 item/document 阅读事务，复用 ReadingDocumentVersion/ScrollArea；不是另一个滚动器，也不是已交付 #16 的单文本事务。固定宽度／字体前插有0pt与像素连续性证据，真实混合 Content 和行内编辑器测试通过。宽度／字号变化只保证稳定 item-local 点与阅读意图，不声称任意组件拥有字／UTF-16 锚点。
- 新本地导出 `bd3ed1b0af81b044306af90d3594bb31110147b7194bda6053b1d2e185871c9a` 已由发送方独立解包复验；外接屏活动原生结构更新／重排和草稿保留已通过并清理，见 `native-active.json`。物理动量、IME、适用 AX／VoiceOver、ClientKit 匹配视觉及接收方结论必须分开。
- 旧标签／模板 API 继续可用；消费者自定义内容不能因通用组合丢失各自交互语义。虚拟化重建／编辑存储策略与窗口级 Toolbar 未实现，不隐去缺口。

## 兼容与迁移

后续集合输入修复见 [collection-live-input-20261020](collection-live-input-20261020.md)：更新了三种既有 View 的当前模型／Escape 契约及 Gallery Tree 转发。机器矩阵已随其源码重新绑定，之前 followup 清单保留其历史精确身份；协议／内容槽统计没有变化，不将输入修复冒认为新增 Tree／DataTable Content 适配器。

#26 的共享受控值只读预检见 [controlled-preflight-20261020](controlled-preflight-20261020.md)：TextInput／TextSurface 复用同一校验，实际 apply 重新验证；独立消费者先预检完整应用输入，再校验 reading token 并同步提交，不承诺回滚任意应用回调。该能力不改变45项组合协议统计或原生验收状态。

集合可用性／完整焦点环的当前修复见 [collection-focus-20261021](collection-focus-20261021.md)：原View状态与绘制能力复用，行列几何及公开签名不变，实际ancestor仍有效；这不是新增Tree/DataTable Content适配器或全组件原生验收。

新增 `CompositionError::ReadingTransactionInProgress` 会使下游穷举匹配需要迁移，是潜在的源码破坏性枚举扩展；不能笼统称为“纯添加、无影响”。处理新增分支即可，callback 中 render 被拒绝不等于所有应用变更会回滚。

`Content::preserving_focus()` 是实例配置，未给 ButtonContent 增加必填字段；Tab／显式 AXFocus 可用。现有 reading_prepend、reading_stream 与结构化消费者的辅助 latest 动作统一接入。普通滚动器默认不启用 reading；异步结果必须保留请求时的 expected token。

已有 required 语义字段涉及直接 struct literal 的迁移，仍保留在机器矩阵的 compatibility_updates_20261020。当前发布的8cf消费者验收不能认证之后的全部 WIP。

## 执行依赖与通过证据

| 顺序 | 任务与负责人 | 通过依据／尚缺 |
|---|---|---|
| 1 | root：当前共享核心与最小消费者 | API／实际接入／GPU 按精确身份验；#22 bd3 已完成限定活动原生功能；#26 预检增量另包复验，最终组合版本与物理输入未完成 |
| 2 | root 与授权 AgentKit 会话：独立消费者 | 接收方按新身份独立构建／真实 Transcript 接入并留结论；不覆盖旧冻结包 |
| 3 | root：适用组件族与语义迁移 | 沿用批次账本，单一共享负责人；只迁移已明确范围，未内建的 Tree/DataTable 等能力如实保留 |
| 4 | root：Gold7 与整页验收 | 同内容、主题、宽度、backing/type scale 的前后／参考对照，原生键鼠与适用辅助技术另证；当前视觉不通过 |
| 后续规划 | Window Toolbar | 统一组合与语义基础之后的稳定item、组、placement、overflow和多窗口设计；本审计不实现、不创建重复Issue |

审计交付仍缺与该 WIP 对应的公开可获取版本、同版本远端门禁和独立最终审阅，以及向原 Issue 回填可公开复核的源链接。准备材料不等于授权提交／推送／关闭 Issue。性能依用户要求延期。

当前审计与实施入口：[八批账本](../../design/remediation/issue-component-batches-20261019.json)、[#22 证据](../../design/reviews/content-reading-anchor-20261020/progress.json)、[原生／视觉边界](../../design/reviews/content-reading-anchor-20261020/visual-reference-source-review.json)。未修改冻结包、Kernel、ClientKit 或 Tekes。

DataTable 最新模型与已挂载导航候选的交集修复见 `docs/design/collection-navigation-live-20261021.md`，仍复用现有 View 与导航函数。公共组合协议维度、原生状态与成熟度均不因此提升。

Separator 已复用现有装饰线语义补齐 backing-scale 输入，当前普通／变体展示统一转发倍率。新增 API 兼容旧1x入口；不等于窗口Toolbar协议、内容组合或原生成熟度提升。见 `docs/design/separator-scale-20261021.md`。

GridView 零面积可用性修复与几何零lookup范围见 `docs/design/grid-layout-availability-20261021.md`。纯Grid模型测量、正常自然溢出与组合协议维度不变，不将子控件token依赖归入Grid。

共享Surface倍率能力已在全部当前运行／示例的适用装饰Panel调用中接通；没有新增API或统一加浅颜色。当前Stack几何与Surface绘制分开观测，子项依赖不归入布局组件。见 `docs/design/panel-scale-adaptation-20261021.md`。
