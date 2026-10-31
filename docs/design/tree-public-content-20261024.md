> 2026-10-30 #198 恢复候选：下文原设计及历史接收身份保留。当前 4815c5a 兼容适配尚未编译；历史 pass 不等于当前主线/候选通过。公共交付由唯一集成批次完成。

> 2026-10-24 主协调接收并集成 C/#75 revision2：Tree wire角色/SetExpanded由真实child原owner处理，host及directbridge零面积操作拒绝；原TreeView仅两处改用已解析Quiet text/icon包，默认/外部原80实际GPU绿，旧64失败保留。当前rules1.9及新Renderer组合再次通过双向进程/80GPU与目标回归，source与各运行身份分列于 design/reviews/tree-protocol-gpu-20261024/root-intake.json。原生/物理IME/VO/性能/公开pin及全库成熟度未提升，原B/设计历史字段保留。

> 2026-10-24 主协调接收 B/#74 sealed revision1并集成。新增24Host契约/5内部负例、62原回归、54原TreeView几何指令对照及本地AX/selector/独立consumer均按各自来源验证；原子更新、RowSlot结构防线经focused advisor复核。根组合1534输入再次通过24Tree契约+3registry+7Skeletonconsumer与controls lib strict，保留Skeleton独立插入。证据 design/reviews/tree-content-implementation-20261024/root-integration.json。wire/GPU由C/#75继续，本接收不表示父#66或整个DS/原生完成。

# #66 Tree 公共 Content 与稳定身份：有限设计候选

日期：2026-10-24。状态：A（#73）已由主协调接收；B（#74）本地隔离候选实施与验证中，尚未被主协调接收；C（#75）尚未启动。原验收与反例保留，不勾选或关闭 #66。关联：[#66](https://github.com/TekesApps/TekesUI/issues/66)、[#67](https://github.com/TekesApps/TekesUI/issues/67)、[#68](https://github.com/TekesApps/TekesUI/issues/68)。后两票仅共享身份约束，不在本设计改造其模型。

## 1. 原实现与可重现缺口

正确源为 `/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI`，HEAD `1a7614a273e1a897ddc8a7ad8cb41d303a4a1290` 加原 WIP。本轮以原 Separator945 冻结 `design/reviews/current-source-observation-20261024/source-snapshot-separator-closure-final/` 为核心源码输入，manifest SHA256 `6e1dddd2616ec14f68161b40edbf5f3f062064d8757113cc2ebb755965c3bf30`。逐文件核验945输入；extended.rs、collections_view.rs、composition.rs、composition/adapters.rs 与现场相同。协议、runtime、app-host 桥接另作只读补充，不声称属于945。身份详见 `design/reviews/tree-public-content-20261024/source-identity.json`。未复制 live Theme，未复制 target。

原代码事实：

- `extended.rs:893` 的 TreeNode 只有 label、parent:Option<usize>、expanded、disabled；Tree 持有私有 Vec 与公开 selected:Option<usize>。add 返回位置，只有追加，没有删除/重排/稳定身份 API。visible 按 Vec 顺序检查祖先展开，并非独立 DFS 排序器。不能把 filename、label、当前 index 或 ContentId 自动视为业务 key。
- `collections_view.rs:550` TreeView 是唯一交互/Quiet 行/分支图标 owner；CollectionInteraction 保存 index 型 hover/focus/capture。render 计算文本高度、34pt 最小行高、20pt 层级缩进并发布 CollectionLayout；图标经 render_icons 单独使用同一布局槽。pointer_down 选择并捕获，pointer_up 对整行分支执行 toggle；Enter 激活而非展开，左右键经 Tree::key。首批保留整行分支点击语义。
- `TreeView::set_expanded` 已有明确状态与挂载守卫，收起会恢复隐藏的选择/焦点到该祖先并清 capture。pointer_up/左右键目前另经 toggle/key，不能假设所有收起路径已经有同样恢复逻辑。
- `composition::Component` 已有 measure/arrange/paint、children、active、semantic_parts/parent、lifecycle、pointer_regions；ContentHost 已有 mount、capture serial、marked transfer barrier、原动作队列、主题继承和阅读事务。不要新 Host 或滚动、选择、键盘状态机。
- 现有公开 SelectableListContent 不是 Tree adapter；其索引型语义不能直接复制。ContentRole、wire SemanticRole 尚无 Tree/TreeItem；Content AccessibilityAction/wire action 尚无 SetExpanded(bool)。macOS 原平台已有 Tree/TreeItem{level}、SetExpanded；app-host 明确拒绝 Content disclosure。Gallery 的既有 Tree 原生适配不构成公共 Content 接入。

已实际执行独立公开导入红例：`missing-api-consumer/src/main.rs` 导入 `composition::TreeContent`，Rust 1.95.0 cargo check --locked --offline 返回101，E0432，精准缺该导出。原 build_lock/environment，唯一既有 target/review-cache，无 prepare/clean/安装/新target。见 missing-api-execution.json、missing-api.log。这只证明945无该公开 API，不证明未来设计编译或新 Tree 行为成立。

## 2. 首批边界与最小模型增强（必要）

支持有限内存单选树、稳定 key、被动自定义行、动态整快照更新、原键盘/指针/取消、Tree 语义。排除多选、拖放、行编辑器、任意交互 cell、文件系统解析/权限、异步远端数据源、大规模虚拟化与性能承诺。沿用外层 ScrollContent/ReadingController；Tree 本身不引入滚动引擎。

建议新增以下原模型 API，名称为本设计候选，不是当前可用 API：

