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
