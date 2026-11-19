> Current local-main intake (2026-10-27): baseline `08d845f` plus the exact Notice delta passed public lifecycle/action/clock, mixed Composer/reading, shared Surface, strict, all nine AC5 and actual GPU/Gallery checks. Completed Dismiss survives its own hidden repaint once; observed ancestor/window inactivity revokes it. Toast/Banner window notifications therefore use window+ancestor activity while their own visibility remains model/runnable-owned; other components retain #78 effective activity. This bounded, evidence-backed distinction adds no policy framework. Evidence and exact identities: SSD `artifacts/issue-136/notice79-main08d845f-intake/root-review.json`. Native/wire/IME/VoiceOver/performance/release remain unverified. Banner Dismiss word-wrap at 320pt/type150 remains a #12 visual acceptance gap. Historical records below keep their original identity.

主协调2026-10-24正式接收本设计；实施、测试、GPU和原生验收仍待独立交付。以下候选来源与未实施标记保留其时间身份；所有最终接缝按本设计执行。

# #79 ToastContent / BannerContent 最终设计候选

2026-10-24。关联[#79](https://github.com/TekesApps/TekesUI/issues/79)，父#56、视觉#55；#9 overlay/动画仍独立。本文件纳入主协调全部最终裁决并提交正式设计接收；**仅设计，未实施/编译/测试/GPU/GUI**。65输入hash/source snapshot见source-manifest.json；HEAD1a7614a273e1a897ddc8a7ad8cb41d303a4a1290+WIP。原candidate/amendment-1不改。#74 hook已正式接收集成并在live直接确认。

## 原模型、范围与API

Toast原message/tone/visible/remaining_ms/paused是唯一权威；new/show初始化，show重设paused=false；advance在!visible或paused时不减，saturating_sub到0隐藏；dismiss隐藏且清remaining。ToastView原hover以layout.hit_test改paused、原Button/measure/render/clip/窄宽重排和显式advance已经存在。原Gallery“推进1秒”fixture不等于自动clock。Banner原message/tone/dismissible/visible、dismiss保护与原View按钮/几何也已存在，不新增消息reducer/队列或通知中心。

仅固定原字符串+NoticeTone；Toast仍只有原Dismiss，Banner可不可dismiss；没有任意action slot/undo/priority/overlay/动画扩展。原Error也可计时，不擅自改变关键错误业务规则。Toast/Banner本体无disabled模型；不新增整体disabled视觉。Banner dismissible=false是关闭动作缺省，不是禁用整条消息。

候选签名（未编译）：ToastContent::new(accessible_name,model:Toast,dismiss_id)->Result；BannerContent::new(accessible_name,model:Banner,dismiss_id)->Result；model只读、layout只读。show_toast/replace_banner/消息tone/duration/visible/dismissible等会换动作曝光的写操作走typed Host入口；pause和只读元数据走受控原owner方法，不能从component_mut改一份公开可写模型。accessible_name非空，message允许原empty fixture；duration_ms:u32包括0合法。

每次Toast显式show（字面相同也算新曝光）、message/tone/timeout含义改变、hidden→visible；Banner新公告/替换message/tone、dismissible恢复、hidden→visible，renew proxy mount。hide/dismiss/timeout立即撤动作可用性；恢复不复用旧key。timeout只changed/dirty，不伪造用户Activate/Submit。原手动Dismiss经原model.dismiss，只proxy记录一次动作。

## 几何、背景与hover

原Toast width<280 stacked，原leading36/right12或50/gap8或4、message min34与Quiet Dismiss测量保留；原Banner dismissible&&width<280才stacked，原消息槽、Primary Dismiss保留。notice_marker_frame依据真实首行对齐，ButtonTextAlign::default保留。父受限高可clip，不为适配器挤压按钮覆盖正文。

**Content采用effective inherited Surface**，不设强制panel default_surface。原裸View旧render默认panel兼容，新prepared/on_surface入口显式接effective Surface；Content默认canvas的视觉差异已明确接收。对照必须两入口同显式Surface，不拿不同默认背景的差异当几何错误。原Panel圆角/装饰hairline/elevation anatomy保留，同surface的surface-frame/content/marker/action包一并解析，不能只传Palette或只改rest颜色。

Toast root为可观察pointer的非focusable Group；root一次原ToastView.pointer_move写paused。proxy仅button-only move维护原按钮press/hover，绝不写pause；**仅对PointerMove使用原Component::bubble_handled_event允许冒泡**，让普通move/exit与captured route_from都到同一root一次。仅此事件开放：Down/Up/Key/AX/action不得借机向外漏，不用Content::bubbling_handled_events(true)开全部事件，不新增Host观察协议。root处理后按有限边界停止继续消费，不生成第二次Activate。

必须留普通move、exit、capture内move/capture拖出消息的逐次调用计数反例；不能从非捕获hover通过推导捕获路径。PointerEvent经原routing映射到PointerMove再做上述event限定。原hover-only pause语义保留：keyboard focus/AX阅读不自动暂停，手工paused也可能被原pointer_move覆盖，不新加pause理由位集。Group+Dismiss无自动announcement。

## 最小runnable与自动clock：在本票真实owner中交付

**#79同一writer交runnable/notice clock+实际ToastContent+Banner adapter，不单开空Host接口/clock票。** 原Lifecycle处理mounted，原#74 window_active_changed处理window；唯一新增默认空Component::presentation_runnable_changed(bool)只描述effective祖先/自身active与本节点非空visible clip。

递归保留children，即使inactive、完全clip、paints_children、retained exit跳paint也收到false；不能沿paint早退。传播ancestor_active，不把parent自己offscreen当所有overflow children不可见；各child看自己的frame∩clip。retained exit paint不延长计时/输入权威。false幂等无mount/editor/reading/业务动作副作用；true只在成功layout及布局后文字服务前置通过后通知，不在measure/preflight扣时。显隐reconciliation及时撤false，未成功layout的恢复节点保持不可运行。

Toast只保last_sample及必要sub-ms余数，剩余时间仅原Toast.remaining_ms；show、window inactive、Unmounted、runnable=false、paused都清sample，恢复首有效frame只建baseline、不catch-up。非runnable/Banner不要求clock。runnable且window active、可见、!paused、remaining>0才请求原ButtonTextRenderer/Host下一帧；paused/隐藏/卸载/失活停帧，恢复事件令Host dirty后重新调度，不靠consumer ticker。Reduced Motion只影响原视觉过渡，不冻结超时。

正常frame顺序：

1. 完成模型/slot/ID/几何/text/Surface等可提前判断的所有前置；成功后取得同一frame的motion_context.now，供全部notice owner复用，不每条读变化的wall clock。
2. **先只读校验全部runnable Toast时钟**（clock存在、时间不回退），不能边校验边advance第一条。
3. 任一clock无效：统一清本帧参与时钟校验的所有Toast sample/sub-ms余数（包括本来有效的owner，防止下一帧补扣被拒绝的时间段），返回明确MissingNoticeClock/NoticeClockWentBackwards类错误；任何Toast都不advance，remaining/visible/业务模型不变。clock metadata的清除是有意失败恢复，不承诺所有metadata原子不变。下一有效frame所有参与owner仅建立baseline无补扣。
4. 全部有效才正常update：无旧sample也调用原advance(0)，使duration=0在首正常update结束，不因“remaining>0才请求帧”永久挂住；有sample按非负delta/sub-ms余数安全饱和或分块调用原advance。一个原countdown，不存第二deadline/remaining。
5. 超时隐藏立即撤proxy/AX，停帧，不发Activate/Submit。本帧已安排geometry只消费，不重新测量移hit框。clock错误不自行无限request frame；等待真实新输入、Host显式重绘/恢复或有效clock配置，不能错误自旋。

时钟preflight需能从原Host render返回错误（Component.paint为void，不能在paint静默吞掉）。只用有限notice traversal/frame-local sample与现有TextAdapter，不增加通用update lifecycle/事务或新scheduler。能预判的失败不扣时；**不承诺兄弟icon/label/GPU在draw后失败时全树回滚**。原Host本来可能paint后才检资源错误，按既有失败发布规则处理，失败可清sample/禁旧布局；不clone所有模型或反向扣回时间。

window inactive的voidhook只撤可用性/原arm、标needs_new_exposure、清sample，不尝试fallible分配；恢复成功布局前置再预留新proxy mount，pending动作一直fail-closed。多owner恢复预留若有错误，不允许任何失败owner用旧授权恢复；不得把另一个owner已预留的clock/mount metadata变化描述成全树事务回滚。

## 完整Surface packet与独立Theme依赖

当前rules1.8/schema1.3；7 Surface=canvas/code/hint/panel/popover/sidebar/toolbar，4 mode=light/dark/light-ic/dark-ic。默认/现有品牌/正式独立外部Theme均需覆盖。父/背板实际Surface为同一个effective s：

| anatomy | 完整包 | 通道政策 |
|---|---|---|
| 背板 | surface-frame/rest | background=$surface,text/icon=primary-text,boundary=separator；focus/marker/helper=null；原Panel几何，不将separator称必要marker边界 |
| message | content/rest | background=$surface,text/icon=primary-text；其它null；实际读取text |
| Toast Dismiss | action-quiet，原Button状态 | rest/hover/pressed/disabled完整背景/文字/icon/boundary/focus包；marker/helper=null；不只换rest |
| Banner Dismiss | action-primary，原Button状态 | 原primary ground/foreground/boundary/focus整组；marker/helper=null |
| tone小圆 | 下述五个新rest-only用途 | marker来源精确，圆几何仍原View首行布局，无badge文字或新颜色 |

**唯一独立Theme marker执行依赖已由主协调去重创建为[#93](https://github.com/TekesApps/TekesUI/issues/93)**；#79消费其正式packet/外部artifact及独立源锁，#93不阻塞#78。本设计会话未建票或联系Theme候选会话。五用途精确七通道：

| usage | background | text | icon | marker | boundary | focus | helper |
|---|---|---|---|---|---|---|---|
| notice-marker-info | $surface | primary-text | primary-text | status-info | null | null | null |
| notice-marker-success | $surface | primary-text | primary-text | status-succeeded | null | null | null |
| notice-marker-error | $surface | primary-text | primary-text | status-failed | null | null | null |
| notice-marker-toast-warning | $surface | primary-text | primary-text | status-failed | null | null | null |
| notice-marker-banner-warning | $surface | primary-text | primary-text | status-running | null | null | null |

两Warning业务tone仍Warning，语义文案不改成Error或Running；用途区分原View的视觉来源。这里只读marker画原小圆，background描述实际背板ground；text/icon是完整中性通道，本阶段无额外anatomy。禁借#83 status-badge：其solid fill/on-fill foreground与Notice小圆不同；content-status marker=null，也不冒充本用途。

精确rest-only状态白名单，hover/pressed/disabled/selected/focus伪状态拒绝；background/marker/text/icon/null绑定必须精确验证，不因数值碰巧通过就允许Warning换角色。**marker/实际parent ground≥3**；消息text/ground沿已有一般≥4.5守卫，不夹带全局增强≥7政策。无额外必要boundary；外部Theme不达标整份拒绝，不consumer补边/派色。decorative separator/shadow不充当marker对比补救。

Theme writer执行#93，新增五用途及Python/正常Rust/外部Theme同等语义与对比校验，必要版本const/pins/生成输出/源锁同步；现有37用途逐字段/state/rolemap保留，新增5共42用途；七Surface是required覆盖，不将原允许extra Surface的规则收窄为only7。不改#83含义、不新palette role/token/RGB或schema形状。最终版本主协调确定，不把旧1.8冻结改签。Theme candidate可并行准备，**整合在#76之后**；adapter使用接收后的精确packet handoff。

Theme反例：缺/多通道、两Warning互换、错误marker角色、非rest、未知Surface、自洽重生成但marker/ground不足、外部源锁不一致；合法→语义负例→合法恢复。Theme包生成通过不代表原Toast/Banner实际消费/GPU可读通过。

## 本票额外真实验证及ownership

除下方共用验证，必须通过公共ToastContent在真实mountedHost自动推进：多个Toast、0ms、长elapsed/分数ms/饱和、hover进出/capture move、paused恢复、show重置、hide保remaining与dismiss清零、全clip/ancestor inactive/retained退出/窗口失活再恢复无catch-up。任一clock无效使本帧所有Toast零advance；错误sample清理可观察、下一有效帧baseline、无无限帧请求。Banner不可dismiss/隐藏与恢复单独验，不接timer。

shared授权helper复用#78接收hunk；本票writer负责runnable/clock与真实两adapter、原View prepare/paint_prepared/Surface/routed activation、独立consumer及原Gallery feedback_adapter。同writer避免clock空接口先验收或跨owner环依赖。新composition/notice.rs和composition模块/Host有限glue交主协调串行窗口；Theme独立writer执行#93，global metadata由主协调最终同步。无Kernel/ClientKit/Agent UI修改；Issue OPEN，无commit/push/PR/发布/清理。

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



## 主协调共享Theme接收（实施仍待交付）

#93已串行接收rules1.9.0/schema1.3.0，五notice-marker用途与实际parent≥3守卫已交，原37用途/packet/rolemap不变。正式artifact与独立锁见 design/reviews/notice-marker-theme-implementation-20261024/，锁SHA80d5fe7eef99b9c707269c90bd042239df09f425d1bbfc02e993f0157b842d75。上文“当前1.8”保留其设计时来源；后续#79实际消费以本次1.9正式handoff为准。共享生成/普通Rust通过不代替#79原View/Gallery/GPU验收。


## Main serial source-intake record (2026-10-24; WIP, not acceptance)

The main-owned serial intake now exposes `composition::ToastContent` and `composition::BannerContent` through the existing component family routes. The live public surface also has `Component::presentation_runnable_changed(bool)` and the finite `ContentHost` notice methods `show_toast`, `set_toast_visible`, `set_toast_paused`, `dismiss_toast`, and `replace_banner`; `CompositionError` includes `InvalidNotice`, `MissingNoticeClock`, and `NoticeClockWentBackwards`. The original `ToastView`/`BannerView` prepared geometry, Surface, dismissal and pointer helpers remain the owner path.

Workbench continues to use the existing Toast/Banner routes and story matrices: static Toast specimens use the existing paused model; Playground uses the original frame clock; Banner has no timer. This is source/API inventory synchronization only. Issue #79 remains open for its original finite intake and acceptance; parent #56 synchronization follows the coordinator's acceptance. Native window, physical IME, VoiceOver, performance and release evidence remain separate.


## Main finite implementation acceptance (2026-10-24)

Issue #79 original five conditions are now accepted against the serial #97+#79 source union, after the earlier WIP-only intake note. Root evidence: `design/reviews/scene-notice-main-integration-20261024/`. Current Controls all281, finite public consumers/wire, all-target strict, Gallery/downstream and actual GPU passed. Default9408 cases/28224 frames and independent external6272/18816 compare byte-identically to original View, with zero ancestor-clip escape and positive-ink checks. Keep Notice structure rejection scope distinct from legacy global-ID cancellation. Missing/backwards clock and inactive/hidden lifecycle remain on the original countdown; no second ticker.

Public type classification/Workbench bindings and normal generated323-type compiler fixture are current; 18 observations were freshly executed, not re-signed. Root default/SVG binding tests and strict also passed. Native windows, physical IME, VoiceOver, performance and publication are unverified, with no extra whole-DS/native prerequisite. Existing static Toast specimens are paused; retained Playground uses the original frame clock. No publication or downstream pin occurred. Historical errors and exact old identities remain unchanged.


## Main original AC5 clock follow-up acceptance — 2026-10-24

The reopened Workbench clock gap is accepted on current main source: original seven MissingNoticeClock regressions pass; two added tests prove exact optional static context restoration/repaint and real retained Gallery clock5000→4875ms after125ms under Reduced Motion, without duplicate same-time decrement. The fixture boundary borrows an explicit original sample (ZERO only for deterministic missing fixture context) and restores the caller; original SDK missing/backwards-clock rejection and actual live frame clock remain unchanged. No second ticker/permanent interactive pause. Main reran144 mounted Gallery actualGPU cases (72Toast original automatic clock,72Banner no timer),96 static matrix/variant cases with the originalGPU lock,12 public notice tests and Gallery all-target strict. Existing criteria1–4 retain original accepted identities, not re-signed as this build.

Receipt `design/reviews/notice79-main-clock-intake-20261024/root-acceptance.json`, closure https://github.com/TekesApps/TekesUI/issues/79#issuecomment-6384270097 . Initial main fixture invocation refused before tests because only coordinator ledger metadata changed after compile; round02 records this exact delta and verifies unchanged actual compiler inputs/depinfo. A receipt-only assertion initially incorrectly required Banner automatic clock; corrected to the original no-timer Banner contract, without changing source/test/GPU outcomes. No public API/type/Theme change. Physical/native-window/IME/VoiceOver/performance/publication remain unverified; no fullGallery/wholeDS extra gate.
