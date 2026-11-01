# MediaViewer：G6 最小设计候选（#201）

日期：2026-10-31（Asia/Shanghai）。状态：implementation candidate / runtime unverified。链接执行票：[#202](https://github.com/TekesApps/TekesUI/issues/202)。
关联：[规划 #201](https://github.com/TekesApps/TekesUI/issues/201)、组件规划 #56、体验验收 #12；复用已关闭交付的 #117。

## 1. 范围与核对结论

用户后续授权设计后继续拆分实施，覆盖r1“首批仅设计/另行授权B1”的旧阶段限制。已建立#202几何、#203 adapter与#204 consumer/验收，三域并行组成可消费批次；只在隔离候选实施，MAIN接收/共享构建GPU窗口仍由集成单owner负责。不改变现有15领域、#131–135或物理IME/VO/性能延期。#201与#202继续OPEN，源码存在不等于运行时通过。

只读源码基线：`cbe40461f9bb7c6b763be63eae16b6c5a767382f`，目标 `/Users/frankqin/.codex/worktrees/tekesui-main-audit-20261026/TekesUI`；相关文件实际字节和既有WIP见 evidence/source-identity.json。旧 `gallery-design-baseline/TekesUI` 只读检索发现 bitmap.rs，无 MediaViewer 命名实现；名称检索不能证明任意业务代码中不存在类似行为。

权威原契约：`design/behavior/reference-alignment.json#/media_viewer`：相对Fit倍率1–8、reset=fit and center、先解锚点再clamp、未钳制漂移上限2pt、空viewport安全、所有状态可退出。`#/local_recipes` 中 `agent.media-viewer` 是 consumer-recipe，不是既有公共组件。保留该ID以兼容历史引用；新增实现采用通用图片Viewer，不引入Agent业务类型。

`design/behavior/gold7-alignment.json`：G6/zoom-preserves-reading-anchor=missing；bounded-split-and-pan-geometry、cancel-and-non-gesture-alternatives=partial。此设计不改这些状态、不改变已有Split证据。

#117 当前已closed，公开交付PR160、merge `067e1306914b938398945e84a2589803b76b5da6`；其Issue记录了bitmap consumer/lease/GPU/本地AX有限通过，但不证明本Viewer通过。当前源码 bitmap.rs 的 ImageContent 已有 owner-bound request、ready/loading/missing/failed、旧像素保留、卸载撤销、alt/decorative；Renderer的BitmapImageLease是唯一资源路径。无需重写这些能力。文档历史“待发布”段落不能覆盖Issue后续交付收据。

GitHub精确检索media/viewer发现#201为直接规划入口；#117是图片基础，#46为被动preview，#49为交互浮层，#4/#9为作用域/生命周期，#5/#120为语义/远端。后者仅是能力边界参考，不能把其整票完成增加为本地Viewer前提。首批执行前复查执行票，复用已存在的合适票。

## 2. 所有权与公共消费方向

执行#203在可选 `bitmap-images` 下提供通用 `MediaViewerContent`，作为现有Content树中的viewport容器，只持有一个原ImageContent子节点。它拥有变换、交互会话和命令；ImageContent独占资源加载状态、request token和lease。Viewer不得复制一个加载状态机。宿主提供预解码资源、原ImageLoadOutcome、逻辑固有尺寸与替代文本；对mounted原ImageContent应用完成后，再由同一调用栈通知Viewer几何变化并请求redraw。

Viewer通过原 `children/children_mut` 暴露原Image Content，将其arrange为变换后的完整图片矩形，clip取viewport与祖先clip交集；Contain作用于同宽高比的矩形，不进行第二次fit，不改变UV或自己push ImageQuad。需在B2公共consumer实际证明此路径，不以设计推断通过；若原树布局不允许该矩形，保留失败交协调者决定最小接口，禁止旁路私有renderer。

既有Dialog/OverlayScope、Button/IconButton、Progress/Banner负责外壳、动作、状态提示和焦点生命周期。可交互viewport拥有一个原焦点入口；图片本身仍为被动Image，不自建捕获或Tab系统。默认modal外壳由公共消费示例组合，Viewer不强制拥有产品窗口。

建议命令（名称未冻结Rust API）：Fit、ActualLogicalSize、ZoomIn、ZoomOut、ZoomAt(relative_scale, viewport_point)、PanBy(viewport_delta)、Close、Retry。Close/Retry由外壳原Button回调，transform_changed只在实际有效变换变化后发出一次，包含mode/q/translation；无变化、拒绝和纯查询不发动作。Host不能同时双向绑定第二份可写transform。

最小公开输入：原Image Content、viewport（arrange产物）、可读load status/逻辑尺寸、动作回调；return_focus_id交原OverlayScope生命周期管理。resource/load并非另一个Viewer资源容器。请求凭据继续原ImageLoadRequest；不序列化lease、不支持远端Viewer协议。

## 3. 几何（全部为viewport本地逻辑pt）

图片固有逻辑尺寸 I=(Iw,Ih)>0；可用viewport V=(Vw,Vh)>0。viewport不含toolbar/helper占用。原Host绝对坐标仅在输入/arrange边界加减viewport origin一次。

- Fit基准 `f=min(Vw/Iw,Vh/Ih)`，相对倍率 `q∈[1,8]`，实际逻辑缩放 `s=f*q`；图片矩形 `D=(T.x,T.y,s*Iw,s*Ih)`。
- Fit模式：q=1，T=(V-sI)/2。允许小图放大，遵循原fit契约。
- Manual模式：ZoomIn/Out按1.25倍或倒数调整q并夹到[1,8]；命令锚点为viewport中心，指向缩放的锚点为原指针位置。Fit是显式模式；Manual即使q=1也不自动变为Fit。
- ActualLogicalSize要求s=1，故 `q_actual=1/f`。仅q_actual在[1,8]时可用；否则禁用并说明“原始逻辑尺寸超出当前缩放范围”，不偷偷放宽边界，也不把clamped倍率显示成100%。100%指1图像逻辑pt:1视口pt，不承诺1 bitmap pixel:1 backing pixel；后者是未授权扩展。
- 锚点O有效且在viewport内：先算 `T_next=O-(O-T_prev)*(s_next/s_prev)`，再按轴clamp。不得先clamp O对应的内容点或用旧fit比例当实际比例。
- 每轴图片长度Dlen>Vlen时 `T∈[Vlen-Dlen,0]`；Dlen≤Vlen时强制居中 `(Vlen-Dlen)/2`，不保留无意义pan。小图轴禁止空白拖出；不允许无限pan、弹性overscroll或惯性。
- PanBy和pointer delta均为视口pt，不再除以s；若通过图片坐标计算则必须先转换。仅溢出的轴变化；到边界无变化命令不产生transform_changed。
- 数值边界在公共输入/尺寸更新处验证一次：非有限、非正图片尺寸拒绝，保留原有效状态并返回现有错误路径；非法命令不改变变换或活动交互。内部用f64计算，在转入f32/Rect前确认有限可表达；不重复验证已接受的不变量。

## 4. Resize与源变化

Resize取消旧手势、保留最后已发布变换，再按以下确定规则布局：

1. Fit模式重新Fit居中。
2. Manual模式以旧viewport中心C下的内容点 `P=(C-T_old)/s_old` 为阅读锚点，保留q（相对Fit符合原scale_relative_to），算新f/s和新中心C'，`T'=C'-s'*P`，最后clamp。未clamp轴内容锚点不漂移；clamp轴的移动是边界强制结果，必须在测试单独记录，不能算成未钳制2pt通过。
3. Actual命令进入Manual，不锁s=1；后续resize依Manual规则保持q。100%值随resize更新，避免暗示仍是实际逻辑尺寸模式。
4. 零面积/隐藏viewport暂停图片绘制和变换输入，清除手势，不除零；保留最近有效V/q/T/mode作为恢复锚点。再次有效时以上次有效viewport为old执行上述resize。无先前有效几何则Fit。
5. source begin_load及Failed沿ImageContent保留旧像素；暂禁zoom/pan，清除交互，保留旧变换。新Ready成功接受（包括同尺寸资源替换）默认Fit；Failed恢复旧图交互并显示错误；Missing无图只允许Retry/Close。旧/外来完成被ImageContent拒绝时Viewer也不更新。加载完成通知与Viewer更新在同一host更新中完成，不能发布半帧。
6. close先撤销当前交互并卸载原图片Content，使旧request失效；reopen是新会话、Fit；不通过复用原return-focus整数接受旧事件。ready lease释放继续遵循原Content/Scene所有权，不手动销毁共享资源。

## 5. 交互状态与取消

单owner内部交互 `Idle | Panning(start_pointer,start_T,session) | Zooming(session)`；open/closed归外壳，load状态从原ImageContent读取，不把ready+panning等做成相互矛盾的独立布尔值。

| 触发 | 准入与结果 |
|---|---|
| primary down | open、ready（或failed有旧图）、有效viewport且至少一轴溢出才进入Panning，经原Host capture；不触发图片Activate |
| move/drag | 原session内从start_pointer/start_T求变换并clamp；up结束，结束事件无重复提交 |
| 缩放命令 | ready有效viewport；锚点解算后clamp；连续命令每次有效变化一次通知 |
| Cancel/丢捕获/失焦/窗口失活/禁用/隐藏 | 结束会话、释放原capture、保留最后已显示变换；旧up/move不再生效；不回滚到拖动起点制造跳动 |
| resize/source变化 | 先取消旧输入，再按第4节更新；旧会话事件无效 |
| Escape | 原Host在modal Dialog内优先关闭Dialog并取消捕获；非modal viewport有手势时取消并消费。原IME屏障优先，不窃取候选取消 |
| Close（所有load态可见） | 一次close；立即停止输入/语义参与；沿原scope恢复原调用者，若已卸载/禁用则沿原fallback，不自造焦点恢复 |
| Retry | 原Button一次动作通知host；不隐式加载、循环重试、访问路径或网络 |

本地现有ContentEvent已有Scroll/Cancel及Pointer元数据；本轮检索未见macOS magnifyWithEvent入口或Content magnification事件。首个实现以pointer pan+显式命令/键盘闭环为基础，不伪称pinch已支持。原指针ZoomAt通过公开命令可测试，物理pinch、modifier-wheel缩放需明确另行输入扩展才可承诺；普通wheel不偷换成zoom。未来若接手势必须调用同一owner，并保留相同替代操作。

## 6. 非手势操作与语义

可见原Button：缩小、放大、适合窗口、原始逻辑尺寸、关闭；错误态按需Retry。viewport有焦点时方向键每次PanBy 32pt；放大/缩小/Fit/Actual均可通过Tab抵达按钮并Space/Enter，故不依赖字符快捷键或新TextCommand映射。操作说明说明方向键平移、Fit复位、Escape按原外壳策略关闭或取消。按钮disabled由同一owner的边界/状态投影；不是仍可激活的“灰色”装饰。

沿现有Image alt/decorative。modal外壳有名称及原Dialog语义；viewport使用原可命名Group语义与独立焦点入口，语义投影显示缩放百分比（实际s×100%，标明逻辑尺寸）和操作说明。不把图片冒充Slider/TextInput，不新增wire角色。公开Group聚焦/值投影若现有契约不足，B3先给最小准确差量，不宣称自动存在。按钮AX Press调用同一命令，原mounted owner/mount校验；旧AX对象、closed/disabled均无副作用。图片只出现一次语义，状态/错误文本与退出始终可达。

采用现有surface-popover、scrim、primary-text及原组件Theme/Surface，不在Viewer派生颜色。Reduced Motion即时更新；本设计不加动画。窄窗/150%字号优先让toolbar换行或纵向排列，viewport扣除真实测量高度，不覆盖关闭按钮；无可用图片区时仍可关闭。

## 7. 验证矩阵与原AC映射

以下均为后续验证计划；本批未运行Rust/GPU/AX/原生。

| ID / 层 | 必需输入与断言 | 映射 |
|---|---|---|
| M1 几何 | 横/竖/小图，q=1/8及超界，无clamp锚点、单/双轴clamp；原2pt上限不放宽；至少一组人工可算值 | G6 zoom；#201 AC2 |
| M2 resize | Fit与Manual，窄→宽→窄、viewport origin变化、0→有效、toolbar变高；固定q中心内容点/强制clamp分列 | G6 zoom/pan；AC2 |
| M3 状态 | ready/failed旧图/missing/loading，down/drag/up/Cancel/lost capture/focus/window inactive/disable/hide；resize/source切换旧up无效 | G6 cancel；AC2 |
| M4 生命周期 | A→B旧完成、跨owner、卸载重挂、close/reopen、失败保留/新图Fit；两Viewer共享lease，卸载一个另一仍可绘制 | #117既有回归复用+Viewer增量；AC1/3 |
| M5 公共消费 | 非Gallery独立consumer只经公共Content/Renderer；原图片子树实际frame/clip/命中/alt；toolbar与viewport焦点、一次动作、父嵌套clip | AC3/4；G7 public recipe |
| M6 输入/本地AX | 原Host pointer/key、Tab按钮/方向键、按钮与AX一次命令、stale对象拒绝；关闭恢复与IME屏障；programmatic/native对象/真实输入分层 | G6 alternatives/cancel；AC2/4 |
| M7 GPU | 透明图片、明显四角标记、父圆角clip，Fit/中心zoom/边缘pan/resize/empty及loading/error；Light/Dark/IC×1x/2x，窄窗150%，保原PNG逐图审查 | #12整页/历史保全；AC4 |
| M8 原生窗口 | 真实拖动到界外释放、失活、resize、按钮及键盘、Close/Retry/焦点恢复；按受控窗口记录实际输入来源，pinch不在首批承诺内 | #12真实键鼠；AC4 |

#201五项AC对应：AC1核对见§1/2及source identity；AC2设计§3–6；AC3责任§2/4；AC4矩阵本节与批次§8；AC5通过Issue摘要+隔离补丁交审，接收决定仍待协调者。#12的VO/物理IME/性能继续原延期；本地AX和原生键盘不能替代VO，截图不能替代性能。M7有限Viewer范围也不能升级全库experimental状态。

## 8. 并行执行票与唯一文件owner

用户已授权并行实施；#202现只负责B1/core类型，#203负责Content/Host adapter，#204负责独立public consumer与验收。几何接口已固定，三域按独立文件先实施，实际构建依赖按集成窗口顺序执行，不等待额外设计批准。

| 执行票 / chat | 文件唯一owner |
|---|---|
| #202 本几何域 | composition/media_viewer.rs仅五公开类型/模块接线；composition/media_viewer/geometry.rs纯几何及局部测试 |
| #203 01a18ef5-82c8-7d82-85ac-343018b08105 | composition/media_viewer_content.rs及局部测试，作为media_viewer的nested content模块，使用super::Geometry；不改geometry |
| #204 01a18ef5-8ba8-7941-b313-070ff25af59d | 独立consumer/M4–M8夹具及验收证据，不改两域生产文件 |
| 集成01a18c4f-d227-7900-8390-520b735d0c4e | MAIN与共享exports/bitmap查询/manifest/Cargo/Host/平台精确hunk接收、串行窗口 |

前缀均为crates/tekes-ui-controls/src/。完整固定接口和最小向量在SSD artifacts/issue-201/implementation-20261031-r2/interface-contract.md。core Error新增Image(ImageContentError)+From/标准source，供adapter加载方法准确传播；加载方法Result错误同为MediaViewerError，坏child返回InvalidImageChild不panic。原adapter未编译WIP已冻结交#203，原source不删除。下表保留依赖逻辑，职责以上表为准。

| 顺序/建议标题 | 交付与验收 | 文件责任/依赖 |
|---|---|---|
| B1 `[MediaViewer] Fit/zoom/pan/resize geometry and cancellation owner` | 私有geometry/state owner+M1/M2/M3纯测试，命令/错误/通知契约；不造加载或焦点owner | 本域唯一writer：新composition/media_viewer.rs及其局部测试；当前已在隔离候选实施；共享exports只交精确hunk |
| B2 `[MediaViewer] Optional public Content consumer over ImageContent` | 原图片子节点与viewport安排、loading/lifecycle桥，独立public consumer，M4/M5；保#117断言 | 依B1；本域新module/consumer；composition.rs导出、public-api-surface/Cargo既有optional分类由集成者单writer精确hunk；bitmap.rs原则零变更 |
| B3 `[MediaViewer] Original Host input, button alternatives and local AX` | M6及同owner命令、Dialog恢复/Cancel，明确Group投影实际限制；不含远端协议或VO | 依B2；本域Viewer+consumer；Host/平台AX若必要先协调文件owner，按精确hunk接入，不另派并发writer |
| B4 `[MediaViewer] Scoped GPU and native acceptance` | M7/M8冻结输入、程序/图片/输入来源/失败证据；继承#12矩阵，反馈缺陷回原owner；文档和不可变公共consumer指针 | 依B2/B3；先用既有build_lock/environment/注册SSD/cache/GPU guard由唯一集成会话分配窗口；无新target/缓存；全库CI只进兼容集成批 |

B1可不占桌面但仍须按仓既有串行build窗口执行；当前几何候选待与#203/#204源合并后由集成分配窗口。共享接口冲突先停该冲突hunk，保全候选交集成会话 `01a18c4f-d227-7900-8390-520b735d0c4e`。不通过发消息例行催进度。

## 9. 决策与接收边界

常规选择已具体化：Fit-relative 1–8、Manual resize保q、中心锚点、取消保最后显示值、Logical Actual受范围限制、无惯性/动画、原Button替代。无必须阻塞本设计的产品争议。

用户已授权上述窄方案的隔离生产候选实施及验证，#202为执行入口；协调者保留集成/发布接收责任。若要物理pixel Actual、超出1–8、pinch/modifier-wheel、Gallery新根、远端Viewer、文档/视频/网络或业务图库，必须先形成独立范围决定；本候选不实施。无常规设计批准门槛；只有实际产品范围选择或共享文件owner冲突才提出具体问题。

设计补丁新增本文、在既有reference说明及#/media_viewer添加候选ref；不替换原契约、修改scale_range/状态或自动刷新generated report。集成者接收时按当时源重核hash/应用检查，若正式采用并改变有效设计源再走原生成链。本文及所有候选仅在SSD artifacts，MAIN无写入。


## 10. 2026-11-01 纯几何交付与分层验收

本节覆盖上文历史的“测试未运行/等待首窗口”进度描述；原数学、状态契约和M1–M8门槛不变。纯几何B1（#202）与原Host/adapter（#203）、独立consumer及GPU/native（#204）可分层接收。#202只负责公共core类型与私有geometry；捕获、Cancel/旧up、生命周期和原Host动作的实际行为证据属于#203/#204，不以纯几何测试代替。

原82d08edb84879d341cec48899471d3acbc2de8fc实际编译执行11项geometry测试全pass，core采用标准mod geometry/mod content布局；原geometry SHA256 7e3011f247345ac2b50c296057e307f17b8de807db1e1c1f4422433a17d7e838。2026-11-01核实公开origin/main=04992fa9ee89016551f9e95e5b10ce1bfb742493尚未包含两文件，不能将本地集成或CPU通过写成公开交付。精确几何/core候选和exports单hunk见SSD artifacts/issue-201/public-geometry-delivery-20261101-r1/；core依赖#203提供同目录content.rs，完整接入与公共API manifest/generated metadata归唯一集成者，不能只接core后宣称可独立编译。

#201设计五项AC可凭本文§1–9、既有Issue拆分/接口责任、#202/#203/#204与本次交付证据映射独立评审，不要求M8先完成；#201的设计完成不代表G6运行时全部验收。#202 M1/M2及纯状态AC已有确定性CPU证据，公开集成时按CPU复验卡检查改变的相关输入；正式公开PR/merge收据到位后可关闭纯几何票。原M7/M8以及物理IME/VO/性能等未满足项继续在#203/#204/#12保持原AC，不迁为纯逻辑阻塞，也不删除或重标为通过。当前无新的产品选择；物理pixel Actual、pinch、网络/视频/图库仍不在本次范围。

## 11. #203 原生 Tab 红例的组件焦点边界修复（2026-10-31 候选）

本节保留 #203 已实现组件的有效行为与原候选解释；其中“待验证”表述属于历史计划，分层实际结果与公开交付边界见§12。历史 decb/c5c→f69 真实原生输入中，Tab 从 Retry 未进入 viewport 的 RED 保持。原 VIEW 非空、可见、可聚焦且未禁用；原 Legacy whole-viewport witness 两个底角落在零 padding Panel 的10pt圆角外。Shared Host 拒绝该完整 witness 并保留 Retry 是原合同，不修改 Host policy、consumer padding、Tab顺序或操作卡以绕过。

### 同一真实阅读区域驱动绘制与准入

Viewer 的键盘焦点边界是其当前可见阅读 viewport 的完整可用内框，而不是任意中心点或缩成一小块的准入标记。

1. 从 arrange 的 viewport 与矩形祖先 clip 的交集开始。逐项取当前 CurveGeometryContext 的 canonical shape；当前整框四角均在 shape 内时保留原框，否则与该 shape 按最大 canonical corner radius 内缩的平坦中心矩形相交。此法是保守的、可解释的 viewport 边界策略，不声称提供通用最大 logical innerRect 求解器，也不借用特殊 dyadic/pixel proof API。
2. 在所得区域内预留原 FocusRingMetrics 的 AA guard，再以所有当前 shape 的四角 contains 查询核对整个覆盖域。此覆盖域就是 RequiredFocusSpace::Sufficient 的 witness；同一 witness 内缩当前 metrics.reserved_band() 得到原 paint::focus_ring 的绘制目标。metrics.bounds(target) 对应该 witness，quad外框另为 witness 内缩 AA guard，不能把 quad 外框冒称完整 AA coverage。
3. 区域任一轴不能容纳两侧完整 reserved_band 时返回 Insufficient，不依赖 Rect::inset 钳零伪造可用区域。无当前布局、撤回/待发布几何或无法认证数值查询时返回 Unmeasured；这些状态都没有充分 witness 或焦点描边。
4. 实际尺度沿原 arrange 的 ButtonTextRenderer::backing_scale() 输入，原 native 在 tree.render 前已提供 window.scale。无需新公开 setter 或 consumer 接线。无效尺度不使用 FocusRingMetrics 的兼容 fallback 来制造 Sufficient；paint 与 arrange 尺度失配须先撤回旧证据，等待新 arrange。

在实际红例 viewport=(0,188,480,532)、Panel=(0,0,480,720)、radius=10、1x guard=1pt 时，完整 witness 为 (11,189,458,520)。这是整个有意义阅读内框；不是任意微小 focus target。图片逻辑变换、命中范围和语义 frame 仍沿原 viewport/祖先 clip，witness不改变zoom、pan或图片加载范围。

### 证据、绘制与生命周期

RequiredFocusSpace/evidence 缓存在同一 owner 的 arrange/context产物中，getter只clone同一Arc，不按读取次数构造新token。arrange、矩形clip/当前曲线generation、尺度、禁用/可运行/窗口活动及卸载按相应生命周期失效。clip回调与匹配context发布之间为Unmeasured；撤回曾有曲线context使旧layout不能被availability调用重新授权。重新安排和当前context可恢复新证据。焦点获得/失去本身不替换布局证据；键盘focus_visible决定描边，blur仍取消原drag。

Viewer通过既有 paints_children 合同先调用 retained 原 ImageContent::paint 一次，再在同一树/当前clip内绘制原主题焦点环，从而不会被图片盖住。资源lease、图片加载、输入、语义、子节点lifecycle均仍是原owner；不旁路renderer、不复制图片primitive、不另建焦点状态机。focus_boundary_visible与实际描边条件一致，供原Panel组合。

### 验证增量与保留边界

- 4项新增CPU覆盖：真实Host的480×720/10pt Panel/Retry→VIEW Tab（无强制VIEW focus）、同绘制coverage与稳定getter；disable/renderer尺度/resize证据失效且图片逻辑几何不变；clip/context撤回/重测量/unmount；小到容不下ring的viewport拒绝充分证据。使用Loading图片的CPU输入不证明真实Ready/资源准入。
- 1项新增ignored真实Renderer/GPU测试：4主题×1x/2x，真实原lease、原图片仅画一次且焦点primitive在图像之后、Tab进入VIEW、实际focused前后像素发生变化。数值测试不是图像审查。
- 必须新增有界focused图审：原consumer在480×720、radius10、4主题×1x/2x，成对保留unfocused/focused共16张新PNG与源/程序/尺度/witness/quad/coverage记录；逐图审查环可见、对比、圆角内完整、不被图像遮盖且无重复图片。旧112张unfocused证据保留其历史范围，不能升级为本focused分支通过。
- 必须重新执行原生失败入口：原真实程序与输入 provenance 下 Retry 后 Tab进入VIEW并出现边界，之后方向键、blur/反向Tab、resize/尺度变化和正常Close按原owner验证。历史RED不可覆盖；modal、AX stale、物理IME/VO/性能继续各自原范围，不能由CPU或截图替代。

本候选只改 #203 标准 composition/media_viewer/content.rs 及本设计追加节；不改 geometry、consumer、共享Host或原图片模块。集成接源后按已存在 build_lock/environment/注册SSD cache分配定向CPU/strict，再分GPU/原生窗口；当前无资源执行授权，无MAIN写入。

## 12. 2026-11-01 #203 公共源码接入与复验边界

公开 main 04992fa9ee89016551f9e95e5b10ce1bfb742493 无 MediaViewer；本批只重组已验正常 fdb0 源的 Content/局部测试，不引入 TEMP、产品图库或无关平台 AX 实验。adapter SHA 0315bbb9669e2ebe6dfa072c6f3258d4dd86c4fbe6a6be916ff2ecf55cab3171；几何/core 与独立 consumer 分别沿 #202/#204 原 owner 接入。共享文件由唯一 Main 接收精确 hunk：原 ImageContent displayed_logical_size 内部只读查询、原 capture-current pure query 与 Host 两处撤回检查，以及已定位的 stale-spatial gate 只拒绝 positioned pointer/scroll、仍允许正常 Escape 的修复和原 modal 回归。保留公开基线 Disclosure/HostRevealVerticalClip 等行为，不覆盖整份历史 composition 或 metadata。

CPU03 原 adapter13/strict、CPU04 修复测试程序编译/strict、实际2f repaired GPU3/focusGPU、FreshPublic04 fdb0 public9/strict 与历史 native/window 证据各保原 source/program；源码相同不把旧 programme贴成当前公开编译。新公开 Host/curve/layout/dependencies 不同，需要新基线 scoped 编译与受影响 CPU/原 Host/图片回归，再由 #204 检验受影响 GPU/public/native；未变几何数学向量可分层复用，不必以新 head 本身为理由全量重跑。精确命令与准入见 artifacts/issue-203/public-content-delivery-20261101-r1/VERIFICATION-CARD.md；卡不是执行记录。

本批提供可接入源码，不关闭 #203：M3–M6 的真实 local AX stale 对象、IME 屏障及 #204 M8 余项继续保留有效 AC，现有有限 Tab/箭头/modal/图像通过不代替未验条件。#204 owns consumer与前台输入；#201/#202 owns geometry。公开交付、CI、metadata生成及窗口由唯一集成者串行安排，禁止另建 cache/target、重复候选或为源码批关闭未完成验收。

## 13. Actual public-source candidate verification, 2026-11-01

The compatible ordinary-source batch contains the original geometry/core7e3011/299080, adapter0315, minimal pure capture query/Host stale-positioned-input correction, original normal consumer and five bitmap-optional public API classifications. It retains the current public Disclosure/HostReveal contracts and the existing45 registered component routes. No new bitmap feature, Gallery route, loader or renderer was introduced.

CPU041 compiled and executed candidate c4522da439f3914274a74cea4f7608cbf923d7ad. Its independent defaultAPI3, Media24 (geometry11+adapter13), bitmap3, composition16, pointer10 and focus_restore5 passed:61CPU, plus4 explicitly ignored GPU cases. The bitmapAPI suite returned2PASS1FAIL because Main appended the five metadata entries rather than inserting them in canonical API order. The original failure/raw/programs remain immutable; no production or assertion was changed to hide it. Main470c7912227424706bdfa294e5167b36c46a8df8 repaired only the three metadata JSON files, preserving all446 records and the original441-entry relative order.

CPU042 on470c/tree0936f143a31a143ffec829931305e09e60475da3 freshly passed the complete repaired bitmapAPI3 and independent consumerCPU2, and completed allfive compile/strict stages: Controls bitmap lib/tests, consumer three test programmes, normal native example build and the original CPU/public/native strict union. The native example was not launched. Main independently accepted the original61 under their original CPU041 programme identity: allother3633source files identical, default selected type projection and all45 component records identical, retained programmes/dep-info hashes matched. Thus relevant CPU acceptance is61qualified+5fresh=66; never66fresh. Historical four GPUignored are not passes.

GPU043 executed the actual CPU042 public_viewer archivefd1d121d97d0b255bfeb30a6465515fa4b1cd46c15903e7eae01b16d4da0202e once with --ignored --test-threads=1. Allnine exact public API/pixel/input cases passed,0failed/0ignored. Compiled470c/sourcefreezee8af and the fresh execution grant remained separate, fully bound identities. This was offscreen/programmatic Host evidence, not physical/native input. No Cargo, relink, matrix/focus-image repetition or renderer retry occurred.

Main checked actual raw/counts and programme/dep-info hashes,3636source bytes/timestamps, cache markers and actual owned PID/PG/lock/guard retirement for each window before this result append. Primary receipts: artifacts/issue-203/public-cpu-execution-20261101-r1/main-resource-release-acceptance.json, artifacts/issue-203/api-order-c4522-20261101-r1/main-qualified-cpu041-61.json, artifacts/issue-203/public-cpu-continuation-20261101-r1/main-resource-release-acceptance.json and artifacts/issue-204/public-gpu-only-runtime-r1/main-resource-release-acceptance.json.

Reference alignment was regenerated through its existing compiler because the new design_candidate reference legitimately changed its audited JSON input. The actual check passed15 mappings,13profiles by4themes and12688 declared contrast checks; native remains unverified. No color/policy/source compiler change or acceptance promotion was made. Required compatible CI/publication remain Main delivery gates. Existing native M6/M8, actual AX/IME/VoiceOver/performance and new112matrix/16focus acceptance remain separate and open; ordinary source/CPU/public9 delivery does not wait for those advanced conditions.


## 14. 2026-11-01 M6 composing-editor containment A

M6 原生 IME 夹具须遵守既有完整焦点环准入：原生绘制的 ring、gap 与一个 backing pixel 均应在实际祖先 clip 内。Native075/080/084 原零 padding MediaPanel 中的全宽 Form slot 与 Panel 横界相同，完整焦点环无法容纳；084 实际记录原 prepared-focus 的 `NoFeasibleRegion`。这些运行保留为失败及负例，3 个 marked 向量和 positive 控制未达，不能因后续移动夹具而改签。

总协调选择 A，保留完整 ring 合同。Core 唯一 owner 修普通 Field 的基础目标为实际输入槽并按真实倍率恰一次包含完整 ring，保持有/无 Scroll 与 pointer/direct/Tab/AX 合同一致，保留 clipped slot、clipped ring、旧 full-width 配置负例。不得将裸输入槽通过称为完整焦点合法，也不放宽 Panel 曲线或 clip。

#204 独立新 profile `M6_IME_LEGAL_CONTAINMENT_A_R1` 在同一原 Host/Dialog 中组合 Form 与原 MediaPanel 为 siblings；使用原 Dialog 内边距，普通 Stack 仅保 Panel 为 flexible child，不添加 padding、强制焦点或复制组件状态。原 Media 资源与命令、Dialog 三 slots/Invoker/return focus 保持，原 IME 三 marked、七计数不逃逸和 Right32 Down/Up/Escape 控制全部保留。实际点击由当前 published label/frame 求得，不宣称旧点或 Viewer 几何不变；实际倍率、完整 ring 与 Dialog clip 必须在新运行记录。新源码/profile/compiled freeze/programme/输入来源与历史红例分开，Core TEMP 诊断不得进入正常功能候选。

本条仅同步已授权夹具与修复边界，源码审、编译、CPU 与 native 结果各自记录，不宣告 M6 或 #204 完成。stale retained native AX、M8 及原物理 IME/VO/性能/pinch 延期边界保持。


### Native095 focused programmatic-native IME 验证（2026-11-01）

新合法 A 夹具已用实际 AppKit selftest 完成原 composing-editor 子场景。实际编译 HEAD 为 `b71a369099fddb3dadf716acd950755c808ef7f7` 加已冻结 Viewer prepared-pointer 候选及 consumer native20445；CPU094 原生 example 定向 strict/build 均通过，独立归档后签名运行。Native095 唯一实例正常 exit0（6.174946s），全部进程组与原 GPU guard 已释放。

原编辑器通过当前 published label/实际点取得焦点与原生 caret；actual scale2、token width2/gap2/physical AA1 推得保留带4.5，输入槽 `(22,80,436,32)` 的完整 ring `(17.5,75.5,445,41)` 独立重算后完整位于 Dialog 实际 clip `(10,24,460,696)`。三个 marked Tab/Right/Escape 均记录同 ID Started/Finished、composing-at-start 和 Host Editor/InputConsumed；七按钮计数、Viewer transform/status、焦点及 Dialog 全部前后不变。随后原 published Viewer 点击实际取得20401焦点与 capture，Up释放；原普通 Right Down 平移恰−32pt，Up无额外变化；普通 Escape Down 关闭 Dialog 并恢复20461，Up不重复，原 ExpectNoText 与最终关闭断言全部通过。未改变原脚本、坐标推导或断言。

证据：`issue-204/m6-legal-containment-a-20261101-r1/native-pointer-r3-run01/DOMAIN-REVIEW.json` SHA `07a8c94a0356b5ab78e5b1cb3a72f99836e9a82130a0a7682131708df96a2617`；Main 独立签收 `ui-main-Native095-pass-release-acceptance.json` SHA `989adafe7f03c63cc6ec555abd2698d03297b863c3834a1cdcf1ba9df47dc26d`。该通过只覆盖 focused composing-editor 的 programmatic native 输入及原 positive controls，不代表物理 IME、候选窗或完整视觉验收。retained stale native AX、M8 与原 VoiceOver/性能/pinch 延期仍保持；075/080/084 原夹具红、086 无效几何红、090 Viewer 拒焦点红继续保留，不能由新 profile 通过改签。


Public candidate formatting: Rust 1.95.0 pinned rustfmt produces native SHA `6b3a2be2884fb0b996edd4f17ac99e902bf3c67a4280a1ac03bb111b8cee39c2`. Per-hunk token-tree review confirms only whitespace and ten optional trailing commas in existing calls or two-element tuples; actions, times, literals and assertions are unchanged. Native095 compiled source remains SHA `20445b260a3229fbac40a297b367ef69b85e846f01025fa14b488d441bc0acf4`; the formatted candidate has not been rebuilt or rerun.


## 15. 2026-11-01 MediaViewer prepared primary pointer admission

Native090 proves the valid sibling Form's complete ring/caret and three native
marked-key frames with unchanged original seven counters/Viewer state. The
subsequent original Viewer click hits the current Viewer, but does not acquire
focus/capture; Right stays with the editor and the original positive32 assertion
fails. This remains an actual failure. Its unrecorded internal error and Viewer
witness must not be invented. The earlier missing-Dialog-passthrough hypothesis
was withdrawn: the compiled original Dialog already supplies that contract.

The frozen Viewer declares Recursive curved-subtree support and required focus
reveal, but lacks an original prepared action contract. In a curved Host its
Unsupported primary Down cannot use the ordinary fallback: reveal requirements
exclude the legacy route, and Recursive nondelegated geometry is not Local.
The component now prepares only its original primary PointerDown decision using
the existing original-owner action/effect adapter. Current readiness, viewport,
transform, focus evidence and prior drag authenticate the decision; the Host
still owns mount/epoch/current clip/full witness/IME/focus/capture admission.
After those checks and focus commit, the effect only installs the decided
original drag. Ordinary Down consumes the same decision. Loading/unavailable
remain inert; Ready Fit consumes/focuses without capture; pannable Ready acquires
one original capture. Move/Up/Cancel, image ownership/loading, transform/key
commands and painting remain unchanged. No shared gate, table transaction,
public API, custom layout or AX verb is added.

Two CPU regressions cover pure preparation and unchanged Loading focus/capture.
One explicitly ignored, mandatory real-Renderer-resource regression uses an
authentic registered bitmap and actual Host primary Down inside a Panel: full
focus witness and current hit precede focus/capture, Up adds no transform,
RightDown pans32 and Up adds none, Fit focuses without capture, and a legal
point with insufficient complete witness refuses without focus/capture/transform
or notification mutation. The same test must be run on old production plus new
tests and on the candidate under separate Main CPU/build and GPU grants. A real
BitmapImageLease requires the Renderer/device; Loading or fabricated Ready is
not a public Ready-pointer CPU proof. No pixel/visual or physical-IME acceptance
is inferred from this resource/input test. Source checks are not actual red/green.

Main integrates only this component and design append over the admitted Core A
R3/native A fixture. New programmes/profile hashes and explicit GPU execution
of the ignored case are required before the unchanged native M6 positive chain
is revalidated. Original075/080/084/090 failures, CPU086 invalid red and the
separate Core087/088 evidence remain immutable and separately qualified.

### Actual affected verification and Native095 result

This fix is a complete tested delivery batch over public baseline
`b71a369099fddb3dadf716acd950755c808ef7f7`. The baseline component is
`0315bbb9…`; the tested component is exactly `185f9c89…`. Main CPU091 compiled
old production plus identical corrected regression tests (`23e33512…`) with
`--no-run` and immediately archived it; no baseline CPU test was executed.
CPU092 compiled and archived the candidate, then ran the exact Content module:
15 passed, 0 failed, 5 explicitly ignored GPU tests (465 other tests filtered),
and the original scoped strict check passed. The five ignored cases are not
five GPU passes. Unchanged Core087/088 and geometry evidence were reused.

GPU093 explicitly executed the same ignored Ready-pointer case on both retained
programmes. Old `422820a8…` reached the designated Ready Down focus assertion at
line2128 (actual ComponentId7703, expected7701): exit101,0passed,1failed,0ignored.
New `b504cee1…` passed the complete original Renderer-registration/Host case:
exit0,1passed,0failed,0ignored. Each programme remains bound to its own complete
compiled source freeze; the current execution freeze is a separate identity.
The case includes primary Down focus/capture, Up release, RightDown32 and no
RightUp duplicate, Fit focus without capture, and insufficient-witness atomic
refusal. It does not render or certify a pixel matrix.

CPU094 scoped native strict/build/archive passed on actual compiled HEAD b71,
with the admitted Core A R3 and consumer `20445b26…`: native archive `e427c9ca…`,
identity `776b5db1…`, full freeze `08e5c638…`, profile `e8b75027…`. The signed
payload `7358ef80…` is a derivation of that archive, not another compilation.
Native095 then ran once under a new grant and exited0 in6.174946s. The actual
Editor complete ring (17.5,75.5,445,41), recomputed from scale2 and token
width2/gap2/one physical AA pixel, fits the Dialog clip (10,24,460,696). Three
composing Tab/Right/Escape transactions retained the same Editor target and
InputConsumed result, with all seven counters and the complete Viewer/focus/
Dialog witness unchanged. The original positive chain acquired Viewer20401
focus/capture by the published primary pointer, released on Up, dispatched
ordinary Right Down by exactly−32 with no Up delta, then ordinary Escape closed
the Dialog, restored20461 and passed the original ExpectNoText assertion.
Down geometry45 and Up46 were separately current; equal generations are not an
oracle. No separate pre-Right None query was added: ordinary routing supports
the unpublished-editor conclusion, while explicit None was checked after Escape.

Main's Native095 pass/release acceptance is
`artifacts/issue-204/m6-legal-containment-a-20261101-r1/native-pointer-r3-run01/ui-main-Native095-pass-release-acceptance.json`
(SHA989adafe…). Owned process/group retirement and the original paired GPU
guard were accepted; no retry or unused-window reuse occurred. This evidence
completes this prepared-pointer regression fix and the focused programmatic
native IME/positive-control subcase only. Native090 and all earlier failures
remain immutable. Whole M6 retained stale native AX, M8 real OS gestures and
external-display conditions, physical IME, VoiceOver, performance and pinch
remain independently open/deferred. They do not block this compatible ordinary
source delivery; merge/CI/publication remain Main's responsibility.

## 16. Native096/098 disabled Actual 原因说明的有限验证（2026-11-01）

Native096 使用 CPU094/b71/native20445 的原归档程序，实际 Fit=2.4、Actual 所需 relative scale=1/2.4，低于允许下限1。禁用状态正确，但原可见说明和 Actual AX Help 均缺少原因，保持 FAIL。consumer 仅在原 `refresh_status` 复用 Ready/原 Fit 与 Actual 可用性及公开 transform，投影同一原因到 STATUS 和原 Button Help，不改变几何、命令阈值或 IME 脚本。Fit 已放大图像时明确说明不能缩小到 Fit 以下；另一个范围分支说明 Actual 超出最大8倍 Fit。Loading/Failed/Missing 文案保留，条件消失时清除 Help。

CPU097 在 actual HEAD `d48a4f8c7789a1f29aea1f192d4ad7054188958f`、native6b3 和新 fixturee217 输入上完成定向 strict/build/即时归档，Main 接受收据 `6009dcc8eb488bb05b19d454c64ef0414c17d2237fc16f9b494f4ffd5bb70583`。新归档 `fbe6c69f2e717c7808b0dcabb3e21ee093b8eb9328c6ce765725057141f8775b` 经独立签名派生为 `4cf1ff93d24899f0528b3c3e353e440b6031f8913448a673a20ceca04db24f63`，不是旧 CPU094 程序的新标签。Native098 唯一正常窗口只读 fresh AX/完整 own-window 截图后使用系统窗口关闭按钮，exit0/21.368836s；原 ownPID/PG 和 GPU guard 均已释放，Main 独立接受有限结果与释放收据 SHA `18bb35bc349ae7401952dfa1e195380d1c5a16cbac81d25ac05fe3f0ea7b188e`。

本次 Fit>1 条件下，Actual 禁用、完整可读且未裁切的两行原因说明、Actual AX Help 中完全相同的原因，分别获得有限 PASS。长说明实际换行使 viewport 从 `(0,188,480,532)` 变为 `(0,204,480,516)`，Fit 的 y 平移由146变为138；relative scale1、absolute scale2.4、x平移0不变，最终 `Fit=min(480/200,516/100)=2.4`、`qActual=1/2.4`。平移符合正常 Fit 居中 `(516-240)/2=138`，但确为一次真实变化。严格 whole-observation transform-unchanged 判据为 **NOT_MET**，不得以剔除启动帧或事后改判得到无条件 case PASS。日志在 render 前记录 state/drain、render 后记录 viewport，不把相邻的新旧布局日志误配为同时刻错误。未建立 stable pre-read baseline，也未将原 normal 日志仅有的 Close/Retry=0 冒称七计数测量。

#203 对冻结 geometry 的补充核对确认：Fit 模式在 viewport 分配变化时按原 `set_viewport` / `fit_translation` 合同重新居中，Content arrange 使用实际 frame.size；无需新增 Pan/Zoom 命令或修改 geometry。补充收据 SHA `e30cb2a7d4fd4ecb6472d597e8be69d8d10ca3d1b9cc74718564f017fb464f21`。该原因说明不改签 strict unchanged 门。

证据：`issue-204/disabled-actual-copy-fix-20261101-r1/normal-readonly/run01/DOMAIN-REVIEW.json` SHA `19097512bdc863fa145afc7db91e6f01abce5913dd48f59d6fc19c8c4b4e7974`；#203 actual review SHA `b2dcaf6df3508537229892a57401f18e248db963642ab83d6f9d4500dfe04946`，结论为 finite copy why PASS / strict unchanged NOT_MET。原096 FAIL和095 composing-editor子场景 PASS 保留。另一 qActual>8 分支仅有源码合同审，未做原生窗口验证；enabled Actual、retained AX、其余 M8 与原延期边界仍开放，无额外运行或整票通过。