```rust
pub struct TreeNodeKey(pub u64);   // Eq/Ord/Hash；调用方分配的业务实体 ID
pub struct TreeDatasetKey(pub u64);
pub struct KeyedTreeNode {
    pub key: TreeNodeKey,
    pub parent: Option<TreeNodeKey>,
    pub label: String,            // 明确可访问名称与默认文本
    pub disabled: bool,
    pub initially_expanded: bool, // 只用于新节点
}
impl Tree {
    pub fn replace_keyed(&mut self, dataset: TreeDatasetKey,
        nodes: Vec<KeyedTreeNode>) -> Result<TreeReconcile, TreeUpdateError>;
    pub fn node_key(&self, index: usize) -> Option<TreeNodeKey>;
    pub fn node_index(&self, key: TreeNodeKey) -> Option<usize>;
}
```

TreeNode 现有字段与 legacy add/select/toggle/key 保留；私有并行 key 表及 dataset/revision 保存于原 Tree，不给 TreeNode 添加必填公共字段。legacy 模式 add 产生无业务 key 的节点；首次成功 replace_keyed 明确切入 keyed 模式，此后 add 返回 None，不允许混合追加。公开 TreeContent 只接收完整 keyed 快照，不默默借 label 造 key。旧独立 legacy Tree 调用无需迁移，不能承诺其 index 能跨替换保留身份。

keyed 模式中选择的权威值仍在原 Tree 内：私有 selected key，公开 selected:Option<usize> 仅是兼容投影；原 select(index)/key 路径必须同步这一权威值。直接写公开 selected 仅在 legacy 模式是受支持操作；keyed 模式不能借它选择另一实体。每次原 View 的 model-aware 入口先校验该投影，若被直接写改则从原 key 恢复投影并取消旧 gesture/布局，不能把“合法范围内的另一个index”当成选择命令。只读几何/AX查询发现不一致只返回不可用，不修改模型。keyed consumer 必须用 select/select_key；不新增 adapter 自有 selection 状态机。

快照是明确的根及兄弟顺序，要求 parent 先于 child，且每个子树连续（preorder）。拒绝重复 key、缺 parent、自引用、环、非连续子树；失败原模型、槽、layout、选择和捕获均不变。使用预校验 staging，成功一次交换。构建 key→index 与 index→key；索引只是本次存储地址，语义/display ordinal 不与 key 混用。保持相同 dataset、持续存在 key 的 expanded/selected；新 key 使用 initially_expanded；disabled 节点不得保持 selected。dataset 改变视为整个数据集换代，即使数字 key 相同也不保留状态。

TreeReconcile 给原 TreeView 用的旧→新 index 映射、移除集合、被隐藏集合及修订号；只供内部协调，不是第二份模型。原 TreeView 新增 reconcile_model：按映射保留仍有效且可见的原 focus，取消 capture/hover，清旧布局/按钮视觉缓存；移除 focus/selected 仅可恢复到旧祖先链中仍存在、可见且启用的最近祖先，否则清空。数据集替换、卸载、窗口失活不恢复历史 focus。键盘新进入时由原 View 选择当前 selected 或首个可用可见节点，不在后台更新时主动抢焦点。

replace_keyed 是公开 API，正确性不能依赖调用方记得手动 reconcile。原 View 在每个 keyed paint、pointer/key/focus/disclosure 与 AX 动作入口，自动对比已接受 model revision；不一致先 reconcile、取消 capture/hover 和旧布局，旧几何输入立即拒绝，只有新 arrange 发布后才可命中。原 pointer_move/render_icons/layout 等不接收模型的 legacy 入口不能授权 keyed 操作：补 model-aware 入口复用同一内部实现；keyed 模式下旧无模型入口返回不可用/不绘制，而不是信任旧缓存。所有几何/AX只读查询检查 revision，若不相等返回空/None，不隐式 arrange、reconcile、选择或派动作。

View 保留上次已接受的 index→(dataset,key,incarnation) 及旧祖先链用于映射，不能仅依赖最近一次 TreeReconcile。原 Tree 的 per-node incarnation 由单调计数分配，删除/隐藏/disabled 后再现或dataset换代重新分配；即使两次 View 调用之间已经 A→B→A，也能辨识中间撤销。结构/可用性 mutator 必须同步更新原模型 revision/incarnation；纯选择变更不让布局 revision 误失效。identity counter 不因清空/换dataset归零。替换整个 Tree 实例按新模型身份处理，clone 不继承对旧 View 的授权；这是原模型的失效信息，不是第二份交互状态。

disabled 规则沿原 Tree：只禁该节点，disabled 祖先不级联禁用仍展开的启用后代；导航可跳过禁用祖先到其可见启用后代，不能操作该祖先的 disclosure。原 Tree::set_disabled 对当前 selected 清空；keyed 更新同样清空被禁用节点的 selected，View 同时清该节点 focus，不自动选祖先。禁用祖先本身不收起，其启用后代原选择/焦点可保留；如果另有收起/删除使后代隐藏，才按旧祖先链恢复到最近“存在、可见、启用”的祖先，否则清空。全树disabled则清View焦点/捕获并拒绝全部动作；恢复不自动复活旧焦点。原模型 selected 是数据状态，可保留全树禁用前的仍有效选择，不能将其解释为焦点或授权。

所有收起路径统一走原 View 的明确展开实现：pointer_up 与左右键不能继续绕过隐藏后代恢复；Enter 保持 Activated。Tree::key 对 legacy consumer 的导航含义保留，View 层只收拢展开后的 reconcile。添加原 View 的只读 focused_index 查询，语义不以 selected 猜焦点。

