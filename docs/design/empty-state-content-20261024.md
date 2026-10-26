> 当前本地 main 接收（2026-10-27）：基线 `b4fbed1` 加精确差量已验证原 EmptyState View/动作 owner、固定代理、Surface、非法结构/容量原子拒绝、隐藏/失活恢复授权、混合 Composer/reading、公开语义协议及严格检查；默认/外部 Theme 实际 GPU 与原 Gallery body 接入通过。必要 #74 共享过滤单独记账，不代表完整 Tree 接收。证据和失败：SSD `artifacts/issue-136/empty78-mainb4fbed1-intake/root-review.json`。原生窗口、物理 IME、VoiceOver、性能、wire bridge 和发布仍未验。以下保留原设计及阶段记录的历史身份。

主协调2026-10-24正式接收本设计；实施、测试、GPU和原生验收仍待独立交付。以下候选来源与未实施标记保留其时间身份；所有最终接缝按本设计执行。

# #78 EmptyStateContent 最终设计候选

2026-10-24。关联[#78](https://github.com/TekesApps/TekesUI/issues/78)，父#56、视觉#55。基于主协调全部最终裁决整理，提交正式设计接收；**仅设计，未实施/构建/测试/GPU/GUI**。正确源/65输入hash见同目录source-manifest.json；HEAD1a7614a273e1a897ddc8a7ad8cb41d303a4a1290+WIP。原candidate、amendment-1不改，amendment-2.md列精确delta。#74 hook本轮已在live源确认，不再当缺失依赖。

## 原实现与有限API

extended::EmptyState原本已有title/detail/action:Option<String>，EmptyStateView已有唯一ButtonModel、同源measure/render/clip、disabled/inactive、pointer/key/focus/取消路径。已有Gallery和独立consumer直接用View，本票只补公共Content接线，不造业务reducer或任意交互slot。

候选签名（未编译）：EmptyStateContent::new(accessible_name, model:EmptyState, action_id:ComponentId)->Result；model()->&EmptyState，layout()->ViewLayout，name/description普通元数据fallible更新。动作语义更新经ContentHost有限typed入口，如replace_empty_state(root_id,model)、set_empty_state_disabled(root_id,bool)，不能从component_mut直接写原model/action。accessible_name非空，原title/detail可空以保empty fixture，Some(action)全空白拒绝；不塞默认产品文案。

action None↔Some、Some label替换、禁用/隐藏后的恢复更新proxy mount；只改title/detail而动作身份/含义未变可保留原授权。原action=None时proxy inactive、不获焦且无AX Button；Some时一个原Button。所有更新先预检后取消旧arm再提交；不要因label改动重建整个EmptyState或Composer。

## 原几何与Surface

frame是父Surface内分配好的内容槽；本体不自画背景，不把Gallery父space-loose padding写进adapter。原算法保留：x+4、title y+6、width−8；标题UiHeading/min34/max420；detail y=max(44,title.bottom+4)、min54/max460；action最大宽150/min34、y=max(detail.bottom,100)。受限高度可clip但动作不能覆盖detail，Button默认居中；量度/prepare/paint共享原服务，不让adapter复算这些数字。

七Surface为canvas/code/hint/panel/popover/sidebar/toolbar；四mode为light/dark/light-ic/dark-ic。ContentTheme.palette+effective inherited Surface来自原Host，显式set_surface/clear沿原继承，adapter不保存另一份Theme，不接RGB：

| 部位 | 完整包 | 七通道 |
|---|---|---|
| title/detail | content/rest | background=$surface仅描述父底、text/icon=primary-text；boundary/focus/marker/helper=null；本阶段只读取text |
| 原动作 | action-primary，状态由原ButtonModel | background/text/icon/boundary/focus完整沿既有rest/hover/pressed/disabled包；marker/helper=null；focus-visible独立 |
| 父padding/背景 | 原父容器 | adapter不重画，不猜背景 |

body rest-only；disabled只影响原action，不把说明变成disabled文案。原View补effective Surface入口、body读既有content包text、原ButtonModel接同Surface完整包；原直接render默认沿原约定兼容。不拿父decorative foreground覆盖按钮专用ink。已有content/action用途足够，**不新增Theme用途、颜色或#83政策**。一般text/icon≥4.5、focus/parent≥3沿当前守卫，不误称增强文本≥7已由全部既有包保证。

## 实施ownership与最小交付

**#78独立先实施实际EmptyStateContent及共享proxy授权最小helper，全部归原票。** 不给未使用的helper单开票；#79后续复用#78接收的授权hunk，并由其同一writer交runnable/clock与真实Toast/Banner adapter，不另开空clock票。#79正式Theme marker依赖为#93，不阻塞本#78。原反馈View补prepare/paint_prepared、原按钮routed activation/cancel/Surface；新增composition/empty_state.rs；composition.rs模块导出及有限Host glue、actions.rs原mount分配fallible底层为主协调串行窗口。原Gallery feedback_adapter、独立consumer/定向tests一起交hunk。无Kernel/ClientKit/Agent UI迁移。

公开API库存/Workbench/设计/父票仅在实现及必要验证后由主协调同步。shared feedback_view/extended/composition/actions等不与Progress/Tree/Panel/Badge并写，不覆盖全文件。Issue OPEN；本阶段不commit/push/PR/发布/清理。

## 共用：原owner、固定动作代理与同一几何

root为被动Group，持原模型与原View的唯一owner，内含一个固定私有action proxy Content；局部共享引用只为访问原owner，绝不克隆ButtonModel、可见性、焦点或剩余时间。root fixed_slots=true、paints_children=true：原View完整画一次，proxy不再画按钮。caller提供稳定action_id，root_id由Content::new指定；无动作时保留同ID proxy但inactive。

根/代理分别保持消息frame与原按钮frame，避免将整个消息变成Button；proxy AX为唯一Button，name来自原action/Dismiss，原View内ButtonModel是唯一pointer/key/AX activation与取消权威。root不发Activate；成功手动动作仅proxy发一条target=action_id；被动更新不发Submit/Activate。未知part、SetValue/Increment/Decrement/文本动作无结果。

原View拆原measure、prepare geometry、paint_prepared；原字符串render入口保留兼容，内部prepare→paint_prepared。Content root arrange只短借owner，调用一次原prepare、保存唯一PreparedGeometry和完整/裁剪目标，然后**释放owner借用**，再arrange proxy；proxy只消费给定frame，不能回调owner布局。Content paint消费本次已安排geometry，不再次measure/改变hit框，不用dummy Scene预画；文字绘制使用同一backend/role/width，不能再独立排一次布局。原动作居中、首行marker几何、受限父高裁剪继续由原View负责。

禁止持owner RefMut进入proxy arrange/lifecycle/focus/Host更新/用户回调，防递归借用panic。纯measure不改arm、mount、clock或业务模型。父finite坐标/非负尺寸/finite clip为公开前置；零面积/完全clip合法无ink。独立consumer对自有非法allocation在进入Host前原子拒绝；Host clean_size只是清洗，不算fallible拒绝。新增name/配置setter先全字段验证，Surface先canonical lookup再提交；能预解析的整组packet在写Scene前完成，单组失败不留半组ink/clip-stack；不承诺所有兄弟绘制失败的整树回滚。

fixed_slots不能阻止外部children_mut破坏内部Vec；各adapter保留独立expected_action_id，validate/布局/输入/AX/typed更新检查恰一proxy、私有具体类型、ID与Rc owner一致、无额外孩子。清空/增加/同数替换/换owner均fail-closed、无索引panic；结构拒绝前不分配mount、不取消输入、不改原模型。

## 共用：动作曝光授权与typed Host更新

稳定ComponentId不能识别同mount换代后的旧AX。现有macOS AccessibilityKey.component实际携带原Content mount，app-host/platform桥分发前查当前mount；wire Target{id,mount}也查原mount。直接ContentHost::accessibility_action(ComponentId,…)只表示“现在操作当前对象”，无请求身份，不承诺分辨旧请求。延迟native/wire请求必须携原key/Target，不能只排队裸ID。

本设计保持root/原View/ButtonModel及proxy对象、ComponentId；**仅在动作语义换代时renew proxy原Content.mount授权**。原动作队列已按target+mount过滤未drain记录，可直接复用；不加epoch协议/第二队列。已经drain交付的业务动作不能由库撤回。

涉及动作语义换代的更新只能经有限typed Host入口；model只读，不提供可从component_mut绕过的动作写setter。入口顺序固定：原模型/结构/全树ID/name/Surface等全预检→原mount分配器fallible预留→局部cancel该proxy输入→一次commit原模型和proxy新mount→失效旧geometry/曝光并标Host dirty。后续成功布局才发布新动作，旧坐标不能操作新模型。原actions::next_mount已经checked fetch_update；只抽fallible底层，原入口包装并保成功序列与原panic行为，不新建counter/allocator或通用事务框架。分配耗尽是提交前明确错误，不改模型/mount/队列/capture。

局部取消沿原cancel_subtree的focus/capture/Cancel路径；它当前会help.clear全局，故不保证邻居help不变。必须保留非目标Composer的focus、marked、selection、binding与reading身份；不误清其它capture。如产品确需局部保help，抽原取消helper复用，而非复制engine。动作以外纯Theme/Surface/resize/reorder不换授权。

#74 window_active_changed已正式接收集成，直接消费原hook（本轮manifest有live源码）；它覆盖inactive保留后代、初始化/新挂载同步和真实活动转换。inactive的voidhook只撤动作可用性、取消原arm、标needs_new_exposure，不分配mount。active并不自动恢复旧授权；恢复成功布局前置以fallible方式预留新mount，pending时所有focus/AX/pointer/key均fail-closed。预留失败持续pending，不能复活旧key。mounted走原Lifecycle；重排不模拟重挂，不把窗口通知再造成新框架。

## 共用：原Host、Composer、reading与AX边界

同一个真实ContentHost混合Text、独立Button、反馈adapter和原TextFieldContent/TextSurface COMPOSER、原ScrollContent reading文档。被动model/Theme/Surface更新不新建Host或editor，不调用follow_latest/reveal/anchor来“保位置”；比较mount、focus_epoch、native binding、draft/undo、selection/caret/marked clauses、Submit门禁及reading key/revision/item anchor。高度变化走原reading transaction合同，不能要求像素位置永远不变；render在reading transaction内仍按原门禁拒绝。显式进入动作的必要focus转移与preserving_focus按原Host策略分别验，不把被动不干扰误写成所有交互永不失焦。

Group+实际Button只有被动消息和原动作语义，无自动announcement/live region或新平台role；AX Press走原View/button，不在paint轮询生成callback。调用AX方法不是VoiceOver听读证据；程序化Preedit不是物理IME。原native/wire布局快照只在成功原Host render后发布；action pending/hidden/inactive/旧mount拒绝迟到动作。

## 共用：验证与证据门禁（计划，全部未执行）

- 独立公共consumer workspace直接导入adapter；原View/原Button命中、量度与新Content同geometry；动态model/Theme/Surface继承/explicit/clear、自然/窄/零/非整数、受限父高/部分与disjoint clip、长/CJK/grapheme。name/ID/结构/Surface/finite分配预检错误各自原子拒绝；不能用clean_size当拒绝。
- 同一个真实mountedHost测试新旧AX key/Target、动作换代/重显/窗口inactive→active延迟Press、按下→替换→迟到up、Cancel、重排与纯Theme不误杀有效动作、未drain旧mount过滤。直接裸ComponentId测试不冒充旧请求身份测试；实例/owner只有一份。
- root arrange与proxy frame/原logical target一致；让backend第二次measure输出不同值的负例证明paint不改变hit框；RefCell作用域及children_mut损坏结构无panic。所有动作各一次，正文无Press。
- 原View参照与必要实际GPU：同文字backend/同packet/同frame/clip/Surface，七Surface×四mode、默认/实际品牌与独立外部Theme，1x/2x/非整数原点、正常/窄/CJK大字、部分clip；可见正常帧positive ink、clip外无ink，零/完全clip单列。Scene或空图不代替GPU；独立小宿主不代替原Gallery真实页面mountedHost。
- 原Gallery对应Playground/矩阵/变体完成真实接入后再记原生窗口结果；源码/定向tests/协议/GPU/native/物理IME/VO/性能/发布分别记录。沿#78/#79原有限验收，不追加整DS/全库native或物理IME/VO关闭门槛。
- 使用原build_lock/environment、+1.95.0 --locked --offline、原review-cache；不新target/prepare/clean/install。当前仅设计：以上未执行，旧历史成功数字不迁移为当前证据。库存/Workbench/live设计/父票同步由主协调接收实施后唯一writer处理，当前QA不改metadata。



## Main integration acceptance (2026-10-24, Issue #78)

Original five finite conditions accepted on HEAD1a7614a plus WIP, not a published revision. Source integration preserves original EmptyState/View/ButtonModel owner, readonly model guard and prevalidated typed updates. Public API/Workbench and canonical 312-type binding tests include EmptyStateContent. Root receipt: design/reviews/empty-state-content-implementation-20261024/root-intake.json (SHA256 763b7a3461c6374801715597441326ad0ae42cfe40f839ae1f19efd5990de8bb).

Root combined3 and independently locked external-current each retained 1571 stable source inputs. Internal4, registry3, default consumer10+1 external-only ignored, anchor1, wire1, Tree24; controls lib and two consumer strict targets passed. Default1176/3528 and external784/2352 cases/actualGPUframes match originalView RGBA, positive ink where applicable and zero ink for zero/disjoint clips; all outside ancestor=0. Executed programs and depinfo were archived and verified (7 default/2 external). Candidate historical 5880 frames and Gallery24 remain separate identities.

Retain strict red attempts combined1/combined2, scanner rejection and test-only macro wrapper, private helper local lint annotation and four equivalent range-Vec test constructions. The first root aggregate audit incorrectly required positive ink on zero/disjoint clips; corrected aggregator follows the existing zero-ink contract, with no renderer/source change. Earlier overwritten cache-failure program is disclosed, not recreated as evidence.

Readonly model Ref guards must not span Host operations. Existing cancel_subtree may clear global help. Restore allocation can consume reserved counters before later failure while exposure remains fail-closed; no global counter rollback claim. No arbitrary reentrant callback guarantee. Full capabilities generator remains blocked by historical runtime-observation source identity; no current whole-library/18-gate claim. Native windows, physical IME, VoiceOver, performance, publication/downstream pin remain unverified and are not extra Issue78 closure prerequisites. PR null.
