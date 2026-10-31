> 2026-10-30 #198 恢复候选：下文原设计及历史接收身份保留。当前 4815c5a 兼容适配尚未编译；历史 pass 不等于当前主线/候选通过。公共交付由唯一集成批次完成。

> 2026-10-24 主协调 v3：第10节定义 #85 事务兼容能力、失败输入所有权与原 allocator 全子树预留。该设计已裁定，候选实现未接收；上文 v2/未启动描述保留为历史，不能替代当前 Issue 状态。

# #67 DataTable 公共 Content 与稳定行列身份：第一阶段设计候选

日期：2026-10-24。版本：v2，按主协调已咨询 advisor 后选择的兼容方案修订。原 v1 原样保存于 review/design-v1.md。状态：设计契约候选，尚未实施，不勾选或关闭 [#67](https://github.com/TekesApps/TekesUI/issues/67)。本轮只写本文与 `design/reviews/datatable-public-content-20261024/`；不创建执行票、不修改生产源码、Theme、矩阵或其它 QA，不新增 Host/输入引擎，不与 #73 Tree writer 双写。下述 API 均为候选，当前不可导入。

## 1. 原范围与源码事实

已读 `/Users/frankqin/.codex/AGENTS.md`、#67 原文及全部 comments（读取时为空）。正确源是 `/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI`，HEAD `1a7614a273e1a897ddc8a7ad8cb41d303a4a1290` 加既有 WIP，不能当作公开 main。15 个输入文件逐文件冻结到专属 `source-snapshot/`，身份见 `source-identity.json`，该文件 SHA256 `e4ab895bb64e0836dc42d093c697e690199697b0ae38779f38a164be455d3715`。这是设计输入片段，不是完整编译或 Theme 闭包；实现前须重新确认接收基线。

| 原位置 | 已有能力与限制 |
| --- | --- |
| `controls/src/extended.rs:1044` | DataTable 全部字段公开：selected_row、sort、rows、columns；业务身份只有 usize 位置。select_row、sort_by、key；单选，sort_by 切换同列 Ascending/Descending，没有实际比较器或数据重排。 |
| `controls/src/collections_view.rs:995` | DataTableView 是原 focus/hover/capture/Quiet chrome owner；DataTableLayout 发布 header/row/column frame 与 content_height。row_order 是消费者提供的原模型 index；过滤超界但未拒绝重复项。文本测量可提高行高/表头高度；34pt 最小高度、header 后 6pt、row_gap、cell 左右各 8pt。 |
| 原 View 输入 | header pointer_down 立即切换 sort 并 focus_header；行 down 选择并 capture，up 只对相同行确认选择。Enter 在已选行返回 Activated，否则选择；上下/Home/End 按当前挂载显示顺序移动选择/焦点，header 左右 rove、Enter sort。Escape/cancel 取消 capture。无独立 cell 焦点。 |
| 原 View 列宽 | 可用宽度均分；没有 resize handle、列拖拽、固定列宽模型或 resize action。#67 所述“列宽”不等于已有 columnresize。首批保留均分和测量，不增加 resize 业务。 |
| Gallery collections_adapter:262 | fixture 按字符串比较计算 row_order；这只是 Gallery 消费者业务排序，不能搬进公共 adapter 作为任意数据的排序器。 |
| Gallery workbench/accessibility:317 | 原生 Table/Column/Header/Row/只读 Cell 已有实例投影；row ordinal 是显示位置，cell 行列范围同理。原算术 ID 依赖 source index/列数，不能复用于新公共身份。 |
| composition / wire / app-host | 当前没有 DataTableContent、Table ContentRole 或 wire SemanticRole。平台 macOS 已有 Table、TableColumn{index}、TableColumnHeader{index,sort}、TableRow{index}、TableCell{row,column}。公共语义需补齐映射，不能用 Group/List 声称表格可用。 |

只读借鉴已接收 Tree 设计的身份、安全队列和事务约束；DataTable 是二维平表，不照抄 parent/preorder/expanded/最近祖先恢复，也不引入 StableCollection 框架。

## 2. 首批支持边界与身份

首批交付有限内存、单行选择的只读表格；稳定 row/column key、被动 header/cell/accessory Content、动态完整快照、原输入/取消、Table 本地语义及最小双向 wire 接收。排除多选、cell editor、Tab 网格编辑、公式、拖放、列重排手势/resize、冻结列、自动业务排序、分页、远端数据源和大规模虚拟化。首批无编辑 owner；唯一交互 owner 仍是原 DataTable/DataTableView。

候选身份与数据接口：

```rust
pub struct TableDatasetKey(pub u64);
pub struct TableRowKey(pub u64);
pub struct TableColumnKey(pub u64);
pub struct KeyedTableColumn { pub key: TableColumnKey, pub name: String }
pub struct KeyedTableRow {
    pub key: TableRowKey,
    pub name: String,                // 显式行 AX 名称，不从装饰树拼凑
    pub cells: Vec<String>,           // 按本次 columns 存储顺序；默认文本/AX 值
}
pub struct KeyedTableSnapshot {
    pub dataset: TableDatasetKey,
    pub columns: Vec<KeyedTableColumn>,
    pub rows: Vec<KeyedTableRow>,
    pub display_rows: Vec<TableRowKey>, // 唯一有序子集；省略即暂不展示
}
/// 模型层包装，字段私有；拥有唯一原 DataTable 与 identity metadata。
/// 不实现 DerefMut，不返回 &mut DataTable，不暴露可写字段。
pub struct KeyedDataTable { /* original: DataTable, metadata: ... */ }
impl KeyedDataTable {
    pub fn new(snapshot: KeyedTableSnapshot, budget: TableBudget)
        -> Result<Self, TableUpdateError>;
    pub fn original(&self) -> &DataTable; // 只读，不赋予 legacy View keyed 执行权
    pub fn selected_key(&self) -> Option<TableRowKey>;
    pub fn sort_key_state(&self) -> Option<(TableColumnKey, SortDirection)>;
    pub fn replace_keyed(&mut self, snapshot: KeyedTableSnapshot)
        -> Result<TableReconcile, TableUpdateError>;
    pub fn select_key(&mut self, key: TableRowKey) -> bool;
    pub fn sort_key(&mut self, key: TableColumnKey) -> bool;
    pub fn row_key(&self, index: usize) -> Option<TableRowKey>;
    pub fn row_index(&self, key: TableRowKey) -> Option<usize>;
    pub fn column_key(&self, index: usize) -> Option<TableColumnKey>;
    pub fn column_index(&self, key: TableColumnKey) -> Option<usize>;
}
```

key 是调用方提供的业务实体 ID，不是 label、当前 index、显示 ordinal、ContentId 或其 hash。row key 和 column key 属不同命名空间；cell 身份是 `(dataset,row key,column key)`，不是 `row*column_count+column`。模型 index 仅是本次存储地址；display_rows 的位置是当前视觉/AX 行 ordinal；column 当前位置是视觉/AX 列 ordinal。sort_key_state 由原 sort 的 column index 经当前映射导出业务 key + direction。

主协调已选择**模型层 KeyedDataTable 拥有原 DataTable + identity metadata**：原 DataTable 四个 public 字段、结构体 literal/解构、new 与 legacy 行为全部保留，不向其增加字段或私有状态。包装不是 Content adapter sidecar；它在模型层拥有唯一原 DataTable，adapter 只持有这个受控模型和原 DataTableView。对外没有 `&mut DataTable`、`DerefMut`、可写字段或能从闭包取出原可变模型的出口；不采用直接改 keyed 投影后再恢复的方案。original() 只读可用于诊断/兼容读取，不是输入授权。

selected/sort 的唯一可变权威是包装内原 DataTable 状态。metadata 保存 row/column key↔index、dataset、instance identity、revision/incarnation 及快照文本；selected_key/sort_key_state 由原 selected_row/sort 经当前映射只读导出，不另存可独立写的第二选择/sort。replace_keyed staging 从旧原状态推导业务 key，映射到目标 index 后一次写回原 DataTable。select_key/sort_key 与原 pointer/key/AX 都经同一受控 reducer 调用原 select_row/sort_by 等内部 helper，保持一次解释/一次结果；原 View 的 focus/capture owner 不变。按显示顺序导航由原 View 计算目标后交这个 reducer，不在 adapter 建第二份状态机。

DataTableView 的 keyed 入口接收 `&mut KeyedDataTable`（只读查询用共享引用），自动身份守卫后调用原内部布局/输入 helper；legacy/keyed 共用同一 helper 和同一 View 实例，不复制 View，不暴露 mutable 原模型让消费者自行路由。内部短暂借用不跨事务/回调逃逸。KeyedDataTable clone 分配新 instance identity；整个包装替换、dataset 换代、撤销再恢复的 incarnation/exposure 规则照旧。可选消费式 `into_legacy(self) -> DataTable` 明确丢弃全部 keyed 授权；原 View 按模型替换处理，旧 token/geometry/action 不可用于该 legacy 值，不能通过往返恢复旧身份。

A 必须用独立公共 consumer 真实编译原 `DataTable { selected_row: None, sort: None, rows: 2, columns: 1 }`、原解构和构造器，并用编译负例证明无法从 KeyedDataTable 取得 mutable 原模型、DerefMut 或写 selected/sort。源码检索不能替代编译。模型契约已修订，可由主协调安排 A 实施；本轮不实施或开票。v1 的 source-break 推荐已被此方案替代。

## 3. 原子更新、自动守卫与可用性

replace_keyed 预校验整个目标：row/column key 各自唯一；display_rows 只引用存在行且不重复；每行 cells 数量等于 columns 数量；所有计数/几何输入有界、counter checked_add 可容纳。零行/零列允许；零列不显示 row hit/可操作语义，不保留无可见 cell 的键盘目标。验证失败保持原模型、选择/sort、focus/capture、布局、token、槽 mount、队列完全不变。

同 dataset 持续 row key 保留唯一原选择；持续 column key 保留 sort direction。源行仍存在但退出 display_rows：选择及该行 focus 清空，不选相邻位置；重新展示不自动恢复。删除选中行清空；无层级/最近祖先恢复。删除 sorted column 清 sort，不自动选另一列；列重排按 key 映射。新 dataset 即使 key 相同也清选择/sort/focus，所有实体换代。更新不主动抢 Host 焦点；重新进入时原 View 根据当前有效选择或首个挂载行选择内部目标，header 可由 AX/程序 focus 进入原 roving 路径。

revision 负责几何/测量新鲜度，incarnation 负责 A→B→A 和跨整个模型替换。原模型单调分配实例 identity，clone 不继承旧 View 执行授权；row/column 删除、行展示撤销再恢复及 dataset 换代分配新 incarnation，counter 不归零。View 保存已接受的 key/incarnation 与 index 映射，不能仅依赖最近一份 TableReconcile，因为两次 View 调用间可以发生多次 replace。

原 View 每个 model-aware measure/arrange/paint/pointer/key/focus/AX 动作自动对比实例 identity 与 revision；不要求消费者手动 reconcile。变化先按持续 key 映射原 focus，取消 capture/hover，废弃旧布局；新 arrange 前所有旧坐标命中/AX 执行拒绝。纯选择/sort metadata 可刷新状态，不因选择一次就错误清焦点；会改变文本/排序标记/测量的变化使 geometry dirty。legacy 无模型的 pointer_move/layout/focus 入口不能授权 keyed 操作，补 model-aware 方法复用原实现；keyed 的旧无模型查询只给不可用，不信任缓存。

没有新增 per-row disabled/hidden flag。原 View 只有全表 disabled：清内部 focus/capture/hover，拒绝 row/header 动作；仍有效选择/sort 是数据可保留，启用不自动恢复焦点。行隐藏明确是 display_rows 省略；列不展示通过目标 columns/cells 原子删除，视为撤销该 column incarnation；没有树 collapse。祖先 active=false/collapse 是 Content 生命周期：停止发布/输入并撤销该次曝光，重新 active 不复活旧 token/动作/焦点。不能将完全 clip 的 mounted 行当成隐藏/卸载。

| 变化 | 原状态与旧事件 |
| --- | --- |
| 持续 key 重排/改文字/插入其它行列 | 选择/sort/focus 按 key 保留，capture 取消，旧坐标拒绝；曝光 token 可留，来源 revision 不同本身不使历史意图失效。 |
| remove→reinsert、display A→省略→A、dataset A→B→A | 新 incarnation/exposure；旧 AX、pointer-up、未 drain 动作及已 drain 延迟任务授权全部拒绝。 |
| 列数变化/同 ordinal 换 key | cell token 按 row+column key 校验，不以 stride 复用；删除列后旧 header/cell 不转给新列。 |
| viewport clip | 保逻辑选择/内部焦点和 token；无可命中几何，语义零面积只读；revealing 归原 reader。 |
| disabled→enabled / inactive→active / ancestor关闭→打开 | 撤销旧动作授权；恢复不自动 focus/capture；disabled/祖先隐藏恢复新 exposure，window activity 用单调 activity epoch。 |
| Cancel / CancelPointerCapture | 原 gesture 取消、capture serial 失效；已成立选择及先前有效历史意图保留，迟到 up 无新意图，不碰邻居 marked text。 |

## 4. 公共 Content、被动槽及 Host 事务

建议 `composition::DataTableContent::new(name, snapshot, displays, budget) -> Result<...>`，私有持有模型层 KeyedDataTable（内含唯一原 DataTable）、原 DataTableView 与展示槽。原 `Content::new(id, table)` 直接挂同一 ContentHost；消费者只提交业务快照和消费 typed intent，不转发基础输入、不计算行高或 AX frame。

displays 是按 column key 的 header、按 `(row key,column key)` 的 cell 和可选 cell trailing accessory。每个槽命令为 `Keep | DefaultText | Replace(Content)`；新实体 Keep 等于 DefaultText，持续实体 Keep 移动原实例，不按显示位置重建。accessory 的 DefaultText 表示清除；它是 cell 内被动尾随展示，占同一 column 宽度，无额外逻辑列、点击区、焦点或 AX owner。header name、row name、cell 文本/AX value 仍来自 snapshot，装饰不猜名称。首批没有整行 accessory/行操作按钮。

槽仅接受无交互、无输入/overlay/阅读选择 owner 的被动子树，如普通 Text/Icon/Separator 与被动 Stack/Grid。递归检验原 validate、interactive/focusable、text_input_snapshot/text_surface、overlay_scope/overlay_open/composing 等能力；拒绝 Button/Link/输入框/SelectableList/ScrollContent/选择型文档/菜单/嵌套 Table，即使外层 wrapper 自称不交互。检查基于能力和约束，不仅依赖类型名或 action_owner。拒绝返回 row/column/slot 和冲突 ComponentId。首批不承诺任意恶意自定义 Component 在只读查询中遵守协议；支持的 Component 必须遵守原 trait 纯查询契约。

**B 首批明确要求 caller-owned row wrapper ID**。候选 `TableRowDisplay { key: TableRowKey, id: ComponentId, cells: Vec<TableCellDisplay> }` 与 snapshot 全部 rows 一一对应，包含暂未展示行。id 由调用方分配，不能从 key/hash/index 自动制造；同 dataset 持续 key 的 Keep 必须保持该 row id 及 wrapper/槽原实例。更换 id 要显式 Replace，按旧挂载撤销处理，不能伪装成 Keep。row key 是业务实体、ComponentId 是原 Content/reading 项身份、exposure token 是语义授权；三者分离。每个 header/cell/accessory 及其后代 ID 也须整棵 Host 唯一。

row wrapper 是被动、fixed_slots 的原 Content 子树节点，没有新 scroll、focus、选择或 AX owner。装饰树用原 suppress_semantics 递归抑制，不只隐藏根留下重复 AX；表格虚拟 parts 负责唯一 Table 语义。未展示 wrapper 保留实例/ID、inactive，不绘制/输入；原 Host 普通 insert/remove/reorder 不得绕过事务改固定槽。排序 Keep 移动原实例而非 remove/reinsert；wrapper frame 必须来自原 View 本次 arrange 的整行 frame，clip 使用同一 visible，不由 consumer 推算。

新增原 Host 受控事务 `replace_table_snapshot(id, snapshot, displays, reading_update)`。只读 staging 必须覆盖**完整目标树**：模型与映射、所有保留但未展示的行/槽、全部 Keep 后代、row wrapper、header/cell/accessory 和 Host 其它分支；校验全部 ComponentId、instance/incarnation/exposure 与容量。不能只查 active/display_rows 或新 Replace。被移除旧子树不算目标，未动的其它 Host 分支必须算入。非法交互、ID 冲突、映射/预算溢出和 reading expected version 不符，都在任何 move/unmount、计数或 reading 身份迁移之前拒绝。

二维预算属于同一 staging。模型 A 的 `TableBudget` 要求构造时提供有限、非零 usize 的 max_units/max_utf8_bytes，不接受 unbounded 默认；B 取模型预算和原 Host/transport 限额的更严格交集。令 R=全部保留行数、C=列数、V=展示行数、D=全部 header/cell/accessory Content 后代数（含 Keep/未展示）、W=保留 row wrapper 数。先 checked_mul(R,C) 得逻辑 cell 数 M，checked_mul(V,C) 得展示 cell 数 Q，再 checked_add 求模型单位 R+C+M、逻辑槽单位 C+2*M（主体及可选 accessory 逻辑槽均计入）、wrapper W、后代 D、语义单位 1+2*C+V+Q 和目标 staging 所需条目之和 U。U 不得超过 max_units；即使空槽不分配也按逻辑槽收费，先计数再分配。字符串/owned payload 及可查询槽文本累计 UTF-8 字节须在 max_utf8_bytes 内；不能声称覆盖未知外部资源的内存预算。乘加溢出或限额失败返回 BudgetExceeded/Overflow，完全无副作用。Host 其它分支仍受原 Host 总树限额，wire 最终序列化仍受 MAX_PACKET/2，不能以 units 代替字节上限。

全部预检通过后才一次提交模型/槽/生命周期、计数、reading 更新与几何失效；提交段不得再执行可失败的用户回调或验证未 staging 条件。原 reading API 会先 advance version 再进入 callback，且文档明确不为 arbitrary callback 提供 rollback（冻结 content_reading.rs:507–532）；不能包一个可能失败的 replace closure 就声称原子。B 需最小原 Host staging/commit glue：先只读验证 expected reading version 和目标 ordered IDs，准备完整不可失败的结构提交，再在同一原 reading transaction 提交双方。任何错误必须保持原模型、槽 mount、focus/capture、queue、counter、reading version/items/anchor 不变；不得先迁移 reading 身份，再因 ID/budget 失败。此处不引入新 reading 引擎。

新 Content 可随错误返回，原实例状态不变。已挂载组件经 component_mut 仅允许不改变 ID 集合和 reading ordered items 的受控数据命令；其它变化须走 Host 事务，返回 HostTransactionRequired。脱离 Host 构造只验自身，挂载由原 Host 再验全树。动态子树修改发布前重检，非法则失败关闭该次发布，不半挂载或继续交互。

## 5. 原动作队列与曝光身份

原 ContentActionRecord 队列增加 `TableIntent` payload：dataset、Row/Column 业务目标、exposure、model revision 来源、activity epoch、`SelectionChanged | Activated | SortChanged{direction}`，并携带该事件后 selected_key/sort_key 快照。header down 和 Enter 只在原 sort_by 成功时各产一次 SortChanged；行 down 选择变化产 SelectionChanged，up 未变化不重复；行 Enter 已选产 Activated，否则仅 SelectionChanged。焦点 rove、paint、查询及后台 replace 无业务 action；不把全部 Changed 当 Activate。排序是意图与 sort metadata 更新，实际比较/重排由消费方回送 display_rows，公共 adapter 不执行 Gallery 字符串比较器、不推断排序成功。

复用 Tree 所需最小 `Component::action_payload_current(&payload)->bool`（默认 true、纯验证）的共享 glue；原 drain_action_records 在原 mount 校验后再过滤。若 #73/B 已交付该 hook，直接复用；否则由主协调串行接入，不能新建表格队列或过滤后重放事件。验证当前 dataset、实体 incarnation/exposure、logical mounted/enabled、activity epoch 及原 root mount，不要求来源 revision 等于当前 revision；持续同 key 重排的历史意图仍指同实体。已 drain 意图是历史记录，延迟业务执行前通过同一纯 `intent_is_current` 校验。

每个根 mount 为 column、header、row、cell 分配独立单调 checked_add usize token 作为 SemanticPart.index；内部 token→kind/key pair 映射，不用 index/label/hash/算术 stride。持续 row/column 重排保 token，当前 ordinal 随布局变；删除/隐藏/disabled/祖先 inactive 撤销授权，恢复新 token。cell 任一 row/column incarnation 撤销即换代；新 dataset 全撤销。counter 清空不重置，溢出原子拒绝；root remount 另由原 mount 隔离。window inactive 递增 activity epoch，未在 inactive 期间 drain 的旧意图在 active 后仍不可复活。Cancel 不抹除已生效历史事件。模型 revision、incarnation、exposure 与 root mount 分别承担不同职责，不互相代替。

## 6. 单一布局、Theme、阅读与 IME

在原 DataTableView 提取同一 measure/arrange/chrome helper，默认字符串路径和公共槽调用同一 helper；不是复制原 render 到 adapter。候选 `TableMetrics` 提供每 header/cell 的被动内容测量，`TableCellLayout` 发布 source index、current ordinal、cell/content/accessory frame 与 visible，原 row/header/column 布局保留。available width 按原均分；先扣 cell 原 8pt 两侧 inset 和 accessory 测量/间距，再测主体；row 高取所有 cell（含 accessory）最大值与原34pt最小值，header band 取各 header 最大测量高度，保留原6pt与row_gap。accessory 间距用既有 token，不新加 Theme schema；超窄时 clip，不挤出 cell。默认字符串回归必须与原 frame/hit/focus ROI 一致。

measure 与 arrange 使用同一 metrics；paint 消费已 arrange 布局画原 Quiet selection/hover/focus 与槽，不再测量一次并改写命中缓存。cell visible=cell frame∩row visible∩table viewport∩ancestor clip；pointer_regions、语义和 paint 消费同一交集。焦点沿原 paint_collection_focus 的 outset 规则，祖先 clip 权威，不能用 cell self-clip 裁掉行 focus。非有限/零宽高按原 clean_size/clean_rect 策略拒绝/归一，零列无除零和伪命中。缺文本/图标服务由原 ContentHost 能力错误报告，consumer 不补画。

Theme 沿原 ContentTheme/Palette/Surface、Content::set_surface(Some/None) 与 child_theme：继承、显式覆盖、清除恢复都用原机制；table 无新默认 surface/色表。原 View 接入继承 Surface 的必要扩展只改变原 Quiet palette 解析与槽 foreground，selected/disabled 墨色仍原 owner 计算，header/cell 显式 Surface 仍建立原边界。主协调与 Theme #71 串行确定固定输入；当前设计 snapshot 不含 live Theme，不伪称一致可运行闭包。

外层 ScrollContent/ReadingController 是唯一滚动/阅读 owner，wheel 走原路由。B 首批纳入 caller row wrapper ID 的逐行 reveal：display_rows 的 row ID 按显示顺序交原 reading transaction，几何只读原 arrange frame；纯排序 Keep 保 wrapper/槽原实例。隐藏/删除从当前 reading ordered items 移除该 ID，按原 reading 文档版本/anchor 失效与既有确定性 fallback 处理；不得把同 ordinal 邻行当成旧锚点。数据集换代或旧身份恢复不复活旧 reveal 授权。省略但保留的 inactive wrapper 仍参与 Host ID/预算校验，不是当前可 reveal item。显式请求沿原 expected version 校验，不手算 offset，不在 semantic query/paint 中隐式 reveal。先完成第4节 staging，再提交原 reading transaction；具体 fallback 保持原文档定义，不增第二恢复策略。#22/#28/#42 的完整 reading 恢复不在本票重开同义任务。

单一 Host Tab stop、原 View 内部 row/header rove；不增加 cell tab stop。primary pointer/ControlKey/Cancel/window lifecycle 映射原 View；header down 排序时机保持，二级指针不选择/排序。首批无 cell editor，因此不在 sort 中提交/取消编辑。邻居 Composer 组字时，pointer/Tab/AX Focus/Press 沿原 transfer barrier，不用“表格无editor”绕过 marked owner；后台排序快照、改文案、Theme、卸载 table 不结束邻居 preedit、不更改其 selection/binding/发送门禁。原 key-start/end 只解释一次。物理 IME 未测单列，不要求全库 IME/VO 才能接收本批。

## 7. 本地 Table 语义与最小双向 wire

新增 ContentRole 及 wire 对应：Table、TableColumn{index}、TableColumnHeader{index,sort}、TableRow{index}、TableCell{row,column}；sort 复用原 SortDirection 的语义数据映射，不把 index 当 token。表格根 parent 沿原 Content 树；column parent=table；header parent=column token；row parent=table；cell parent=row token。平台通过 cell column ordinal 关联 column，与当前原生角色一致。逻辑只有一个 cell，不为 AX 复制到第二 parent。

row index 是 display ordinal，column index 是当前 column ordinal；token 在重排后稳定而 ordinal 改变。row selected 来自原模型，row/header focused 来自原 View 新增只读 focus 查询，不能以 selected 猜焦点。header sort 来自原 sort key/direction；cells 只读、name/value 来自 snapshot 字符串。header/row 的 Focus 转给原 View 目标，header Press 调原 sort_by，row Press 调原 select/Activated 规则；column/cell 不允许 Focus/Press，保留原 Gallery 的只读 cell 边界。不新增 SetValue、SetSelection、resize 或 SetExpanded；DataTable 无 collapse。

语义发布全部当前 display_rows 和 columns，含结构父闭包；完全 clip 的已挂载 part frame 使用其原点零面积 rect、focused=false，只读且不可动作，不能给它旧的可点 frame。部分 clip 用真实交集；逻辑省略行完全不发布。当前 semantic_parts 没有 visible/action capability 字段，执行入口必须额外检查 token→part kind、当前布局 revision、非空 visible、disabled/activity。离屏键盘导航仍由原 View/外层 reveal 协调；首批不宣称完整离屏 AX 浏览。旧 geometry 的语义查询返回空/不可用，绝不隐式 arrange 或 reconcile。

**B 必须闭合真实平台零 frame 路径。** 冻结 platform accessibility.rs:331–378 的 validate_accessibility_nodes 对 width/height <= 0 一律拒绝，并要求全部 parent 存在；原 snapshot_validator_rejects_cycles_and_invalid_frames 测试也明确拒绝零高。shared.rs 经该校验才发布。因此仅生成 Content 零 frame 或漏掉 parent 都不成立。B 必须在主协调串行窗口补最小桥接：仅 Table 家族的只读结构节点允许规范零面积 frame（有限原点、width=height=0、focused=false、平台 enabled=false、read_only=true），保留真实 parent/token。有可见后代的零 frame table/row/column parent 也保留，不能为过校验漏父。其它非 Table 角色维持原正面积规则；负数/非有限 frame、孤儿、环、重复 key 继续拒绝。若 Tree 已改共用桥接，复用同一策略入口，不覆盖其 hunk。平台动作桥与 Content owner 执行层都拒绝零 frame readonly 结构，不能仅靠 enabled 标记假定程序请求会被挡住。

单纯 clip 时内部逻辑 focus 可留，发布 focused=false 不修改原 focus。后续显式 arrange 确认同一 live token 重新可见，且原 Host window/ancestor/owner 仍 active/enabled、Host 当前 focus 属该 table，才恢复发布原 View 实际 focused part；不主动创建 focus、不自动 reveal。纯 clip 保 token，隐藏/禁用/卸载后恢复必须新 exposure。header/row 每次 Focus/Press 都校验 root mount、token kind/incarnation/exposure、当前 model/layout revision、非空 visible、window/activity epoch/ancestor lifecycle 和 enabled；column/cell 的 Focus/Press/写动作在原 owner 执行层全部拒绝。

本地 B 须实际交付 controls 角色、app-host→平台 Table 投影和原 owner 动作，并沿 controls→app-host content_nodes→平台 validate/shared snapshot 接收路径验证零 frame parent 闭包、ordinal、可见后代、readonly 执行拒绝及重新可见 focused 发布。不能仅补 enums 或推迟给 C 后就声称 B 可用；路径仍拒绝 snapshot 则 B 未接收。这是本地结构/动作投影验证，不是物理 VoiceOver 或完整 native UI 验收。反复 semantics/part_parent/semantic_mount/intent_is_current 不改变模型 revision/token/focus/capture/marked/队列，不产生 action、隐式布局或 reveal。

最小共享 glue 是：上述角色及 SortDirection 数据、原 action payload/current hook、Host 原子更新入口、原 View 单一布局服务与只读 focus 查询。wire 无新表格事件引擎，继续原 Focus/Press 请求与 Target mount/part token。C 同时闭合 child 原 owner→wire snapshot→ProcessContainer/native_content→host Table 投影，和 host Focus/Press→wire→child 原 owner→真实 sort/selection 结果读回；数据型 role 映射、序列化往返不是双向完成。验证非法 ordinal/孤儿 parent/重复 token/错 kind、旧 mount/旧 exposure、inactive、超预算 snapshot 明确拒绝；未知新角色的旧 peer 报 unsupported，不降级 Group、不返回成功空白。精确接入文件由主协调按 Tree 共享改动后的实际路径登记。

## 8. 三个有限执行候选（不在本轮建票）

| 候选 | 精确验收 | 依赖、独占文件与写入边界 |
| --- | --- | --- |
| A：模型层 KeyedDataTable 与原 View 布局 | 原 DataTable literal/解构/构造器在真实独立 consumer 编译兼容；包装无 mutable/DerefMut/字段写出口负例；同一原 reducer，无双权威；keyed 快照/维度/重复 key/order/二维 checked 计数和模型预算错误原子拒绝；row/column key↔index↔ordinal 正确；自动 revision/跨 replacement/clone/dataset ABA 守卫，无手动 reconcile；down(A)→重排/删除/同 index 换 key→up 不选 B；column shrink 不重定向；原字符串布局/动作回归。 | 修订后的模型 API/reducer/literal 测试契约允许主协调安排 A 实施，仍等待 #73 Tree shared-file 接收窗口。独占新增 `controls/tests/datatable_identity.rs`、`examples/composition-consumer/tests/datatable_legacy_literal.rs` 及专属无可变出口编译负例；`extended.rs` 只新增模型包装/必要原 reducer helper，保留 DataTable 四字段，`collections_view.rs` 仅 DataTable/必要原helper，由主协调串行 hunk 接入，不改 Tree/List/Theme。A 不新增 ContentRole/Host 事务/reading/平台 bridge；A 接收后才实施 B。 |
| B：公共被动 DataTableContent、本地语义与事务 | 独立公开 consumer 直接挂原 Host；caller row wrapper ID 与真实 frame/原 reveal；排序 Keep 同一实例；含未展示 Keep 后代/wrapper/其它分支的全树预检；二维统一预算、ID/非法槽/reading version/token 错误提交前原子拒绝；实际 app-host→平台 validator/shared 路径保零 frame parent 闭包/只读执行；真实 Table 家族语义与 Focus/Press 原动作；重新可见 focus 发布、纯 query、ABA/原队列/合成 Preedit 负例。 | 依赖 A，独占 `controls/src/composition/datatable.rs`、`controls/tests/datatable_content.rs`、`examples/composition-consumer/src/datatable_content.rs` 及同名定向测试。`composition.rs/actions.rs/lib.rs`、原 content_reading staging/commit、consumer 导出、app-host accessibility、平台 accessibility.rs 零 frame 最小桥接/shared 路径测试均为主协调串行 glue；复用已接收 Tree hook/bridge，不双写。row wrapper/reading 首批必交，上述平台与事务契约未真实实现验证前 B 不可接收。B 不以 C 的未来 wire 代替本地完成。 |
| C：最小双向 wire、必要 GPU 与有限证据 | child snapshot→host 表格语义、host header/row Focus/Press→child 结果读回；旧 peer/非法 parent/token/ordinal/旧 mount 拒绝；固定源码与主题闭包的默认表格对原 View、被动 Text+Icon/accessory、长文本、零列/窄宽/clip/focus 必要 GPU 留原图与真实失败。 | 依赖 B 接收与固定 Theme。独占新增定向 wire 测试、`design/reviews/datatable-public-content-implementation-20261024/`；app-protocol accessibility、module-runtime content_accessibility/native_content、ProcessContainer、app-host 映射与 Tree 共用，主协调登记串行 hunk 窗口。矩阵/#8/#12/#14/#55 仅交证据/diff 候选，主协调最终接收更新。 |

A→B→C 是实际串行接收依赖，不以同时建票代替前置验收。shared extended/collections/composition/协议文件不得交独立 writer 同时改；新文件独占也不授权回写共享源。A 不实现 DataTableContent，B 不把本地语义缺口推给 C，C 不扩完整远端表格或全库 native 能力。

必要反例清单：

1. 同 name 不同 key、改 name 同 key、row 和 column 同数值 key、columns 重排/删除/增长、display_rows 排序/省略、零行/零列、重复与缺失 key/错误 cells 长度、所有失败原状态逐项相等。
2. 更新后未 arrange 直接 pointer_up/key/AX；View 两次调用间 row/column/dataset A→B→A；整个包装替换/clone 与 into_legacy 丢 keyed 授权；原 DataTable 外部 literal/解构编译绿例，KeyedDataTable 无可变出口编译负例；旧无模型入口不得授权；只读 stale 无副作用。
3. 行 down 的 SelectionChanged 只一次，up 无重复；Enter 选择与激活区分；header pointer_down/Enter 各切换一次 sort；只发排序意图，业务排序回送后同 row key 选择/焦点仍对；没有 resize/cell 编辑动作。
4. 记录后持续 key 重排仍可 drain；记录后隐藏/删除/disable/ancestor inactive 或 window inactive→active 后才 drain 必须丢弃；已 drain 延迟验证拒绝；Cancel 保已成立历史，迟到 up 无新动作；根重挂同 ID 仍拒绝旧 mount。
5. 被动 Text/Icon/Stack/accessory 可测量；Button/TextField/Scroll/选择阅读/伪被动 wrapper 递归拒绝；Keep 跨排序保原 Content mount；完整目标树冲突含隐藏 Keep 后代/row wrapper，R*C/计数和溢出、统一预算、reading expected version 失败均在提交前拒绝，lifecycle/counter/capture/队列/reading version/items/anchor 不变。Header/cell name 不从槽重复播报。
6. 默认/长文本表头、多行 cell、accessory 宽度、0/窄宽/部分与完全 clip；真实平台校验/接收路径保零 frame parent 闭包、ordinal 和 readonly 执行拒绝；query 无隐式 reveal/状态改变；重新可见按原 Host focus 发布，不复活失效 lifecycle；同 geometry 旧 exposure 仍拒绝。caller row ID 原 reading/reveal 测隐藏/删除锚点不转邻行、排序 Keep 保实例，不双滚动。
7. Theme 继承/覆盖/clear 与 disabled/selected ink；普通 Text→Table→Composer 独立消费者；邻居合成 Preedit 中后台更新/Theme 不改 binding/selection，Focus/Press/Tab/pointer 走原 barrier；语义重复查询无状态副作用。

后续 cargo 验证必须使用原 `scripts/tekesui_build.py` 的 build_lock/environment，`cargo +1.95.0 ... --locked --offline`，仅既有 `target/review-cache`；不 prepare/clean/install/new target/复制 target。锁被占用则记录未执行并等待主协调窗口。必要 GPU 用接收后的固定源码/Theme，不把包检查当 GUI 验收，不要求全库 native/IME/VO 成为本批关闭前提。

## 9. 主协调 tradeoff 与本轮验证状态

主协调已咨询 advisor 并选择兼容方案，本轮仅修订设计：①原 DataTable 保持全 public literal，模型层 KeyedDataTable 拥有原状态和 metadata，同一 reducer，无 mutable 出口/双权威；②二维父链接受，本地 B 补实际平台零 frame 结构桥接/动作拒绝、原逻辑 focus 与重新可见发布、纯查询无 reveal；③B 必交 caller row wrapper ID、真实 arrange frame、原 reading transaction，完整目标树与统一二维预算在任何迁移前预检。原 v1 原样保留在 review/design-v1.md，SHA256 `fa1147dde7a2bc8d1611c569552ea99b1b6620d5e5e3f2f55ff6bb3e55ef46c3`，原 source manifest 不改。A 可按修订契约安排实施，B 细节未实现验证前不宣称接收；本子会话不联系 advisor/旧会话，不新增批准流程。

原文依据：#67 body 的“行/列稳定key、header/cell/accessory Content槽”“测量/裁剪/命中与语义几何一致”及“不预设所有单元格必须支持任意交互”；原 DataTable 四字段/方法见冻结 extended.rs:1044–1100；平台正面积和父闭包校验见冻结 platform accessibility.rs:331–378；reading callback 边界见冻结 content_reading.rs:482–532。修订决定来自本会话主协调直接指令，不将未收到的 advisor 逐字意见或设计静态检查冒充外部评审/实现证据。

本轮实际完成：读取 Issue/原源码/Tree 契约、15 文件 hash 快照、有限设计和三候选拆分、设计静态检查。未执行缺 API 编译红例（源码检索仅证明当前无公开 DataTableContent，不能声称编译证据）；没有 cargo/GPU/新 API/新行为测试，没有生产源码改动。原生窗口、物理 IME、VoiceOver、性能、完整 Gold7、公开不可变发布及第二产品真实接入均未验证。设计不是实现接收，#67 保持 OPEN；未提交/推送/PR/发布/关单。


## 10. v3 主协调架构决策：有界生命周期与失败输入所有权（2026-10-24）

本节是 #85 的正式设计修订，尚不是实现接收。#84 模型已接收；#85 revision1/revision2 的旧失败、候选和封存证据不改签。v2 及上文历史执行状态保留，本节补足第4节原子事务无法从原 trait 推导的前提。主协调已读取原调用图并两次咨询 advisor。

### 10.1 可表达的事务兼容被动子树

普通 `Component` 的 lifecycle、Cancel、focus、activity 或 Drop 可以合法调用返回 Self 的 `Content::new`，并动态增删子树。仅数当前后代不能界定 hook 的 identity 分配数；原有限 u64 allocator 耗尽只能 panic，无法在先移走 live children 后成为可恢复错误。原接口也没有通用可克隆隔离状态。故本 typed Table 事务必须公开增加一个可由任意消费者实现的、递归的事务兼容能力声明，默认 Unknown；具体公开命名随实现/API 库存同步，不能使用 unsafe 或库内具体类型白名单。

能力承诺适用于整个受影响子树和提交完整调用链：Cancel、set_focus、lifecycle、activity、可变 children 访问及 Drop 均不创建 Content/mount identity、不增删或换序当前声明子树、不重入事务；纯查询仍无副作用。原 Text/Icon/Separator、被动 Stack/Grid 等只有经实际源码审查满足承诺才声明；自定义 wrapper 可声明同一能力并组合真实任意被动 Content，不退化为字符串或固定组件枚举。所有新 Replace、持续 Keep 及隐藏后代均须递归满足能力和原无交互/输入/overlay/reading-owner 条件。

这是比 v2 单独“被动”更强的公开准入边界，不能称任意动态 passive lifecycle 的原范围无损保留。需要惰性建树的消费者须先在事务外构造完整目标子树，再提交并声明能力。普通 Component、普通 Host 和其它 adapter 的合法动态 lifecycle 不受此新 Table 门槛影响。动态表数据/完整快照、Replace/Keep、隐藏行、真实自定义装饰和独立公共消费者等原功能必须完整保留。能力是可审计的语义契约，不是防御任意违约自定义实现的证明。

### 10.2 所有拒绝路径交还拥有的原输入

构造和 Host 更新入口若按值接收含 Content 的 displays/update，任何失败（包括事务已在进行、目标不存在、Unknown 准入失败）都不能由 SDK 析构未知输入。Drop 本身可能合法创建 identity；即使早返回也会破坏失败计数不变，或在极值 panic。

使用拥有原完整更新对象的拒绝结果，或等价借用 staging、成功才接管的形式；失败须交还所有原 Content-bearing 输入和其余更新内容，不能先拆走/丢弃已检前缀。读事务 guard 或最早错误也遵守此边界。调用者之后主动销毁退回对象是该 API 事务之外的动作，测试与文档明确区分。不得通过 mem::forget/leak、吞 Drop 或省生命周期通知掩盖副作用。

### 10.3 原 allocator 一次 checked 预留及提交

先以不分配映射/ID集合的结构计数与 checked 乘加完成统一预算和保守 staging peak admission，再构建只读目标 plan。最后取得原 table/reader 的可变借用并复验 expected version/身份，通过后、任何 live mutation 前，从原 NEXT_MOUNT 一次 checked 预留完整 range；不建立第二 allocator、ambient fallback 或默认新全局身份域。

令 N(c) 为已准入稳定子树的全部节点数，完整成本包含：每个新 row wrapper constructor 1；mounted 新 header/retained-row body/accessory 的 Replace 各 N(c)；mounted 新 wrapper 的 wrapper+全部目标后代（slot 不双计）；所有被移除/替换旧 display 的 Unmounted 后代（含隐藏行/删列/dataset 换代）；被移除旧 wrapper 自身（已抽走的 display 后代不重复计）。连续 Keep 为0。activity/focus/Cancel/Drop 不再额外分配的依据是10.1公开承诺，不能遗漏这些调用链。

内部有限 constructor/lifecycle helper 只消耗这批预留身份，公开旧入口行为不变。预留不足时原 counter 和 live 模型、生命周期、reading version/items/anchor、queue/capture 全不变；MAX 下零成本 Keep 可成功。成功预留后不得有普通 Err 尾段，不省略或重排原 lifecycle/Cancel/focus/activity/Drop 顺序。OOM、任意自定义 panic/违约行为不作虚假回滚保证。

### 10.4 必须验证的补充边界

在原五项验收之外补足其实现证据，不能只证明负例绿：合法动态 hook 的 Unknown 输入被完整交还且 SDK 不调用其 Drop；自定义 wrapper 显式声明后可携真实 Text/Icon/嵌套被动树进入独立消费者；原普通 Host 动态 lifecycle 仍可用；实际 u64 MAX 零成本 Keep、全子树预留不足、隐藏/删除后代卸载、最早 reading guard 拒绝、budget 先计数、commit 前 expected 复验。每一失败逐项对照 counter/live 状态，成功保原回调通知和顺序。此前候选 lifecycle 编译错误不是测试执行证据。#85 在这些及原全部条件未实际验收前保持 OPEN，#86/#95 继续等待。


### Main integration clarification: complete activity hooks (2026-10-24)

The existing StableHooks promise covers both `window_active_changed` and `presentation_runnable_changed`, their forwarding and every actually notified descendant, in addition to Cancel/focus/lifecycle/mutable children/Drop. During the checked transaction these callbacks must not create Content or mount identities, change the declared tree, or reenter Host/typed transactions. This explicitly names the full activity chain already required by v3; generic custom opt-in remains public, with no component whitelist.

The #79 union retains inactive hide/revoke, immediate runnable=false withdrawal, then effective activity propagation. Activity=true alone never publishes runnable=true: only a successful current layout may do so. Table visibility transitions must exercise a compliant custom passive slot, callback order, zero-cost MAX Keep and no pre-layout runnable revival in the root combination. No existing hook is skipped to satisfy mount budget.


## Main B implementation acceptance (2026-10-24)

Issue #85 original five local B criteria are accepted on the serial current source union, including generic stable-hook opt-in, borrowed failure ownership, original-allocator full reservation, readonly indexed target/reader paths and final expected-version revalidation. Main root evidence: `design/reviews/table-main-intake-20261024/root-acceptance.json`. Current all295 library tests, Table19/identity9/Tree14/reading24, public Table3 plus original mixed Notice/Composer/EmptyState/Toolbar, actual local platform3 and AppHost2, local-only wire refusal8, all-target strict, downstream and selected legacy composition/View/literal/Skeleton regressions passed. The custom nested activity probe proves MAX zero-cost Keep, inactive false withdrawal and no runnable=true before successful layout.

Earlier actual half-commit/MAX and mutable-DFS failures remain archived; the first main activity test also preserves its incorrect initial exact-log assumption and the documented fixture correction for original pre-layout false reconciliation. No production gate was weakened. #86 remote wire/GPU is separate; #67 full delivery remains open. B metadata is candidate-scoped per original fifth criterion;335 public paths are explicitly classified, while a new canonical #8 refresh is required for changed source instead of re-signing old18 observations. Native/physical IME/VoiceOver/performance/release remain unverified. #95 List and #102 Split may reuse this accepted core (same original allocator, generic capability, ownership and indexed-path discipline); their own bounded notification plans remain their responsibility.


## Main finite C acceptance — 2026-10-26

Main accepts #86's original finite C implementation and evidence after the main-only metadata handoff. This completes the original A/B/C split of #67; it does not establish all-DS, native or published-revision acceptance.

- Protocol support adds `tekes_ui_app_protocol::component::content::TableSortDirection` (defined in the private accessibility module and publicly re-exported by content), alongside exact Table/Column/Header/Row/Cell roles. It is transport support, not a Controls public component, route or alias. Historical complete pre-Table Reply rejection remains distinct from #111's actual old complete ClientMessage decoder/peer evidence.
- Main21's independent original Table owner/receiver consumer passed with 262 transport trace records, stable 1737-input run identity and actual parent/child PIDs. Header/Row original Focus/Press, native-projected sort, readonly cell/column refusal, malformed ordinal/orphan/duplicate/kind/budget, old mount/exposure/activity and clipped retained geometry are covered. Zero-area Table parts no longer publish a false visible focus after final parent clipping; stable token and true owner focus remain. Refused actions preserve full owner state; layout restore preserves business focus/selection/sort/queue while allowing geometry bookkeeping to change.
- Current default and independent external Table GPU builds/runs each passed 64 cases under the original build/GPU locks, with stable source/artifact identities, production View-to-Host RGBA delta0, outside-clip0, actual custom glyph/icon and focus ink. Evidence: `/Volumes/HP P900/DEV/TekesUI/artifacts/main-scene-notice-20261024/tablegpu-current-root-20261026-01/`. Default executable SHA256 `692f0ce7cf71daee5a239661877605391abce03c5c440b1ddfd414a834fecbfd`; external `cd8b00e0b08bc0cf5e09d57deb0f2f30830e32fd4c885f71b24c95ec272c6e84`.
- Main20 runtime119/public52/build/strict and main21 owner build/strict passed. Their source/program manifests retain their execution-time identity; the later documentation and finite metadata edits are not retroactively relabeled runtime inputs.
- Main synchronized eleven existing DataTable-family API reasons, the three existing rest/focus/selected coverage notes and corresponding capability entries/metadata hashes. No state classifications, other component behavior or eighteen old observation seals were changed. The eleven DataTable classifications match actual source reverse coverage. Complete capability freshness is still NOT_PASS: unrelated Stack observation hashes are stale and the broader saved derived public surface has351 entries versus417 current classified paths. That independent #8 refresh remains open; no full-inventory freshness is implied by this bounded C handoff. Evidence: `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-86/metadata-intake-main-1737/main-r2-intake.json`.

Historical C geometry/GPU RED candidates and old schema receipts remain immutable. NSWindow/native, physical IME, VoiceOver, performance and release are unverified; whole Gold7/Design System is not an added #86/#67 closure prerequisite.

### 2026-10-31 #198 Collections-only 接纳来源 r4

本轮以不可变 `9725cdccc4609212587f5d88a1e5aa3cb0032e67` 的实际 Combo/Selection22 schema 为前置；仅接入原 List/Tree/DataTable 有限合同与 Tree2＋Table5 角色。原 SetExpanded、Combo/Selection、capture 与完整旧 decoder 保留。普通 Link 仅为原被动槽负例的实际类型依赖，复用 Nav owner 的模块/View/painter，不接导航主体。reading R2 与成功更新撤销旧 Host 输入凭证沿原 #198 r2；DataTable prepared decision 使用原模型/View owner。当前只完成隔离来源与静态适配，未编译或运行新三个负例、原 consumer/IPC/GPU；原证据按原输入身份复用，不能改签当前通过。详见 issue-198/collections22-minimal-20261031-r4/HANDOFF.json。

### 2026-10-31 r5 精确前置兼容

接纳包改以不可变 `6ec7a53dcf28bbefff866c4d80e43ffee6c7bbd4` 的实际 Nav/Search/Combo/Selection26 来源为前置，仅加原 Tree2＋Table5（26→33）。已准入的普通Link/module/View/painter与三个Nav Host修复保当前字节；原Link负例直接消费它们，不重装。old26完整decoder重新取该实际对象并记录Git blob/hash，不重命名old22；原R4/失败保持。Source-only、typecheck/三个新负例与原consumer/IPC/GPU均未运行，窗口仍由原域执行。

### 2026-10-31 #198 有限 Table 正常交付收据（候选 r1）

本追加保留 r4/r5 和旧 Table 失败的各自身份。唯一正常 Table 完整 IPC 收据位于 `delivery/integration-executor-20261030-09/table-runtime-ack-normal-window01/run/HANDOFF.json`，其实际源为不可变 `089dc65d958d62900fc68ba6e7ce41f603f05200`、tree `7f3b7241d3f12b745236177ab3b6ca36f31e9d9d`。四条命令均退出 0；两项注册 CPU 测试通过；只执行一次真实 `table_owner_receiver` IPC，得到 `TABLE_PROCESS_PASS` 和 ordinal/orphan/duplicate/kind/budget 五种 malformed receiver 记录。原业务、恢复、stale 与 inactive gates 全部到达；实际程序 SHA256 `ffca741cbdc18c3adbc740f2868d1001f68eca1e6c0275ad7d7d234a8c545045`，parent PID 24036 与 child inspect PID 24061 不同。638 archive entries、397 unique retained paths 已核验，27 owned PIDs absent、4 groups empty，源/输入保持、guards 与 lease 释放。精确结果和证据清单以该 handoff 为准。

不得把此成功倒签到旧 `cbe404...` 的 Table IPC 失败；也不得把 `853eed2f20792a2c1c64ae485b5aa82f8f650de5` 文档 successor 当作已编译源。旧 GPU lane 属 `70ee52503eeb9e830268d40918f42f005aa1da3b` 并依原身份保留；`collections-gpu-ipc-window02/continuation01` 中 List/Tree 是其各自的 cbe/70ee 收据，不是本 Table run 的共享来源。

状态仍是有限 Table domain review complete、公共交付未完成：handoff 明确 broader Issue #198 OPEN，PR/CI/release 未由该 domain run 交付；native、physical IME、VoiceOver、性能和全 DS 验收不在本收据内。