## 3. 公开 adapter、展示槽与输出

建议入口及变更命令：

```rust
pub struct TreeContentNode {
    pub row_id: ComponentId, // caller-owned wrapper ID, independent from key/display IDs
    pub node: KeyedTreeNode,
    pub display: TreeDisplayUpdate,
}
pub enum TreeDisplayUpdate { Keep, DefaultLabel, Replace(Content) }
impl TreeContent {
    pub fn new(name: impl Into<String>, dataset: TreeDatasetKey,
        nodes: Vec<TreeContentNode>) -> Result<Self, TreeContentError>;
    pub fn replace_nodes(&mut self, dataset: TreeDatasetKey,
        nodes: Vec<TreeContentNode>) -> Result<(), TreeContentError>;
    pub fn select_key(&mut self, key: TreeNodeKey) -> bool;
    pub fn set_expanded_key(&mut self, key: TreeNodeKey, expanded: bool) -> bool;
    pub fn selected_key(&self) -> Option<TreeNodeKey>;
}
impl ContentHost {
    pub fn replace_tree_nodes(&mut self, id: ComponentId,
        dataset: TreeDatasetKey, nodes: Vec<TreeContentNode>)
        -> Result<(), TreeContentError>;
}
```

2026-10-24 B主协调补充裁定：row_id由调用方分配，普通ComponentId，与业务key/display后代ID独立但同属Host唯一ID检查。相同dataset持续存在同key的Keep/DefaultLabel/Replace均必须保持row_id；否则RowIdentityChanged原子拒绝。需要换wrapper ID须先显式移除再新建；新dataset即使复用数字row_id也创建新原mount/exposure。prepare不构造wrapper且没有global/magicrange ID allocator；碰撞调用方可换合法ID恢复。DefaultLabel/Replace只换display，保留持续wrapper原实例/mount。

新建时 Keep 表示默认 label；同 dataset 持续存在节点的 Keep 移动原 Content 实例，绝不按显示 index 重建。DefaultLabel 明确撤销自定义槽；Replace(Content) 明确替换且按原 Content mount 生命周期卸载旧槽。改变 dataset 的 Keep 不取回旧槽。TreeContent 私有持有原 Tree、TreeView 和行槽，不公开可写 model/view 绕过 reconcile。消费者 `Content::new(ComponentId(...), TreeContent::new(...)?)` 直接挂原 ContentHost，原 get_mut/component_mut 后调用受控更新；不转发基本输入或绘制。

replace_nodes 是跨模型、全部display子树/ComponentId、token容量和行槽生命周期的一次原子事务。先只读 staging：校验整个目标 keyed 拓扑、包括 Keep 保留槽在内的全部被动约束、目标树内ID唯一及与原 Host 其余树的ID冲突、模型revision/incarnation/exposure计数容量、全部目标映射。Host 全树ID冲突必须在上述 ContentHost::replace_tree_nodes 受控更新事务内预检，不能 component_mut 提交后等render报 DuplicateId 才发现；脱离Host构建仅能检查自身，挂载仍由原Host检查。上述验证全部成功后才一次提交 Tree、槽顺序、token表、layout失效与原生命周期通知；Keep 原实例不提前移动/卸载，不能先 Tree::replace_keyed 成功再因非法槽或token溢出回滚一半。任何错误模型/选择/展开/焦点/capture/队列/token计数和原槽mount均不改变；新传入Content可以随错误返回给调用方，已挂载实例不受影响。原 ContentHost get_mut/component_mut 只可调用不涉及全树ID变更的受控命令；涉及display替换/ID变化必须经上述Host事务入口。已挂载 TreeContent::replace_nodes 只接受保留现有display ID集合的更新；其它更新返回 HostTransactionRequired，不允许绕过Host全树检查。

首批槽只允许普通 TextContent、非可选择的被动文档、IconContent、SeparatorContent、被动 Stack/Grid 等组成的无交互子树；不是按类型白名单硬编码，递归检查 interactive/focusable、text_input_snapshot/text_surface、overlay_scope/overlay_open/composing，并运行原 validate/唯一ID校验。Button、Link、输入框、SelectableList、ScrollContent/可选择阅读 owner、菜单、Tooltip交互 owner、嵌套 Tree 全部拒绝，并返回包含 node key 与 child ComponentId 的 TreeContentError::InteractiveDisplay。动态槽修改后 mount/render 前也重检；失败关闭本次输入/绘制发布，不半挂载。不能只借现有 NestedInteractive 检查，因为 Tree 不是原 is_action_owner。

每行用私有被动 slot wrapper 暴露原 children，隐藏/折叠 wrapper.active=false；逻辑存在但仅被 viewport clip 的槽保留 active，不当卸载。wrapper 内对其 display Content 调用原 pub(super) suppress_semantics（抑制整棵展示子树，单独 semantic_hidden 不会抑制后代），行名称明确来自 node.label，避免图标/装饰文本重复 AX；首批不承诺自定义富文本逐字符阅读。行槽可含自己的 ContentId，必须全树唯一；key 与 ContentId 是不同命名空间。非法修改只通过 Tree 的受控入口诊断，不允许 Host 任意 reorder/remove 固定行槽（fixed_slots=true），根内容仍使用原 Host 更新能力。

