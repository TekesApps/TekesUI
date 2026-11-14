# Shared control visual polish — 2026-11-08

Tracking: [Issue #245](https://github.com/TekesApps/TekesUI/issues/245).

## Menu and Tab selection

Menu rows and Tabs retain their original ButtonModel interaction, geometry,
Surface fill/text/icon states and independent keyboard focus. Their selected
background is a fill without a resting enclosing stroke, including hover,
pressed and disabled selection. The outer popup keeps its original decorative
boundary, clipping and elevation. Explicit errors retain their error boundary.

This is a component-owned presentation choice. It does not change the global
Quiet action state rules, selection semantics, global focus metrics, input
routing, capture, or any consumer-specific Gallery paint. MenuRuntime opts its
original row buttons into `set_selection_fill_only(true)`; Tabs uses the same
small internal option around its original controls.

The shared Button renderer applies this option while producing its original
Quad. It does not erase scene primitives after painting or copy Menu/Tab input
implementations. Ordinary actions keep their existing default rendering.

## Input-only attached focus boundary

The earlier TextSurface Field paint deliberately combined one 1-point input
boundary with an independent 2-point ring and a 2-point outer gap. FieldView
only paints metadata; no repeated field-frame/focus call was found. At 2x,
that separate ring alone used four physical pixels. The cause was this intended
geometry and its existing theme/brand ink, not a duplicated paint bug.

The same-machine focused AppKit NSTextField screenshot was directly reviewed:
`work/appkit-visual-20261108/focused.png` in the coordination workspace. It shows
an attached light-blue focus contour. A reliable unfocused screenshot was not
obtained, so this is a focused-state structural comparison only. The change
adopts an attached single contour; it does not claim exact AppKit color or
pixel reproduction.

For `TextSurfacePresentation::Field`, editing focus now changes the original
input Quad's border to the existing `c-focus-width` and `border-focused` color,
without an extra ring or outward gap. Error ink retains `status-failed` priority
on that same contour; focus still changes its width. Read-only fields keep their
existing background and selection capability. Disabled inputs retain the
ordinary edge and do not gain a focus indication. The original radius, hit
geometry, content, caret and ancestor clipping remain.

Undecorated document Content and other controls retain their original
independent focus rings, except the later Radio-specific contract below.
Global focus tokens and global Quiet rules are unchanged.
The existing conservative field focus-space admission/reservation remains for
this finite batch; it can require more space than the new attached ink and is
not a claim that the attached border paints outside its frame.

## Verification scope

Menu coverage checks original production Scene output for selected/hover/
pressed/disabled rows, 1x/2x backing and every built-in theme (including
increased contrast), the untouched popup boundary and a single keyboard focus
ring. Existing selection, stable command identity, capture and overflow tests
remain applicable. A separate check protects ordinary actions and error
boundaries from this component option. Input Scene checks cover all built-in
themes, focused/rest, editable/read-only/disabled/error, 1x/2x and ancestor
clipping, asserting one original frame contour and unchanged input state. The
real CoreText public-consumer theme test now uses the attached input-focus
contract; its text, state and contrast assertions remain. Tests have been added;
execution and native appearance acceptance are recorded separately after the
controlled build. Light/dark and error/disabled appearance still require review.

The three affected Gallery theme tests (TextField, TextArea and Form) now assert
that their actual Field surfaces use the attached focus edge. Their original
native value, selection, scroll, persistence, retry and text-contrast assertions
remain. These fixtures are decorated Field presentation; their changes do not
alter the independent focus-ring contract for undecorated document Content.

## Navigation List opt-in

The Gallery template's two grouped navigation lists use the original
SelectableListContent/ListView. That legacy string-row path does not have a
Surface override, so it still inherited the Quiet selected border after the
Menu/Tabs change. ListView now exposes the explicit
`set_navigation_selection_fill_only(bool)` option, default false. It forwards
only selection presentation to each original prepared-segment ButtonModel.
The old renderer honors this choice while computing its own original Quad;
its existing fill/motion/disabled/text computation is unchanged. No alternate
Surface, data model, interaction implementation or style framework is added.

The Gallery consumer opts its two groups in at assembly. Ordinary List, Tree
and DataTable defaults are untouched. List keyboard focus remains its original
independent `paint_collection_focus` scope. The independent public `list_theme`
consumer runs its original selection/value/hover/disabled/contrast/keyboard
checks with both the default and opted-in view and checks the exact row-border
contract in each case. The option changes neither row geometry nor command
identity.

## Unified interaction states — 2026-11-09

User-authorized continuation of #245: keep the existing Theme schema, generated
Surface/use/state packets and all registered component consumers. Diagnose the
Toolbar pointer-to-state-to-paint path before changing colors. No Gallery-only
color overrides or parallel theme tables.

Normal decoration should recede behind content; hover and pressed must remain
visibly distinct, selection persists independently, and keyboard focus remains
independent. Checked/selected is applicable only to controls that own selection.
Disabled controls remain noninteractive. Existing ordinary and increased-contrast
readability thresholds, including #206 Quiet disabled labels, remain unchanged.
Shared palette adjustments must enumerate changed roles/modes and affected
consumers; historical fixtures and prior native evidence remain immutable.

Source coverage, rendered state tests, native observation and publication are
separate results. This section records the intended contract; completed checks are recorded below
and exact program identities are retained with the Issue #245 receipts.

The Toolbar defect is repeated focus synchronization during paint: sending
`set_focus(false)` to an already-unfocused Button calls `blur()` and destroys
its independent pointer hover. Synchronization now blurs only the actual former
focus owner; it still updates the current owner's keyboard-focus visibility.
Hover/pressed colors continue to come from the existing Surface packet.

Ordinary dark `p-tertiary-text` changes from #B4B4B0 to #9F9F9B.
Light stays #606060: a lighter meaningful adjustment fails the existing
tertiary/pressed and selection-hover contrast contract. This reduces dark
auxiliary/disabled emphasis; the old
dark disabled ink was brighter than normal secondary text. Increased-contrast
values and explicit per-theme overrides are unchanged. Quiet disabled labels
still meet 4.5:1, or 7:1 in increased contrast. The immutable v1.6.1 fixture is
retained; only precisely mapped ordinary-dark tertiary channels receive the documented delta.
The Rust external-theme acceptance boundary is aligned with the same existing
Python Quiet-disabled contrast rule, including rejection of an invalid packet.

Local batch 023 passed 9 Toolbar tests (including the new actual Scene
regressions), 55 design-export tests, 32 token tests, strict clippy for the five
affected crates, and the versioned Gallery release build. Python Theme tests
passed 51 cases; the full 13-brand compiler, generated artifacts, historical
package/goldens, composition audit and formatting were checked. Native results
are recorded separately in Issue #245; these checks do not certify every
component's visual appearance.

The broader Gallery regression found the unchanged historical Tokens dark
snapshot expected the previous tertiary swatch. The historical test now builds
a separate reference-color Scene: exactly one known Tertiary swatch and three
NOTE_STYLE glyph runs in ordinary Dark recover the old color. Actual current
Scene is also rendered and must differ only through those asserted semantic
channels. Geometry, primitive ordering, historical PNGs and the original
radius/AA pixel tolerances are retained. The 24-image regression and strict
Gallery clippy pass; the other 511 Gallery tests and all 456 Controls tests
passed on the same product sources and are reused.

## AppHost standard-dark follow-up — 2026-11-09

The former fixed-Light AppHost limitation is resolved by initial Theme inheritance
from Gallery. Host-owned glyphs use the original Button/IconContent state-color
path and toolbar measurement uses the same Palette as final paint. Palette values,
arbitrary image colors and the two existing collapsed-panel shapes are unchanged.
Native CU2 standard-dark review at real 2x covered the complete AppHost page,
Toolbar hover/held press/release/leave, Focus/Others exchange, sidebar collapse and
expand glyphs, selected Tabs, filled Menu rows and attached Field focus. This is
scoped native acceptance, not a whole-Gold7 or all-theme certification.

The first publication CI exposed three stale 177-icon assertions (the canonical
catalog now has 179) and a deferred-help test that conflated button hover animation
with help timing. The repairs preserve all 51 historical proposals and 29 distinct
historical artwork comparisons, explicitly leave the two new templates without a
fabricated historical baseline, and settle the real hover animation before asserting
that deferred help does not replay buttons. All 10 catalog and 22 content-help tests
pass. No production or palette change was made for these test repairs; the accepted
native program remains the same. AppKit Menu/Tab reference records from the original
Issue scope remain separate outstanding work; the existing native Field comparison
and all prior artifacts remain retained.


## Transparent Tab close cross — 2026-11-09

New user decision under #245 supersedes the earlier circle-and-cross artwork
choice recorded in `tabs-close-20261108.md`. That historical decision and its
original evidence remain preserved. A Tab close affordance now uses the existing
canonical `tekes-remove-one` plain cross, centered at the same standard icon size
inside its original independent close hit target.

The original ButtonModel prepares the same layout, pointer capture, motion and
keyboard activation. A crate-private transparent content-frame paint entry skips
only the close button's Surface background and boundary; no Scene primitives are
removed after rendering. Hover/pressed/disabled foreground continues to resolve
from the existing Quiet Surface packet. Independent visible keyboard focus is
still painted. The enclosing Tab retains its own fill and selected state.
Ordinary Button entry points keep their existing background painting.

The original independent `tabs_close` consumer now checks a plain canonical
cross, its unchanged centered artboard and larger hit target, no close-frame
background in rest/hover/pressed/disabled across all four Themes, retained Tab
selection fill, exact semantic foreground, and visible close keyboard focus.
Original once-only pointer/AX/keyboard, disabled, IME, reorder, close/reopen and
resource-failure checks remain. Execution/native review is recorded separately
against the frozen candidate; source changes alone do not claim those results.

## Cross-container Toolbar hover — 2026-11-09

Pointer hover ownership is independent of the toolbar selected for keyboard routing.
When a plain pointer move crosses container toolbars, the previous hover owner
receives the same outside point before the new owner receives it. Only that prior
host is notified; unrelated or hidden toolbars do not receive a broadcast. Existing
menu, pointer-capture and body/splitter drag ownership stays authoritative. A hover
exit clears pointer presentation without blurring keyboard focus; whole-input
cancel retains its original focus-clearing contract.

Both ordinary A-to-B movement and a keyboard-routing interleave reproduced the
stale hover in the original Host render path. These are distinct from v0.3.4
semantic icon tint and its same-container native leave observation. Raw failures
and the corrected test fixture prerequisites remain recorded separately.

## 2026-11-09：普通主题边界降噪与实际画面验收

用户反馈实际 Light AppHost 仍显强对比；此前候选只改布局/交互，没有调整 Light 配色。共享普通主题的必要边界从 Light #737370 调为 #7C7C79、Dark #A3A39E 调为 #7E7E7A；焦点从 Light #42423F 调为 #5E5E5B、Dark #EEEEEA 调为 #B0B0AC。必要边界仍需对全部已声明相邻背景达到 3:1，焦点保持可辨；文字、选中/hover/pressed 绑定及 Increased Contrast 不变。装饰性分隔线继续使用 separator，不能拿必要控件边界角色画每个装饰框。

颜色选择须通过原品牌/Theme 对比度门槛，并在同一 AppHost 的主题、字体、点尺寸、backing scale 下比较原生前后画面。静态 token 完整、生成一致或测试通过均不代签视觉效果；公开发布/Dock 更新单独确认。

Settings 原生 Theme 首次鼠标点击的 Down/Up 已到达 Form 却未打开菜单，作为 #229/#245 的真实行为缺陷处理；配色变更不代签此修复。

### Settings MenuButton 首击修复

`MenuContent` 原 prepared-pointer hook 对 Down 返回 no-op，跳过内部 Button 的真实按下状态。现在由 MenuContent 通过既有 `prepare_original` 包装原 ButtonModel 决策，在 Host 的动作/焦点准入全部通过后提交；保留 Press/Release 激活阶段及原 Up 打开菜单路径，不采用绕过原子准入的 legacy fallback。真实 ContentHost Down→Up 红例已转绿；组字 owner 拒绝转移时，菜单不得进入 pressed/capture 或打开。对应 #229/#245。

本地 20 项菜单回归、457 项 Controls 原有回归及新准入回归、32 项 tokens、110 项 AppHost 和 affected strict 已通过。历史24图只还原明确批准的普通边界/焦点色和此前 tertiary 修订后比较，原PNG、几何/文本及误差门槛保留。原生首击/主题切换和视觉效果由最终候选独立验收。

## 2026-11-09 用户修订：Theme 单选与菜单自然宽度

General 的 Theme 直接展示 System／Light／Dark 单选组，复用原 RadioGroupContent，三个选项互斥；TOML 的 enum 字符串及选项值、保存/Undo/Redo/加载合同不变。字段可声明 `presentation = "radio"`；未声明时保留原自动选择（enum仍Menu），不按字段ID硬编码、不改变其他消费者的枚举呈现。Radio组选中与历史恢复必须通过原组件及既有草稿/存储链；标签对齐首个选项。

共享弹出菜单默认宽度由最长实际行内容、Body按钮token内边距、check/submenu列和focus gutter决定；280pt是上限，不是默认填满宽度。长文本按最终可用宽度重测行高，保留viewport clamp、header约束和原命中/滚动。原native004菜单程序/源及Dark保存证据冻结保留，不能代签新Radio界面。

自然宽度下，首次声明 `checked: Some(false)` 会预留真实check列，允许比无列菜单更宽；同一列 `false/true` 切换必须保持尺寸与位置。旧固定宽度断言不能继续要求新列占零空间；原CI失败保留，新消费者回归分别覆盖列声明和勾选切换。

Radio的prepared pointer入口按现有事件合同归一化原生Primary Down，保持原model/view和焦点事务；Secondary不选项。实际SettingsColumn/LocalContainer回归覆盖原生Down/Up，不用旧式事件代签。

## 2026-11-09 后续共享控件布局修正

单行TextField显式 `.scrollbar(true/false)` 控制水平指示条，缺省false；实际水平滚动及插入点reveal仍采用原Auto策略，多行垂直策略不改变。NumberField在减号/原编辑器/加号之间使用Spacing::Tight，两侧一致；狭窄分配下按可用空间收缩，不改变数值/保存/输入语义。RadioGroupContent增加基于原RadioGroupView的Axis选项，默认Vertical，Settings选择Horizontal并沿原value列右对齐。所有新方向几何共用原rows/indicator/label与输入和AX投影，不能只改画面。

## Radio attached focus — 2026-11-13 (#245, T82)

The Radio indicator retains its original circle and selected dot. Keyboard-visible
focus changes that circle's existing inward border to `c-focus-width` and the
existing focus ink, without drawing a detached outer ring or gap. Error ink keeps
priority on the same contour; disabled indicators retain their original ordinary
border and marker. Pointer-only focus does not add keyboard decoration. Light,
Dark and increased-contrast themes use the same component-owned geometry and
existing theme values. Layout, hit targets, selection and roving keyboard focus
are unchanged; Settings does not override the shared component.

Both the public Radio primitive and the Surface-aware RadioGroupView follow this
contract. Scene tests verify actual indicator/marker geometry and enabled,
disabled, error and keyboard-focus states; original selection/input checks remain.
Native appearance and input observations are recorded separately, not inferred
from CPU results. The original detached-ring failure is retained.


### 普通主题共享中性色值微调（2026-11-14，#245）

按用户要求，本批只修改既有 token 值，不改变角色、组件逻辑、状态合同或对比度门槛。`border`、`border-focused`、`separator` 与按钮/选中底色联动减弱普通 Light/Dark 的装饰分量；ScrollContent、TextSurface 和输入框沿原共享映射自动继承。表面阶梯、文字、错误/状态色、品牌覆盖和增强对比主题保持原值。普通边界最差声明配对仍至少 3:1。仅检查生成一致性及配色关系，构建外观候选；不重复功能测试、不新增 CI；不据静态检查宣称实际 Gold7 体验通过。精确旧/新值及候选证据保存在协调目录 `outputs/quiet-neutral-palette-20261114/`。


## 2026-11-14 滚动条低对比外观修订（#245 / T84）

用户在普通主题调色候选中仍观察滚动条过深，明确要求只比选中项背景稍强。CU 基线在 Light/Dark Gallery 内容区确认此现象。原 ScrollContent/TextSurface 共用必要输入边界 `border`，仅调该值无法同时满足两种视觉职责。新增共享 `p-scrollbar-thumb` / `scrollbar-thumb`，普通 Light #DCDCD9、Dark #363634；Dark 采用比选中底稍亮的对应层次。两处共享滚动绘制使用此角色，所有消费者继承，不改变布局、hover、滚动、捕获或输入。增强对比主题继续直接读取当前 palette 的 `border`，保留消费者覆盖和原可辨性。

这是按用户最新要求修订普通滚动条的视觉合同，不是纯色值微调：普通滚动条不再声明旧3:1；输入框必要边界、焦点、文字和增强对比的门槛保持。对应旧绘制断言仅同步角色与IC范围，未执行功能测试或CI，不以断言修改代替原生外观验收。本批只做生成/颜色接线检查、构建和明暗外观审阅；旧失败、旧程序和功能证据保留身份。


## 2026-11-14 内容角色、控件视觉重量与焦点泛化（#245）

状态：共享源码实现完成，Check299受影响19项、strict、Gallery release构建及生成/composition检查通过；官方CU已完成有限明暗外观核验，Dock已安装并运行同一签名候选；键盘焦点专项的缺口按下文保留。用户10:54截图否定了整页视觉已足够安静的推断，保留前一批已通过的滚动条有限结果，不将其扩展为全部Gold7通过。用户随后明确要求考虑泛化。

### 来源与适用边界

已查阅 Apple HIG 的 [Text fields](https://developer.apple.com/design/human-interface-guidelines/text-fields)、[Text views](https://developer.apple.com/design/human-interface-guidelines/text-views)、[Focus and selection](https://developer.apple.com/design/human-interface-guidelines/focus-and-selection)、[Color](https://developer.apple.com/design/human-interface-guidelines/color)、[Pop-up buttons](https://developer.apple.com/design/human-interface-guidelines/pop-up-buttons) 与 [Sliders](https://developer.apple.com/design/human-interface-guidelines/sliders)。Apple 区分短字段与长正文，使用按用途定义的颜色，并要求焦点可辨；“所有多行文本一律无边框”不是引用自 HIG 的规定，本项目按内容职责作下面的设计选择。

参考 skill 为 [HIG Doctor 的 foundations / controls](https://github.com/raintree-technology/hig-doctor/tree/main/skills) 和 [platform-design 的 macOS](https://github.com/ehmo/platform-design-skills/blob/main/skills/macos/SKILL.md)。本机条目只是目录索引，本次读取了上游实际指南，不声称安装了整套工具或运行其自动审计。仅采用相关桌面视觉规则；不复制全部示例、不改变已确认窗口布局、不据第三方清单增加本批VO/功能/CI门槛。平台规则以 Apple 原文为准，精确尺寸与色值以 TekesUI 的共享 token 合同为准。

### 已定位事实（源码87c6550）

- `gallery/apphost.rs::document_content` 已调用 `TextFieldContent::read_only_content`，正文是只读可选Content，并非可编辑多行Field。`TextSurface::draw` 的Content静止时无框，有焦点时画整块独立focus ring。
- `composition/adapters.rs::TextFieldContent::set_focus(f, _: bool)` 丢弃了Host已有的keyboard来源；因此鼠标取得正文焦点也会进入上述画框路径。这是共享组件状态接线问题，不能通过把全局border调透明来修。
- Inspector的标题/作者为TextField，Revision为NumberField，Published为Checkbox，Draft为Select，Priority为Slider；本页没有Radio。用户反馈的Radio另沿共享Radio与Settings消费者核对，不把Checkbox或Select误记为Radio。
- `SelectView::render_trigger` 固定使用 `ButtonVariant::Primary`，令普通值选择呈现为主操作按钮；现有Quiet/Panel风格可复用。此为选择控件默认语义问题，不能靠降低所有accent颜色补救。
- Radio已有 `focused && focus_visible` 条件、`border-focused` 与 `c-focus-width` 接线；指针与键盘焦点来源不同。缺少动态画面时不能将“鼠标点击未显示键盘框”认定为丢状态。仍需验证真实Tab焦点可辨。
- Slider已填轨道使用accent、hover/pressed用对应accent状态色，thumb用surface-0与border；轨道厚度为本地常量。视觉重量同时涉及语义、几何和色值，不能一概称token色值问题。

### 共享默认规则与最小落点

| 类别 | 默认视觉合同 | 共享责任与消费者边界 |
| --- | --- | --- |
| 文档正文 / Content | 静止和鼠标选择时不围整页大框；选区仍清晰，键盘进入时保留可辨的焦点提示 | 复用现有Content presentation；保留Host传来的键盘来源，分开逻辑焦点与focus-visible。不复制编辑器，不关闭选择、复制和文本服务 |
| 表单Field / NumberField | 用统一底面与轻轮廓表示可编辑区域；focus在原轮廓上清楚变化；错误有独立语义 | 先区分静止边界与焦点的视觉职责，再决定共享值。必要边界、文字、错误及IC标准不被默默降低；不新增Inspector专用颜色 |
| Select | 表示一个当前值，普通情况下不是强调动作按钮；菜单可发现，选项互斥与当前值清楚 | 优先在共享Select默认路径复用原Quiet/Panel与现有菜单指示机制；不要求每个Gallery/AppHost/Settings调用者设置variant |
| Radio / Checkbox | checked、hover、pressed、keyboard-focus、disabled各自独立；不能用checked冒充focus，也不能为去掉大外圈删除焦点 | 保留既有状态机及focus-visible来源；共享绘制消费相应状态，使用贴合控件的清楚提示，禁止重新加巨大外圈 |
| Slider | 保留数值位置、已填段与thumb的辨识，降低大圆和粗深轨道对正文的竞争 | 先核对既有slider语义与尺寸token，再在责任组件统一调整；不降低全局accent导致主操作/状态标记一起变化 |

ScrollArea只管滚动、裁切及滚动条，不猜测子项是否为正文/表单，也不替子项擦边框。AppHost只组合，Gallery只演示；内容角色由现成正式组件API表达。同一角色在普通页面、Settings和AppHost的默认结果应一致，不通过父容器名称、嵌套位置或消费者临时布尔补丁推断。只有存在已确认第二种业务语义时才增加明确variant，不增加任意颜色/边框开关包。

### 接续顺序与验收

1. 先修已经定位的共享接线：TextFieldContent保留keyboard焦点来源，使Content的鼠标选择不显示大框；Select默认触发器不固定Primary。保留文档选择/逻辑焦点及菜单实例、ID与命令路径。
2. 再以同一套角色表处理表单与Slider的视觉重量；不先在消费者零散写HEX，也不为视觉安静把正常/hover/selected/focus全部变成同色。
3. 验收只补改变的绘制合同：Content鼠标/键盘来源区别；Select静止/hover/打开的外观与原菜单身份；Radio真正键盘焦点与checked区别。复用原输入/滚动/菜单功能证据，不重跑全功能套件、不新增CI。
4. Light/Dark用同一页面、尺寸与字号比较；检查正文是否优先、默认控件是否安静、目标状态是否可辨。截图不能单独证明所有焦点变化。当前用户截图与源码定位是失败/设计证据，本节不标实现或原生验收通过。VO仍在后续版本。

本轮证据：协调目录 `outputs/inspector-gold7-design-20261114/`，用户原图已保留；前批滚动条与Dock身份继续使用原收据。本批将构建同一共享候选，由官方CU验收后更新Dock；CU2当前不可用。不把研究skill当成修复交付。

实施边界补充：Slider视觉尺寸使用共享 `c-slider-thumb-size`（16）与 `c-slider-track-size`（2），命中区域仍由原SliderView负责。Field正常边界保持原必要边界色和rule宽度，focus统一用原轮廓上的c-focus-width；没有把装饰hairline用于必要控件边界，故不宣称静止Field边框已经减轻。Select复用Quiet底色；其normal/increased boundary实际为none，不能宣称保留封闭轮廓。共享触发器补trailing展开提示，并为文字保留空间，不引入消费者专用样式。

Check299只补Content来源1项、Field轮廓1项、Slider4项、Select13项，strict与Gallery构建通过；普通Select共享trailing图标测量与裁切、文字role保留已纳入。旧功能通过结果保留身份，没有新增CI。构建冻结期间源码及原缓存标记不变。新候选 `v0.3.27 Shared Controls.app` 的源码/程序/实际Dock身份由协调目录 `outputs/shared-control-visual-20261114/` 记录；正式Release仍v0.3.26。

本批实际原生：Light/Dark正文拖选有选区、无整块框，Select普通底/下箭头及打开/Escape关闭、Field同轮廓focus变化、紧凑Slider已由root对截图复核。Settings已恢复System/100%。Dark Radio的AX键盘焦点到达但截图不能可靠区分checked/focus；正文键盘焦点未到达；纯hover无官方无按键移动接口，三项均未验通过，不扩成整个Gold7完成。Field静止轮廓本批未减轻。程序与截图保持同一签名SHA995b8b359507fb635f932d8baa5da0dcf67adb4e0b2cf78a81b88a049ab47ab9。

## 2026-11-14 Settings整行视觉分组（#245）

SettingsRow统一承接整行低对比底面、panel圆角与四边Regular留白，标签和对应控件落在同一底面；不把静止背景当selected/hover，不给各行使用强调色，不把背景写到AppHost/Gallery。背景采用共享c-settings-row-ground别名引用既有surface-0，不改变表面阶梯色值；行间复用既有间距。保持原child IDs、焦点/草稿/错误与滚动身份。只补受影响行几何/裁切绘制和本批已授权官方CU明暗外观，无全功能测试或新增CI。Apple Settings/grouped Form为参考；非声称所有macOS设置行必须交替背景。

本批Check300：3项SettingsRow几何/滚动身份/裁切绘制、strict及Gallery构建通过（构建184.35秒）；生成/composition/格式检查通过。原token metadata的unit误写color被校验拒绝，已改为规范sRGB，未放宽schema。官方CU两色完整截图由root复核，Theme/Font size底面与左右对应清楚，未见截断，主题切换实际生效；恢复System/100%。新Dock为本地v0.3.27 Settings Rows候选，SHA1812c73d7611428acdfc8b3494c34ac592c5b46ee0eaa64d2c3f2bf77cf9bec1。无新CI、无正式Release；以前键盘focus/hover缺项不由本批代签。证据：协调outputs/settings-row-background-20261114/。

### 用户参考图纠正：共享分组底面（r5，2026-11-14）

上段r4记录保留为历史：用户随后明确否决逐行独立卡片的外观，不能继续称当前外观已接受。以用户15:16的macOS System Settings截图为本批视觉参考：同一schema group共用连续底面，仅首尾外角圆润，组内用缩进装饰hairline分隔，组间Relaxed留白。Shared SettingsForm用既有Stack组织原SettingsRow，不新增输入层/临时容器；字段ID、控件与挂载身份不变。各行Regular内边距，表单共享最大宽560pt、随正文type scale增大并在窄窗口内收缩。共享c-settings-group-ground引用专属primitive：Light #F5F5F4、Dark #242423、Light IC #EEEEEB、Dark IC #30302E；这是被动分组，不改变必要控件边界、选择或焦点token。没有AppHost/Gallery局部样式补丁。只补受影响布局/绘制检查与明暗外观，不新增CI；IC仅静态解析，实际外观仍待验。

尺寸核对：对齐全局layout密度合同c-row-min-height下限与“content measurement wins”，行高取真实标签/控件高度加上下Regular并满足下限；不把所有字段强行压成固定行高。子项在实测行高内垂直居中。组内Stack为0间距，组间沿用Relaxed；分隔线复用decorative_border的物理像素与IC策略，不另写粗细/颜色。Check302三项行测量/身份/绘制通过，Check303共享Form分组几何及strict通过；保留新测试夹具缺label服务和误算滚动内容宽度的两次失败，修的是测试前提，未放宽产品合同。原有不受影响功能证据复用。明暗实际字号外观待本批CU。


### r6同批密度修正

r5分组结构已经由Light/Dark100%与Dark150%原图核对，无明显文字截断；用户进一步指出行高偏大、左右留白不足，故r5不能作为最终间距接受结论。原图/Check302–304保留。

当前采用共享语义space-grouped-form-row-inline→space-loose（16pt），space-grouped-form-row-block→space-tight（4pt），统一用于SettingsRow的测量、安排与分隔线缩进。不是改全局spacing primitive，也不是AppHost/Gallery局部常量。这对token仅适用于分组表单行的背景到标签/控件的内边距；不用于主界面留白、Container内容边距、List item或TextBox内部padding。整体行高随实测内容加8pt并满足c-row-min-height；100%下比r5减少8pt，上下对称、左右对称。较大字号和窄窗口继续按真实内容重排。

r6实际验收：Check305四项受影响行/分组检查、strict、release构建通过（构建181.88秒），无新CI。root已核对Light100%、Dark100%、Dark150%原图：组底/细线保持，左右留白增宽、行高减少，标签/控件居中，放大时减号/数值/加号可见且无明显截断。精确16/4由共享token与代码保证，截图不冒充pt测量。恢复System/100%，键鼠释放，Dock更新为本地v0.3.27 Settings Density，SHA04cd78be712ae6451e4e4944f7b8786a68e5958d0426925cecbd926720bad5f7。IC外观、Light150及旧键盘/hover缺项未被本批代签；正式Release仍v0.3.26。详见协调outputs/settings-density-background-20261114/。


### 分组表单行命名收窄（2026-11-14）

按用户最新决定，将原space-content-row-inline/block重命名为space-grouped-form-row-inline/block，限定为分组表单行语义。仍分别引用space-loose=16pt、space-tight=4pt；只更新SettingsRow既有引用及生成记录。共享spacing尺度不表示各组件采用相同padding；主界面、其他组件和原始档位均不变。本次仅命名/定义澄清，复用r6已有布局与原生证据，不运行功能测试、CI、构建或CU，不替换Dock程序。
