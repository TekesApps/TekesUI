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

Undecorated document Content and all non-input controls retain their original
independent focus rings. Global focus tokens and global Quiet rules are unchanged.
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