输出沿 ContentActionRecord 队列，新增 `ContentActionPayload::TreeIntent(TreeIntent)`，包含 dataset、key、节点 exposure token、本次 model revision、activity epoch 和 `SelectionChanged | Activated | ExpansionChanged { expanded }`。仅真实模型变化/原 Activated 发对应意图；paint/焦点/幂等展开不发业务动作；一次原事件至多一个意图，选择变化加展开同时发生时按明确事件结果生成 ExpansionChanged 并附 selected_key 快照，不重放两次 Activate。消费者业务命令只处理该数据，不用回读当前 index 猜目标。

现有 drain_action_records 只检查整个 Content mount，节点替换不更换根 mount。因此需最小原 Component hook：`action_payload_current(&self, payload: &ContentActionPayload) -> bool` 默认 true、纯只读且无副作用；drain 原队列时额外调用它，仅作旧队列排出的权限检查，不能生成/重排/重放动作，不新建队列。Tree 以当前 dataset/key/exposure token、逻辑可见/启用、窗口active/activity epoch及原 mount 校验；隐藏或移除前未 drain 的旧动作丢弃。intent 内 model revision 仅记录来源，不能要求等于当前revision：持续同key仅重排/插入/改label的旧意图仍指原实体，保持有效；排出历史事件不依赖新几何，未来pointer/AX执行仍须新arrange。已 drain 的记录是历史事实，不赋予未来执行权；延迟业务任务执行前通过同一纯查询 `TreeContent::intent_is_current(&TreeIntent)` 校验，不得只验 key。此项属于必要共享 glue，由主协调串行写，不能另建 Tree 专用并行动作队列。

## 4. 隐藏、替换和 ABA 的具体契约

每个根 mount 内分配单调、checked_add 的 usize exposure token 作为 SemanticPart.index；它不是 source index，也不是 key 的 hash。同一root mount的counter绝不因dataset切换、清空、隐藏后恢复、失活/恢复或Cancel归零；新root mount由原mount身份隔离。token→(dataset,key,当前index) 表保存当前逻辑节点；disabled行仍可读，但其旧可操作token撤销，为disabled只读结构分配新token，重新启用再分配新token。溢出拒绝更新，不wrap。same-key 稳定重排保留 token；可访问名称/纯装饰变化保留 token；隐藏、disabled、删除、dataset替换、卸载撤销原token，之后恢复必须取新token。祖先折叠即刻撤销后代，不能等下一帧render才失效。viewport离屏保留逻辑节点/token，语义沿本设计第5节保留零面积结构链，不复用旧frame。revision只控制几何新鲜度与记录来源，不代替exposure身份，不能因持续key重排而撤销其token。

| 更新/事件 | 模型与焦点 | 捕获/布局/语义/队列 |
| --- | --- | --- |
| 插入/兄弟重排 | 持续 key 的原选择/展开；原 focus 映射 | 取消原 capture/hover；重排旧曝光 token 保留；旧坐标在新 arrange 前拒绝输入 |
| 收起/隐藏后代 | 原最近可用祖先恢复；不焦点到其他树 | 撤销隐藏 token/旧动作，取消捕获，后代 slot inactive |
| 删除/同 index 换 key | 不把 index=0 的选择给新实体 | 旧 token 永久无效；新 key 新 token，新 layout 后才可命中 |
| A→B→A / disable→enable | A 新 incarnation 无旧 focus/capture；显式新操作才可选择 | 相同 key/name/frame 都不能复活旧 token、旧AX、旧未drain动作 |
| 完全 clip 离屏 | 原逻辑焦点可留；不制造新焦点 | 无指针/AX几何；祖先 reading/reveal 管理；滚轮原路由 |
| 卸载 | 原lifecycle清View焦点/capture；不结束邻居组字 | root mount作废，原Host丢弃未drain动作；重挂同ID不得复活 |
| 窗口inactive→active | 原blur清内部focus/capture/hover，选择数据可留；恢复不自动focus | inactive转换递增原TreeContent的activity epoch，未drain旧意图经原queue过滤永久拒绝，即使inactive期间未drain、active后才drain也不能复活；曝光token可保留，动作仍需active与新layout |
| Cancel / CancelPointerCapture | 只取消当前原gesture，保留已成立选择和键盘focus，不结束邻居组字 | 原capture serial失效；已由先前有效事件生成的选择/激活意图仍是历史事实并保留，Cancel本身不发意图；取消后迟到up不再产出动作 |

几何 invalidation 时 adapter 立即清 TreeView 当前发布布局与 capture，并标记 needs arrangement；不能在旧 frame 上用新模型接受 pointer/AX。原 Host get_mut/render 流程继续作为外层守卫，不假设其根 mount 能辨识节点换代。

## 5. 一个布局 owner，原输入与主题

从原 TreeView::render 提取共享 measure_rows/arrange_rows 与 row chrome paint，不在 adapter 抄行几何。建议提供原 `TreeRowMetrics { content: Size }` 与 `TreeRowLayout { source_index, row, content, disclosure, visible, depth }`；默认字符串 render 调用同一 helper，自定义槽只回传测量尺寸。原34pt最小高度、20pt缩进、按钮inset、分支slot和row_gap不变；行高容纳被动内容加原button垂直inset。宽度先扣缩进/inset/分支slot，再 measure；0宽、有限窄高、非有限输入按原 clean_size/clean_rect。

TreeContent::measure 和 arrange 使用同一 metrics/helper；arrange 放置行槽且发布原 View 布局；paint 消费这份布局绘制原 Quiet chrome、选中/hover/focus，再绘制原槽与原 render_icons。不可在 measure 用 label 高度而 paint 用自定义内容高度；不可 render 第二次改 hit/AX frame。分支 icon 必须经原图标服务（requires_icon_services + WithIconRenderer）；没有服务按现有 MissingIconServices 失败，不能 consumer 自画三角。

