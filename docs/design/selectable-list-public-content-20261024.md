> 2026-10-30 #198 恢复候选：下文原设计及历史接收身份保留。当前 4815c5a 兼容适配尚未编译；历史 pass 不等于当前主线/候选通过。公共交付由唯一集成批次完成。

主协调2026-10-24正式接收 amendment-1 设计。A可实施；B依赖A及#85实际reading接缝接收，C依赖B。下文候选来源保留时间身份，均不代表实现验收。

# #68 可选择列表自定义被动行与稳定身份：正式文档候选（amendment-1）

2026-10-24。按主协调已读全文并咨询advisor后的直接修订指令形成；方向已接收供A有限实施，本会话只交文档，A排期/建票由主协调负责；[#68](https://github.com/TekesApps/TekesUI/issues/68) 保持 OPEN。正确源 gallery-design-baseline/TekesUI HEAD `1a7614a273e1a897ddc8a7ad8cb41d303a4a1290` + WIP，23文件只读片段与SHA256见source-manifest.json；不是公开main、编译闭包或Theme冻结。无实现、编译、GPU、GUI、生产写入、全局矩阵修改、commit/push/PR/发布/清理，不建票、不委派、不联系旧会话/advisor。下列API全是候选，不是本会话已交付实现。原design-candidate.md与23个源片段保留为历史；本候选整合amendment-1，冲突以本版为准。

## 1. 已有能力、范围与两入口迁移

ListContent是已有异构纵向layout，children保留独立交互；SelectableListContent已包装原单选List/ListView，有焦点/事件、Quiet绘制、List/ListItem及选择动作。后者目前是字符串行、源索引身份。本票补自定义展示与业务身份，不新增选择引擎。原API及consumer见original-api-evidence.md与冻结副本。

首批是有限内存单选、完整快照、被动行Content、插删/重排/过滤/换dataset、原导航/取消、原reading/reveal、本地语义及现有List/ListItem Focus/Press端到端链路。多选、大规模virtualization、行编辑器、行内按钮/菜单/链接、拖放、分页/远端数据源明确后置，不承诺规模性能。

| 现入口/consumer | 兼容与迁移 |
| --- | --- |
| ListContent::new(name, Vec<Content>) | 不改字段/构造器/语义。Text/Button/Switch/TextField/Menu异构布局继续使用，孩子拥有交互。要整体单选才显式迁移被动行，并将交互控件移出；不自动给全部ListContent加选择。 |
| SelectableListContent::new(name, Vec<String>) | public model/view/labels/name、literal与new继续工作，旧字符串/源index路径保留，不承诺其直接字段写入跨替换身份安全。候选新增keyed关联构造器返回独立KeyedSelectableListContent，旧struct不加字段、不能隐式切模式；新类型直接实现原Component。 |
| settings同名兼容adapter | 仍是另一个真实类型，SettingsSidebar直接写selected并drain后回读index。保留旧用法；本轮仅迁移说明与可选diff：未来改controls公开keyed类型、业务路径→稳定key、select_key/clear_selection、typed intent。本轮不改Settings消费者，不将其迁移作为A/B验收门槛。 |

新增支持类型归已有tui.list，不另立组件/矩阵项。关联构造器只是迁移便利，也可直接new新类型。本轮不写public库存。

## 2. 模型层包装与有限API

```rust
pub struct ListDatasetKey(pub u64);
pub struct ListItemKey(pub u64);
pub struct KeyedListItem { pub key: ListItemKey, pub name: String }
pub struct KeyedListSnapshot {
    pub dataset: ListDatasetKey,
    pub items: Vec<KeyedListItem>,       // 原模型源存储顺序
    pub display_items: Vec<ListItemKey>, // 唯一有序子集，省略=隐藏
}
pub struct ListBudget { /* 显式非零模型/文本/Content/深度/parts/staging上限 */ }
pub struct KeyedList { /* private original: List + snapshot + identity metadata */ }
impl KeyedList {
    pub fn new(s: KeyedListSnapshot, budget: ListBudget) -> Result<Self, ListUpdateError>;
    pub fn original(&self) -> &List;
    pub fn selected_key(&self) -> Option<ListItemKey>;
    pub fn select_key(&mut self, key: ListItemKey) -> bool;
    pub fn clear_selection(&mut self) -> bool;
    pub fn replace_keyed(&mut self, s: KeyedListSnapshot) -> Result<ListReconcile, ListUpdateError>;
    pub fn item_key(&self, source_index: usize) -> Option<ListItemKey>;
    pub fn item_index(&self, key: ListItemKey) -> Option<usize>;
}
pub enum ListDisplayUpdate { Keep, DefaultLabel, Replace(Content) }
pub struct ListRowDisplay {
    pub key: ListItemKey,
    pub row_id: ComponentId, // caller-owned wrapper，与display根ID不同
    pub display: ListDisplayUpdate,
}
pub struct KeyedSelectableListContent { /* private KeyedList + original ListView + slots */ }
impl SelectableListContent {
    pub fn keyed(name: impl Into<String>, s: KeyedListSnapshot,
        rows: Vec<ListRowDisplay>, budget: ListBudget)
        -> Result<KeyedSelectableListContent, ListContentError>;
}
impl ContentHost {
    pub fn replace_keyed_list(&mut self, owner: ComponentId,
        s: KeyedListSnapshot, rows: Vec<ListRowDisplay>,
        reading: Option<ListReadingUpdate>) -> Result<(), ListContentError>;
}
```

KeyedList是**模型层**拥有唯一原List的包装，非adapter sidecar。原List两个public字段、derive、literal/解构、new/select/key保留，不添加必填字段。包装无mutable original、DerefMut、可写snapshot或可逃逸原可变模型的回调。唯一选择权威是内含原List.selected；selected_key由当前index→key导出，无第二份可写selected key。replace从旧原selected推导key，在staging映射目标index，最终原模型一次写count/selected。count=全部items.len()，不是显示数。

业务key来自caller，不用name/index/ordinal/ContentId/hash。item_index是本次源items位置；display ordinal是display_items位置。原CollectionItemLayout.id内部可保source index，SemanticPart.index是独立exposure token；三者分开。

原ListView keyed入口接收受控KeyedList，legacy/keyed共用同一private pointer/key/chrome/helper及原List::select，View仍唯一focus/hover/capture owner。记R=全部source items数，V=全部逻辑展示行数；layout按display_items构造V个items，每个item.id仍是对应source index，count=R。原ListView::key已经从layout.items取IDs，经row_key_target再调用原List.select，复用这条helper即可；无需新导航reducer，也不直接用List.key(count)。V包含viewport外仍逻辑展示的行，不能按clip删导航序列；退出V清其selection/focus。V=0且R>0时无focus/key/AX伪目标，不按count>0选source 0。Enter按原View选择并返回Activated；down变化选择、up同capture确认而无单独Activated；Space维持原语义。默认labels从snapshot生成，无独立可变labels/count漂移。

## 3. 身份、新鲜度与生命周期

同dataset且持续存在并展示的key保选择及原focus；插入/重排/改name不换实体。删除或退出display清其选择/focus，不选同ordinal邻行；恢复不复活旧选择/focus。换dataset清选择/focus。首批无per-row disabled字段，只用原View全列表disabled：清focus/capture/hover，可保有效选择数据，启用不自动focus。祖先inactive撤销授权；viewport clip仍逻辑挂载，保实例/选择/内部focus，不能当隐藏。

模型实例用独立Arc<InstanceIdentity>，View保Arc引用，旧引用活着时地址不可ABA；clone新Arc，整个包装replace为新实例。revision负责geometry，item incarnation负责删除/隐藏/dataset换代，exposure负责part授权，activity epoch负责活动换代，原root mount负责挂载。有返回错误的预检路径对所需计数checked，计数clear/dataset不归零；void lifecycle/失活通知不分配part token、不因耗尽panic。不globalcounter、不地址全局表、不magic ID段。两次View调用间hide/show、remove/reinsert、dataset A→B→A也必须识别，不能仅信末次Reconcile。

每个keyed mutating入口自动核对实例/revision/incarnation，映射持续key focus，取消capture/hover，清旧geometry；更新后新arrange前拒绝旧pointer/up/key/AX，不要求caller手动reconcile。选择变化不使geometry无故失效。无模型legacy layout/focus/pointer_move不得授权keyed操作，补model-aware入口共用helper；切回legacy render先清keyed授权。

layout_for/focused_index、semantic_parts/parent/mount、selected_key、intent_is_current必须纯只读。stale返回None/空，不暗中修复/reconcile/arrange/分配token/reveal/派动作。

同一Content对象unmount→remount也必须新原mount/授权；不能假设对象没换就可保旧AX/intent。window inactive只撤销有效part映射、capture/focus/hover并标记需新曝光，保root mount，不在void callback里分配token或因耗尽panic。恢复后在可失败预检路径按当前V为新的单调part token预留完整容量；失败保持不可操作，不发布部分映射、不回用旧token、不改root mount。成功显式arrange才发布新映射；不自动恢复旧focus/capture。祖先关闭/重开同样撤旧曝光后预检恢复。Cancel只取消held gesture与原capture serial，保已成立选择/历史动作，迟到up不产新事件。

## 4. 完整只读预检与一次commit

Host事务先对完整目标做只读staging，任何改动/reading迁移/lifecycle前完成：

1. 先核对当前结构登记的count/type/caller ID/私有slot owner identity/slotcount，坏结构InvalidStructure并关闭后续遍历/查询/输入/更新，不panic或猜修。items key唯一，display为唯一存在子集；rows记录覆盖全部items含隐藏。所有row wrapper及display后代ID在完整目标Host全局唯一，含Keep原树、隐藏inactive后代与其它分支。替换分支旧ID可释放，保留分支不能因没显示而漏查。
2. 全目标被动能力、原validate、Surface、fixed_slots结构合法。持续dataset/key的caller row_id在Keep、DefaultLabel、Replace下始终必须相同，包含隐藏后再展示；不符在提交前拒绝，不提供同key换row_id/wrapper替换功能。Keep保原display对象/mount；新项Keep=DefaultLabel。DefaultLabel撤销装饰，Replace只替换display，二者仍保原row wrapper ID与owner marker；换dataset的Keep不取回旧实例。显式换实体须remove后新增或new dataset，撤销旧授权；不能伪装同一持续key改row_id。
3. 显式有限非零budget只按一维items（含隐藏）、display映射、name UTF-8、wrapper/隐藏槽/全部后代与深度、part/token表及staging可验结构计数检查，checked所需累加和counter容量。不搬Table二维R*C或Tree拓扑预算；staging是确定性结构计数而非精确进程内存/OOM保证。与实际Host上限对齐；仅实际走transport时应用其现有限额，不使本地模型依赖transport。模型A与Content B各检查所拥有结构，不截断后假成功。错误指明key/slot/ComponentId/budget类别。
4. reading存在时先只读验证owner、expected version、目标revision、有序row IDs及原reading容量；预备View协调、geometry失效、生命周期通知，所有可失败验证/分配/用户回调在提交前结束。

通过后在**原KeyedList实例上一次commit，保原Arc identity**；不能clone包装再swap导致持续实体全换代。模型、槽顺序/保留实例/token、View失效、reading一起提交。Keep不能提前move/unmount；不能先replace_keyed再查非法display/Host/reading；提交后无返回Result的验证/容量分配/用户回调。对支持Component遵守原lifecycle契约的正常路径保证无fallible尾部，不伪称任意panic/OOM可rollback。

任一Result错误保持原count/selected、Arc/revision/incarnation/counter、View focus/capture/layout、queue/mount、槽实例、reading version/items/anchor全部不变。脱Host构造仅验自身，mount再验全Host。已挂载component_mut只允不改ID集合与reading有序项的受控命令，其它HostTransactionRequired；不能假设没有public mutable slots出口：Component::children_mut暴露的Vec仍可被clear/pop/swap/push或替换。fixed_slots只挡普通Host修改API，不替代原owner结构验证。

原reading先attach/advance再callback且明确不保证任意callback rollback（冻结content_reading.rs:477–532）。A可先行；B须等待DataTable #85实际reading事务接缝接收，再在主协调串行窗口复用实际差集，不能假称现有reading callback已可rollback。不可套一个可能失败的closure就声称原子。

## 5. 被动槽、一个布局owner、Theme和真实混合输入

caller给每行稳定wrapper ComponentId以及Replace(Content)全部ID；DefaultLabel无需库自动造公共ID。不能key算ID/globalcounter/预留magic段。持续dataset/key的row_id在全部display variant及隐藏/恢复中固定，wrapper对象/owner marker持续保留；Keep额外保display对象/mount，DefaultLabel/Replace只改变display，不换row wrapper。隐藏wrapper及display保留但inactive。display整棵suppress_semantics，行name明确snapshot.name，不从装饰拼AX名称。

递归能力检查interactive/focusable、text_surface/text_input_snapshot、阅读选择owner、composing、overlay_scope/open/presented、全部children与原validate。拒绝Button/Link/输入框、空SelectableList、ScrollContent/选择阅读、菜单/Tooltip交互owner、嵌套集合，定位row key与违规ComponentId。普通Text/Icon/Separator/被动Stack/Grid可用；按通用能力，不类型白名单、不只查focusable或NestedInteractive（List非原action_owner）。复用/补最小validate_display_slot；#74仅给原空SelectableListContent加interactive()->true，本轮不写其focus/count。动态能力变更发布前重检，非法关闭本次输入/绘制并诊断，不半挂载。自定义Component须遵守纯查询/生命周期契约，不承诺识别恶意隐藏状态。

继承已接收#74的结构防线：独立于可变Vec保存key→(caller row_id, 私有slot owner Arc identity)登记，按source顺序检查row数量与模型R/登记数量一致、每项具体私有slot类型、caller ID、Arc::ptr_eq原owner、display槽数符合DefaultLabel无自定义或Replace单个自定义的0/1不变量，再做递归被动/全ID检查。不能只检查ID或类型；另一detached owner同ID同类型wrapper偷换也拒绝。Keep/DefaultLabel/Replace均保同一row wrapper marker，合法mount生命周期不改变marker。

坏结构的validate/update返回InvalidStructure；children遍历返回安全空切片、children_mut返回None，语义/layout/freshness查询返回不可用，输入/动作/布局发布fail closed；不panic、不索引坏Vec、不把坏结构当空列表提交、不修改模型/token/queue/reading或自动修复。布局需要改变状态时只关闭操作与旧geometry，查询本身无副作用。须测试clear/pop/swap/push、同ID错类型替换、另一owner同ID同类型替换，以及slot内部多display数量破坏。不能由“fixed_slots=true”或Rust字段私有推导这些攻击不可能。

从原ListView提取共享measure/arrange/chrome，legacy字符串、keyed默认与自定义槽共用；槽只交测量尺寸，不复制View。保row_height=spacing/space-loose.max(24)+12、gap=4、Quiet Body Leading/UiBody/原inset，不抄Tree/Table的34pt。先扣原inset测content，行高取原最低值与content+垂直inset较大值；默认frame/hit/focus ROI须与原相同。measure/arrange同metrics，paint消费已arranged布局，不能二次render改cache。

row.visible=frame∩列表viewport∩祖先clip；display、pointer_regions、AX共用，focus保原outset与祖先clip，不额外selfclip削环。窄/零宽、多行、非有限约束按原clean/valid处理，拒绝伪命中。Text/Icon用原服务，缺服务原错误，consumer不补画。Theme用ContentTheme/Palette/Surface/child_theme继承、覆盖及clear；选中/禁用墨色原owner算，无色表复制或schema修改。必要Surface输入与Theme writer由主协调串行确定；QA不是live主题闭包。

单一Host Tab stop，内部方向/Home/End原View，Tab原Host；primary pointer/ControlKey解释一次，二级指针不选行。首批无editor，仍需真实混合consumer：普通Text→Scroll+keyed列表→真实Composer。合成Preedit期间后台更新/重排/Theme/卸载列表不改邻居selection、marked binding、focus与发送门禁，不提交/取消组字；pointer/Tab/AX Focus/Press沿原transfer barrier。物理IME单列未验，不绑#5整库native。

## 6. 原reading、AX及原动作队列

外层ScrollContent/ReadingController唯一滚动owner。display row的caller wrapper ID与真实arrange frame按display顺序交原reading transaction；Keep保实例/ID，隐藏删除从ordered items移除，旧锚点不转给同ordinal邻行，fallback沿原文档。换dataset/撤销恢复不复活旧document/reveal授权。显式reveal验expected version，不算consumer offset；wheel原路由。全部先预检再同一不可失败结构/reading提交，不新滚动引擎。

复用原ContentRole::List/ListItem、wire SemanticRole和macOS映射，不新增ListBox/Tree/Table。根List，行虚拟ListItem；selected来自原List，focused来自原View实际focus且Host焦点在此列表，不能以selected猜focus。part.index为mount内checked单调exposure token；token映射dataset/key/incarnation/source index/ordinal，不用index/name/hash/stride。平part parent=List根，无树/二维父链；原role无ordinal字段，首批不为ordinal新增wire字段。

**首批只发布正面积可见ListItem**：frame用visible交集，完全clip项不发布part但保逻辑身份/token/focus供原keyboard/reveal。重新可见且Host焦点/lifecycle仍有效才发布原focus，不主动focus/reveal；clip不换exposure，逻辑隐藏/disabled/卸载则换代。空/完全clip列表不造零frame可操作part；根完全clip沿原Host隐藏投影整体不发布，平列表没有父完全不可见但合法孩子可见的布局。B须实际controls→app-host content_nodes→platform validate/shared路径证明正面积/父闭包；共享桥接若仍产非法root，主协调窗口最小修正，不能借Tree/Table零frame策略放宽全部List。首批不承诺完整离屏AX遍历。

Focus/Press校验root mount、实例/token/exposure、dataset/key仍展示、当前geometry revision、正面积visible、window active/祖先active/enabled及当前有效part映射。Focus仅原View焦点不选择；Press保持原adapter选择并一次激活，其它写动作/SetExpanded拒绝。同key/frame/ComponentId也不能复活旧part；同实例重挂检验原mount换代。纯查询无分配/重排/派动作。

候选ListIntent payload沿原ContentActionRecord队列：dataset/key/incarnation/exposure/activity epoch、来源model revision、事件后selected_key、SelectionChanged或Activated。down/导航变化一次SelectionChanged，up不重复；Enter一次Activated，AX Press一次Activated，附selected_key，不再加第二Activate。paint/focus/幂等选择/后台replace/Cancel不造意图。旧legacy activated映射不变，新keyedtyped payload区分选择与激活，业务不把Changed全当Activate。

复用Tree/Table最小Component::action_payload_current默认true纯过滤；原drain mount校验后验证item曝光/activity，不新队列/过滤后重放/callback。持续key重排的历史意图仍有效，来源revision不同本身不丢历史；未来坐标动作仍需新geometry。hide/remove/disable/ancestor inactive/dataset ABA后未drain意图丢弃，window inactive后active才drain也拒绝旧epoch。已drain为历史，延迟执行须同一纯intent_is_current验证；Cancel保已成立历史，clone/整值替换/同实例重挂旧授权拒绝。

C只覆盖原#68必要GPU及**现有**List/ListItem Focus/Press端到端链路：child原owner→既有snapshot→现有receiver/host投影，host原Target Focus/Press→child原owner读回真实focus/selected key。mount与part原值贯通，不重新编号或按当前index重定向。保存旧Target→窗口失活→恢复成功新曝光→投递旧Target须定向拒绝；此时root mount不变、part必须新单调值，故无需wire activity字段。新Target只在最新geometry/可见/启用时有效。activity epoch仅负责业务intent freshness，不能代替part授权。

不新枚举、不加未知peer协商/版本化授权要求，不以全面远端验收或任意远端业务数据源扩大#68。已有schema/错误路径继续使用；必要负例限本次目标旧mount/旧part、错目标及原值贯通。serde往返不能代替上述真实owner结果，原生VO/物理IME/GUI/性能/#5也不成为本票前置。

## 7. 五项对应验收与A/B/C最小拆票建议

| #68原项 | 精确有限验收，均待实施 |
| --- | --- |
| 1 key/槽/单选owner | 真实独立consumer编译原List literal/解构/new；KeyedList mutable原模型/字段/DerefMut/逃逸闭包负例；同名异key/同key改名，key↔source index↔ordinal；Text+Icon/被动Stack，自定义单选且多选不交。 |
| 2 原导航/诊断 | legacy默认frame/选择/focus/动作回归；同一reducer按display导航；down一次变化/up不重复/Enter与AX Press各一次激活；空SelectableList与伪被动wrapper内Button/TextField/Scroll/阅读owner递归拒绝定位ID，不改#74focus/count。 |
| 3 生命周期 | down(A@0)→reorder/remove/B占0→up不指B；过滤删除恢复/dataset ABA/两次View调用间多replace/clone/整个包装replace/同实例重挂/window与ancestor失活恢复；旧AX/队列/已drain延迟任务拒绝；更新未arrange拒绝旧geometry/key，纯stale查询状态不变。 |
| 4 Theme/clip/reading/IME | 单一measure/arrange/paint/hit/part geometry，长文本/0与窄宽/部分完全clip/focus；Surface继承覆盖clear/disabled墨色；caller ID原reveal，Keep原mount，隐藏删除锚点不转邻居；真实Composer合成Preedit下更新不改marked/selection，Focus/Press/Tab/pointer走barrier。 |
| 5 consumer/GPU/证据 | 公开consumer直接Host无Gallery私有导入；本地实际platform校验链及现有List/ListItem Focus/Press端到端结果；固定接收源码/Theme后必要GPU对原List默认路径、新默认与Text+Icon/多行/窄宽/选择/focus/祖先clip留原图和失败。#8/#14/#55仅交diff证据候选由主协调写，native窗口/物理IME/VO/性能/发布单列未验。 |

错误测试必须断言原模型/Arc/选择、View focus/capture/layout、counter/queue/mount、槽对象与reading逐项不变。持续key跨三种display变体/隐藏恢复改row_id均拒绝；坏结构全路径fail closed。完整Host其它分支/隐藏Keep后代/wrapper重复ID、非法槽、一维budget/深度/UTF-8/真实counter极值/reading expected version均在commit前拒绝；不能只查新Replace树。极值故障注入击中同一生产路径。

A→B→C建议由主协调查重后决定是否建票，本轮不创建。平List不复制Tree拓扑/展开或Table二维/header/sort/accessory复杂度；兼容模型、Host事务与必要端到端/GPU有独立验收边界。A可先行，B等A与#85实际事务接缝接收，C等B接收；三段仅为主协调去重建票/排期草案。

| 候选 | 有限交付与必要验收 | 依赖/独占及共享窗口 |
| --- | --- | --- |
| A 模型KeyedList/原View | 原public List兼容、单一reducer、R/V/source index导航与一维预算/Arc/原实例commit、自动freshness、纯focus/layout、旧输入拒绝、默认几何和compile负例，不做Content。 | 可先行，复用#73/#84契约不抽StableCollection。新增controls/tests/list_identity.rs、consumer/tests/list_legacy_literal.rs及QA compile负例；extended.rs只List新增包装/helper，collections_view.rs只List与必要helper，主协调串行hunk，不碰Tree/Table/Theme/全文件覆盖。 |
| B keyed Content/本地事务reading/AX | 完整Keep/隐藏/Host ID与预算预检、保Arc一次commit，caller wrapper/reveal、通用passive/typed原queue、真实本地platform链、混合Composer、本地Focus/Press纯查询与公开consumer。 | 依赖A与DataTable #85实际reading事务接缝接收。独占composition/selectable_list.rs、controls/tests/selectable_list_content.rs、consumer/src及tests/selectable_list_content.rs；actions/composition/reading staging/导出/库存/app-host共享glue主协调串行，复用#74结构守卫与#85实际接收hook差集，不新Host。Settings仅迁移说明/可选diff，不改消费者、不作A/B门槛。本地AX/reading/原子性未交不能推C后称B完成。 |
| C 现有链路/必要GPU | 现有List/ListItem Focus/Press端到端真实结果、mount+part原值贯通、保存旧Target→失活恢复新曝光→旧Target拒绝；固定源码Theme必要GPU，账本证据diff候选。 | B接收后。仅在现有accessibility/receiver/owner链路实际需要处主协调登记串行差集；独占定向链路/GPU测试及后续QA目录。无新枚举/未知peer协商/全面远端门槛，#8/#14/#55/global metadata主协调接收，不扩大#5。 |

实施才用原scripts/tekesui_build.py build_lock/environment、cargo +1.95.0 --locked --offline、既有target/review-cache；锁占用记未跑等待窗口，不prepare/clean/install/newtarget/复制target。本轮遵禁令未执行cargo/GPU。

## 8. 主协调接收点与当前证据

主协调已固定模型KeyedList、Content KeyedSelectableListContent；旧两入口兼容；持续key全display变体固定caller row_id；结构守卫沿#74；活动恢复只换part授权不改root mount/wire。R/count=全部source，V=全部逻辑展示，复用原ListView::key(layout IDs→row_key_target→List.select)。A可先行，B等#85事务接缝实际接收，Settings本轮不改。精确A/B/C验收草案见acceptance-draft-amendment-1.md，由主协调去重建票排期，本会话不自行创建或实现。

本轮完成Issue原文与comments读取（当时0）、23文件只读manifest、原API/consumer证据、候选与验收/拆分建议。无实现/编译红绿/GPU/GUI/物理IME/VO/性能/公开发布；Tree/Table来源README记述未复跑，不计本票验证。只写本QA目录，无PR；本会话按指令只交append设计；A的授权方向不构成本会话实施指令。


## 主协调执行登记

有限执行票已去重创建：A #94（KeyedList/原View，可先行）→ B #95（依赖#94与#85实际reading接缝接收）→ C #96（现有链路及必要GPU）。原#68保留总体原验收，拆票不代表父票完成。


## A main acceptance (Issue94, 2026-10-24)

Four List-only source/test hunks integrated, preserving Tree/Table changes. Nine public support types registered under tui.list; canonical type binding count321. Root receipt design/reviews/list-keyed-model-20261024/root-intake.json SHA256 909ae0e13b5876a82c3c613d1dcb11c8389aaacc2f885d0191dc5b187daeddbe. Stable current-source inputs1572: List12+legacy21, counter4, registry3, independent literal consumer1, controls lib/List test and consumer strict passed. Candidate public8 expected compile negatives and384 deterministic original layout/Scene comparisons retain their own source/program identities; no GPU claim. No new selection reducer, generic StableCollection, Content, reading or wire delivered by A.

First root registry assertion exposed main classification ordering, corrected by sorting only. First source stability false was one Issue9 design documentation update made by main during verification; second full window stable. Retain both attempts and candidate cache-failure disclosure. #95 still depends on #85 accepted actual reading transaction plus this A; #96 waits B. Native/physical IME/VoiceOver/performance/public release remain unverified, not extra A gates. PR null.


## B main acceptance (Issue95, 2026-10-24)

Main integrated15 necessary source/test paths over current #101/#106, preserving SearchInput and sameAtomic test fixture isolation. Generic immutableindex/targetexclusion helpers are moved from Table without a new allocator or transactionengine. Passive KeyedSelectableListContent keeps original KeyedList/ListView/reducer, callerwrapper/privateArc identity, borrowed refused inputs, fullHost/hidden budgets and staged originalreader update, fullV activitytoken recovery and typed originalqueue intents. Advisor found no new localB blocker.

Main11lanes passed, including defaultparallel controls/lib and current List/Table/Tree, independent mixededitor/reading consumers, AppHost actuallocalprojection, strict, Gallery andSearch101union. Exact counts/program/depinfoSHA in design/reviews/list95-main-intake-20261024/root-acceptance.json and root-lane-verification.json. Currentroot384 layout/Scene cases exactly equal frozen originalView, notGPU. New7publictypes classified, total342; existing canonical bindingfixture335/source observations nowstale andqueued for normal#8 refresh, not re-signed or claimed currentall342compilerproof. Independent public consumer directly compiles new API.

List faulttests no longer write sharedglobalMAX: reuse #106 scopedAtomic with identical allocator operations. Test-onlyTLSmacro moved inside privatecfgtestmodule to satisfy unchanged strict publicsurfaceindex; final defaultparallel+strict rerun. No guardchange/ignoredtests/privatecolors. Native/physicalIME/VO/performance/publicrelease and#96 wire/GPU remain unverified; none added to original#95eight criteria. NoPR/commit/push. Parent#68 and#96 remainOPEN.

### 2026-10-31 #198 Collections-only 接纳来源 r4

本轮以不可变 `9725cdccc4609212587f5d88a1e5aa3cb0032e67` 的实际 Combo/Selection22 schema 为前置；仅接入原 List/Tree/DataTable 有限合同与 Tree2＋Table5 角色。原 SetExpanded、Combo/Selection、capture 与完整旧 decoder 保留。普通 Link 仅为原被动槽负例的实际类型依赖，复用 Nav owner 的模块/View/painter，不接导航主体。reading R2 与成功更新撤销旧 Host 输入凭证沿原 #198 r2；DataTable prepared decision 使用原模型/View owner。当前只完成隔离来源与静态适配，未编译或运行新三个负例、原 consumer/IPC/GPU；原证据按原输入身份复用，不能改签当前通过。详见 issue-198/collections22-minimal-20261031-r4/HANDOFF.json。

### 2026-10-31 r5 精确前置兼容

接纳包改以不可变 `6ec7a53dcf28bbefff866c4d80e43ffee6c7bbd4` 的实际 Nav/Search/Combo/Selection26 来源为前置，仅加原 Tree2＋Table5（26→33）。已准入的普通Link/module/View/painter与三个Nav Host修复保当前字节；原Link负例直接消费它们，不重装。old26完整decoder重新取该实际对象并记录Git blob/hash，不重命名old22；原R4/失败保持。Source-only、typecheck/三个新负例与原consumer/IPC/GPU均未运行，窗口仍由原域执行。

### 2026-10-31 #198 有限 List 交付证据追加（候选 r1）

本追加仅补登记后续有限证据，不覆盖上文各自的来源、失败或接收身份。List 的独立协议证据来自 `collections-gpu-ipc-window02/continuation01`：真实 `list_protocol` 进程以原留存程序 SHA256 `838a179a2ff7a9d29288861a395d61c8911c7dc7e4963514706ccf71cda797d1` 返回 `LIST_PROCESS_PASS`，实际 parent/child PID 不同；见该目录 `execution-receipt.json` 与 `HANDOFF.json`。此 IPC 来源为不可变 `cbe40461f9bb7c6b763be63eae16b6c5a767382f`，不是当前普通候选 `089dc65d958d62900fc68ba6e7ce41f603f05200` 的新运行。

同一 continuation 只复用原 GPUIPC01 的 List GPU lane：来源 `70ee52503eeb9e830268d40918f42f005aa1da3b`；default/external 各 96 cases，原 PNG 身份和源程序身份保留，见 `collections198-integrator-gpuipc01-primary-integrity.json` 与 continuation `external-reuse-verified.json`。这些既有输入一致性证明不是对 `089dc` 的重新编译或重新验收。

因此本次只增记有限 List evidence；不据此声称 #68/#198 全部验收、完整当前源联合、原生/物理 IME/VoiceOver、性能、CI、PR 或发布通过。#198 和父设计要求仍由统一交付批次结案。