row.visible = 原行frame∩Tree分配viewport∩祖先clip；child content、pointer_regions、semantic frame 用该同一结果。焦点墨迹沿原 paint_collection_focus 的 outset 策略，真实祖先clip仍权威，不能用行 self-clip 剪坏原焦点。语义发布所有逻辑展开且已arrange的行，以维持父子闭包：可见行frame取row.visible，完全clip的行frame为原行原点的零面积Rect，focused=false，仅作为只读结构节点；折叠后代完全不发布。原SemanticPart没有visible字段，不能伪造离屏行的可点击frame。Focus/Press/SetExpanded都要求该行当前row.visible非空；离屏内部键盘导航仍沿原View，外层可对行wrapper ContentId使用现有reveal，不在本批承诺完整AX离屏遍历。候选B定向测试此限制，能力账本如实记录。

输入原样映射到 TreeView：primary pointer_move/down/up、ControlKey、Cancel、WindowActive/生命周期；普通二级指针不能触发主选择/展开。Tree 是单一 Tab stop，内部方向键由原 Tree::key/View 管理；Tab 退出原 Host；Space 保持原 View 的 handled 行为，不擅自新增激活。pointer_up 分支整行切换保持原行为，disabled/未挂载/失活拒绝。

首批 Tree 不有 editor。邻居 Composer marked 时：AX Focus/Press 与 pointer/Tab 走原 ContentHost transfer barrier；新增 SetExpanded 同样纳入 barrier（该动作可能恢复内部 focus，不得绕过原组字 owner）。后台replace_nodes/纯Theme更新不调用 focus、不提交/取消邻居preedit；同树编辑子树在验证阶段已拒绝。保留 key-start/end 原一次解释，不手工模拟 marked。

主题采用现有 Content::set_surface(Some/None)、原 ContentHost::render 的 Palette 输入与 Component::child_theme 传递；Tree没有新默认Surface。原 View 接受继承 surface 的最小扩展只影响其原 Quiet palette派生与slot前景，行选中/禁用颜色仍由原 control_state/Palette/Surface 解析，不能复制色表或新增 Theme schema。显式槽Surface建立原边界、clear恢复继承；被动行显示原语义foreground/disabled ink。#71新Theme闭包完成前以945固定主题作候选验证；不把正在变化的live主题复制成一致闭包。

## 6. Tree/TreeItem 语义及有限接收

新增 `ContentRole::Tree`、`TreeItem { level: usize }`（与原平台level约定对齐，根行以原 Gallery 实际约定测试固定），`AccessibilityAction::SetExpanded(bool)`。Tree owner是唯一焦点容器；每个当前行为虚拟 SemanticPart：index=exposure token，name=node.label，selected=原model.selected，disabled=原model/view状态，expanded仅分支Some，focused=原View.focused_index。semantic_part_parent 返回当前父 token，不能返回 parent 的 source index。对外角色不是 List/Group 降级。

Focus明确转给原 View 当前行；Press调用原 select/Activated，绝不解释为展开；SetExpanded调用原 View::set_expanded，幂等、叶子、隐藏、禁用、失活、旧token拒绝。ContentHost原action transfer barrier包含新增disclosure。数据更新后、下一layout前拒绝目标动作，不以旧部分frame授权。

本地查询必须纯只读：反复调用 semantic_parts、semantic_part_parent、semantic_mount、selected_key、intent_is_current 不改变revision/token/focus/capture/marked/动作队列，不触发layout/paint。遇到过期geometry返回不可用，由后续显式原render恢复。零面积离屏结构节点保留完整parent token链与原label/disabled/expanded读取，但focused=false且不提供Focus/Press/SetExpanded操作；不能通过父节点的有效token授权离屏子节点。B必须实际交付这一本地Tree/TreeItem/SetExpanded映射及marked屏障，不能用“C稍后补语义”宣称B完成。

最小接收需要三处同步：controls角色/动作；app-protocol content/accessibility wire角色/动作与合法性；module-runtime/native_content/ProcessContainer及app-host数据映射。平台macOS已有Tree/TreeItem/SetExpanded，不重造原生控件。旧peer对新enum拒绝并报告unsupported，不替换为Group、Press或假称旧远端已兼容。首批接收只做数据/原owner分派与负例，不扩远端数据源、任意remote tree或#48全能力；若接收未随候选完成，则能力账本必须明确 local_only，远端拒绝而非成功空白。

C的wire接收必须双向闭合：child原owner→wire snapshot→ProcessContainer本地Tree/TreeItem/parent/token/零面积结构语义；host SetExpanded(bool)/Focus/Press→wire请求→child原owner并读回真实结果。测试保留旧peer未知Tree角色与未知SetExpanded请求拒绝、非法level/孤儿parent/token复用拒绝和失活/旧mount/旧exposure拒绝；不能只补一个producer枚举或单向serde往返便认定完成。查询方向仍不得派动作；SetExpanded不能降级Press。

## 7. 原契约检验计划与必要反例

以下是待实施验证，不能从本阶段红例推导通过：

1. 原 Tree/TreeView 定向回归：legacy add/导航/Enter/整行点击；同label不同key；重复/环/非法parent/非preorder原子拒绝；持续key重排后selected/expanded/focus正确；所有collapse路径归祖先；disabled、零行与失活守卫。
2. 独立公开 composition-consumer：普通Text→Tree→Text直接ContentHost；Text+Icon被动行、窄宽/多行高度；只用公开API；原字符串模式与新默认槽row/frame/icon/focus基准相等；无私有Gallery导入。公开API缺失红例先保留，再单独新绿证据，不覆盖原日志。
3. 身份反例：down(A@0)→删除A/B占0→up不激活B；down(A)→reorder→up取消旧gesture；AXtokenA→collapse→expand同A后旧token拒绝；removeA→insertA旧token拒绝；same-key label改名保持实体；dataset换代同key拒绝旧动作；未drain动作后隐藏/替换被丢弃；已drain意图延迟校验拒绝；窗口恢复无历史capture/focus复活。
4. 原内容/组字：嵌套Button/TextField/交互自定义Component递归诊断；fixed slots拒绝Host任意改序；邻居Composer合成Preedit中后台更新/Theme/折叠，不改marked binding/selection/focus/发送门禁，AXdisclosure遵守barrier。只是公共契约层合成输入，物理IME单列未验。
5. 几何与语义：measure→arrange→paint共享frame；窄/零/部分clip、已离屏祖先的结构父；disabled branch叶子disclosure守卫；旧坐标更新后未arrange拒绝；AXFocus与selected分别验证；原文本/图标服务缺失有明确错误。wheel交原外层reader、不双滚动。
6. 必要GPU：默认文本Tree对原View逐ROI比较，新增被动Text+Icon行检查对齐、分支slot与focus，浅/深/增强对比度×1x/2x及窄宽/祖先clip；固化准确源码、程序、原图与失败负例。冻结945可复现原Theme；#71若引入新Surface用途则由其owner另收新闭包，不重绑本批。GPU不代替交互/原生/物理IME/VO/性能。

所有cargo验证遵循原 scripts/tekesui_build.py build_lock/environment、cargo +1.95.0 --locked --offline、原 target/review-cache；不prepare_cache/clean/newtarget/安装/复制target。锁被占用则记录未执行并等主协调窗口，不能抢锁。只跑本批必要定向组，之后#8/#12/#14当前源接收由主协调安排。

## 8. 建议拆成三个候选执行 Issue（未创建）

| 候选 | 精确交付及原验收 | 依赖与独占文件 |
| --- | --- | --- |
| A：原Tree稳定模型与View统一布局/收起 | replace_keyed原子更新、key/index映射、原View自动revision守卫/同一measure-arrange/chrome、全路径collapse及disabled守卫；只跑第1组和原View几何负例；不新增Content实现 | A接收后才开始B。独占 controls/src/extended.rs（仅Tree区）、collections_view.rs（仅Tree区及必要原helper）、controls/tests/tree_identity.rs；不改List/DataTable，#67/#68需另约共享文件写入窗口。与Progress #72共用extended.rs时只交Tree hunk，由主协调整合；禁止全文件覆盖 |
| B：本地公共TreeContent与语义/身份 | 新tree.rs、跨model/display/ID/token原子事务、TreeIntent原队列检查、exposure ABA、原Host挂载与独立consumer；真实本地Tree/TreeItem/SetExpanded及marked barrier、无副作用查询；第2–5组；本地语义未完成不得宣称可用 | 依赖已接收A。独占 controls/src/composition/tree.rs、controls/tests/tree_content.rs、consumer/src/tree_content.rs、consumer/tests/tree_content.rs。actions.rs、composition.rs角色/导出/trait/barrier/Host事务、controls/lib.rs及consumer/lib.rs是主协调串行glue，禁止与Progress/Theme并行写；app-host/src/accessibility.rs与其定向测试是数据投影接收窗口，不改变窗口实现 |
| C：必要接收与证据闭合 | wire Tree角色/SetExpanded最小映射与旧peer拒绝负例；实际GPU第6组；向#55/#8/#12/#14提交有限证据候选，未验维度明确false | 依赖B及确定Theme冻结。独占 app-protocol/src/component/content/accessibility.rs、module-runtime/src/content_accessibility.rs、必要native_content/ProcessContainer映射与定向测试、design/reviews/tree-public-content-implementation-20261024/。若这些文件已有writer由主协调拆串行接收窗口。global metadata/capability/coverage及矩阵只能主协调最终接收写，C仅提交diff候选，不直接写 |

候选A不泛化成共享StableCollection框架；#67/#68未来复用的是“不以index/label为身份、exposure不可复用、原owner单一”的契约，是否共用类型另由其设计决定。阅读锚点/reveal/恢复仍归#22/#28/#42，不新建同义任务；Tree只通过原祖先事务报告几何，不要求consumer做offset补偿。

### 创建执行票时必须保留的精确验收

- **A1 身份/兼容**：legacy原测试保持；keyed add拒绝混用；直接写selected不能在keyed模式重定向选择；重复key/parent/preorder错误和counter溢出原子拒绝。
- **A2 自动守卫**：公开replace_keyed后不手动reconcile，直接pointer_up/key/paint/AX前守卫均拒绝旧geometry；model-aware重排新arrange后同key的选择/展开/focus正确；连续两个replace导致A→B→A仍拒绝原incarnation；无模型legacy入口不能授权keyed操作。
- **A3 原行为**：pointer与左右键收起统一恢复；Enter仍Activate、整行branch点击保持；disabled节点清selection/focus、disabled祖先不级联禁启用后代、全树disable恢复无旧focus；原字符串布局和焦点geometry不变。只读查询stale返回不可用且状态不变。
- **B1 公共入口/原子性**：原ContentHost和独立consumer真实挂Tree；Keep保留实例，非法交互槽、Tree内/Host其它分支重复ID、token容量不足分别失败且原模型/槽mount/计数/队列/capture全不变；涉及display ID变化只能走Host事务。
- **B2 身份/队列**：持续同key重排保token，旧intent的来源revision不同仍可排出；hide/disable/remove→恢复及dataset A→B→A/clear→恢复新token，旧AX/未drain意图拒绝；counter不重置。inactive后不drain再active才drain仍拒绝旧epoch；Cancel保有效历史意图但迟到up不产动作。action_payload_current默认true且纯过滤，无新队列。
- **B3 本地语义/屏障**：本地投影实际Tree/TreeItem/SetExpanded，parent链正确；叶子、禁用、零面积离屏结构节点无操作；重复查询不改focus/marked/token/队列；邻居合成Preedit中后台替换/Theme不改marked与selection，Focus/Press/SetExpanded遵守原transfer barrier。几何/指针/语义共享已arrange布局；不承诺完整离屏AX导航。
- **C1 双向接收**：真实child snapshot到host Tree语义与host动作到child原owner读回均通过；新enum旧peer显式拒绝；旧mount/token、孤儿parent、非法level、失活拒绝。B未接收前不启动C实现或GPU。
- **C2 GPU/证据**：固定接收后的源码与主题闭包运行第7节必要GPU，对原View参照及窄宽/clip保留真实失败和原图；metadata只交候选，主协调回填#8/#12/#14/#55。原生/物理IME/VO/性能/全Gold7未跑即明确未验。

A→B→C是实际实现与接收依赖的串行链，不能以同时开票代替先验收前置候选。主协调负责最终hunk整合、共享glue窗口与闭包身份接收；A不覆盖Progress72同文件其它段，B不把局部语义留给C补齐。

## 9. 主协调审查点与当前未验

主协调已接收advisor方向，按其反馈补清：自动revision守卫及legacy边界、跨模型/所有display/ID/token的原子提交、counter不重置与revision不误杀持续key、disabled/inactive/Cancel规则、本地语义与双向wire精确验收。最终设计候选交主协调复核后统一创建A/B/C；不向最终用户新增批准流程。本阶段不创建Issue/会话、不调用advisor；主协调按已授权流程审查与分派。

设计阶段实际完成：源码/945身份核验、精确缺API编译红例、设计与有限拆分。随后 A（#73）交付原 Tree keyed 模型与 TreeView 自动身份守卫的隔离候选，证据见 `design/reviews/tree-keyed-model-20261024/README.md`：16 项新契约、10 项 Tree 相关单元测试（含3项真实 counter 极值负例）、21 项原集合回归及定向 strict 通过；原 API 与 pointer collapse 行为红例独立保留。A 尚待主协调接收，未改 live 生产源；adapter/B 与 wire/C 尚未实施，未来公共 Content contract/GPU 均未执行。原生窗口、物理IME、VoiceOver、性能、完整Gold7、公开不可变发布、第二产品真实接入均未验证；未提交/推送/PR/发布/关单。证据目录内QA不是SDK实现，也不是独立产品consumer验收。

## B共享接缝裁定记录

候选B承担collections_view.rs仅TreeView纯measure/arrange/row chrome拆分，原字符串render复用内部helper；不改原Tree/List/DataTable/Progress。Component新增默认no-op window_active_changed观察hook，Content递归传播到所有保留mounted后代（含逻辑inactive）；原Host实际活动转换时通知，原Popover owner/close/cancel保留，insert/新mount同步当前活动状态。动作过滤仍原队列，无并行队列。仅隔离候选，等待主协调逐hunk接收；本地语义不代表wire/GPU/native/物理IME/VO通过。

B本地编译接缝补充：controls新增SetExpanded后，module-runtime/container.rs原穷举产生真实E0004。主协调批准仅加SetExpanded=>ContentResponse::default明确未处理拒绝arm，使实际app-host本地投影可编译；不发wire、不映射Press。原content_accessibility角色unsupported拒绝保留。该临时local-only拒绝由C#75双向实现替换，B不代表远端支持。

TreeIntent来源绑定补充裁定：原payload包含私有origin:Arc<()>，直接保留原Tree::model_identity；消费者从payload接收/clone，不提供可伪造origin构造器。intent_is_current纯查询额外ptr_eq，PartialEq显式比较origin指针及公开字段，不能Arc<()>按unit值比较。此绑定拒绝跨独立TreeContent根实例同dataset/key/token/epoch的旧延迟意图；same实例卸载重挂仍靠原lifecycle撤销exposure、单调counter与原queue mount过滤，不靠origin复活。原反例origin-red.log真实失败已保留，后续精确绿例另记。

本地TreeItem level固定为原Gallery实际约定：root depth=0，child=1（原gallery/workbench/accessibility.rs:288/2132）。候选初版错用root1，level-red保留真实反例，最终改回原depth约定。不得据初版local projection绿例把level1说成原平台兼容。

原native selector最小接缝裁定：只对零面积TreeItem的mutation selector不advertise；保原enabled/disabled/expanded等读取值，正面积沿原selector规则，其他role无新增几何规则。平台私有tree_item_mutation_has_geometry策略供原native selector与纯测试共用，platform-selector-red保实际旧策略反例。该纯策略测试及本地dispatch负例不构成真实GUI/VoiceOver验收。

原被动槽能力补充裁定：SelectableListContent仅增加interactive()->true声明，交互owner身份不依赖count空态；原focusable仍count>0、原model/View选择行为不改。普通ListContent保持非交互，可组成被动行。空SelectableList嵌入的真实emptylist-red反例已保留；此单一adapters.rs hunk纳入B共享窗口，禁止负向类型downcast/白名单。

本候选最终基线：主协调已接收#84 DataTable A与#83 Theme rules1.8.0/schema1.3.0；1453输入的精确combined baseline见tree-content-implementation-20261024/rebase-receipt.json。旧日志/源/程序保留各自身份；本Tree-only hunk不覆盖DataTable/Theme。五个无关设计文档仅作为只读输入冻结，不交B补丁。

有限结构守卫（主协调/advisor修订）：fixed_slots只约束Host入口，调用方仍可通过Component::children_mut破坏Vec。TreeContent私有登记key→(caller row_id, 原RowSlot实例Arc marker)，与可变rows分离；只核对原声明/实例身份，不复制业务选择模型、不用global registry。所有可拒绝入口先验证行数=原model节点数、RowSlot类型、每key登记ID/实例ptr_eq、展示槽0或1，然后检查passive与全ID。clear/pop/swap/push/同ID错误类型/另一个detached Tree同ID同类型wrapper偷换均返回InvalidStructure，查询不派动作或改变token，输入/measure/arrange关闭旧几何，不panic、不猜修复；非法结构的children查询/遍历为空，避免发布或路由伪造后代。新marker仅成功commit创建，同key Keep/DefaultLabel/Replace保原marker，真实lifecycle可更换Content.mount而不锁死marker，remount仍按原exposure/队列契约。RowSlot多展示槽同样拒绝。

原revision0输入f4aede115d997950d36f5e0e0b24c1cc57ff05a4ff0143722f58f2f046182626及源/日志/程序完整保留revisions/0-before-structural-guard，不把结构修订的新测试追认为旧测试。之后主协调独立集成Skeleton/#37属于不交叉根源delta，本B仍基于既有83/84冻结身份；主协调负责串行hunk合并与combined binding，不滚动重写本轮输入。

### 2026-10-31 #198 Collections-only 接纳来源 r4

本轮以不可变 `9725cdccc4609212587f5d88a1e5aa3cb0032e67` 的实际 Combo/Selection22 schema 为前置；仅接入原 List/Tree/DataTable 有限合同与 Tree2＋Table5 角色。原 SetExpanded、Combo/Selection、capture 与完整旧 decoder 保留。普通 Link 仅为原被动槽负例的实际类型依赖，复用 Nav owner 的模块/View/painter，不接导航主体。reading R2 与成功更新撤销旧 Host 输入凭证沿原 #198 r2；DataTable prepared decision 使用原模型/View owner。当前只完成隔离来源与静态适配，未编译或运行新三个负例、原 consumer/IPC/GPU；原证据按原输入身份复用，不能改签当前通过。详见 issue-198/collections22-minimal-20261031-r4/HANDOFF.json。

### 2026-10-31 r5 精确前置兼容

接纳包改以不可变 `6ec7a53dcf28bbefff866c4d80e43ffee6c7bbd4` 的实际 Nav/Search/Combo/Selection26 来源为前置，仅加原 Tree2＋Table5（26→33）。已准入的普通Link/module/View/painter与三个Nav Host修复保当前字节；原Link负例直接消费它们，不重装。old26完整decoder重新取该实际对象并记录Git blob/hash，不重命名old22；原R4/失败保持。Source-only、typecheck/三个新负例与原consumer/IPC/GPU均未运行，窗口仍由原域执行。

### 2026-10-31 #198 有限 Tree 交付证据追加（候选 r1）

本追加不改写上文历史接收及旧失败。真实 `tree_protocol` IPC 来自 `collections-gpu-ipc-window02/continuation01`，来源不可变 `cbe40461f9bb7c6b763be63eae16b6c5a767382f`，原程序 SHA256 `5b594191b4aa6741240b8c9c6f7ef607c1b52ff3ff7787de33f62579197928ab`，退出码 0 且结果字段为 `TREE_PROCESS_PASS`；进程快照覆盖 child owner、SetExpanded/Focus/Press 读回、零面积父链/拒绝和旧 mount/activity 拒绝。IPC 结果中的 `GPU_visual_acceptance:false` 是该协议程序自身字段，不能替代也不推翻独立 GPU 收据。

同一 source 的 Tree GPU 后续实际两 lane 均完成 readback：default 与 external 各 40 cases、240 PNG；合计 80 cases、480 PNG，报告 failures=[]、changed channels zero。480 个 PNG 哈希和 32 张 contact sheet 引用均校验；去重仅用于确切 SHA 类，所有 sheet 均有审阅记录。证据见 continuation `gpu-summary.json`、`all-visual-reviews.json` 和 `review/tree/png-review-manifest.json`。这些是 cbe source 的新 Tree lane；它们不重签早期失败。

List/Table GPU lane 在该续跑中仅沿用 `70ee52503eeb9e830268d40918f42f005aa1da3b` 的旧身份（default/external Tree 外的两域各自原报告），见 `external-reuse-verified.json`，不是 cbe 或 `089dc` 的新编译。普通候选 `089dc65d958d62900fc68ba6e7ce41f603f05200` 及设计 successor `853eed2f20792a2c1c64ae485b5aa82f8f650de5` 不因这些旧收据被改签。

该追加只同步有限 Tree IPC/GPU 证据与来源边界；不宣称 #66/#198 全部验收、公开发布、原生窗口/物理 IME/VoiceOver、性能或完整 Gold7 通过。
