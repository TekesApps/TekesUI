# Passive original Content hover preview (#46)

The effective acceptance criteria remain in [Issue #46](https://github.com/TekesApps/TekesUI/issues/46).
This batch follows the original context-menu owner/mount work in #44 and reuses
existing PopoverContent/PopoverView layout, shell, theme, original Content paint
and help routing. It does not introduce another ContentHost or trigger.

`HoverPreviewContent` owns a single noninteractive original Content subtree in
its existing Popover shell, with the built-in Dismiss omitted. It never opens
an overlay input scope, exposes a focus target, routes pointer input or adds an
editor. Interactive descendants are rejected when the subtree enters the slot.
The normal ContentHost tree still validates changes made through mutable Content
access before event/layout publication. The passive shell is visible in the
original semantic tree only while open; its text children retain their hierarchy.

`bind_hover_preview(owner, preview)` captures the original anchor/preview mounts
and returns an opaque `HoverPreviewVersion`. Pointer enter/change/leave uses the
same clipped/curved topmost-owner hit path as context menus; this small helper is
shared in help.rs. The host retains the anchor frame. Movement/clipping, owner
retirement/remount, scrolling, pointer gestures, window deactivation, capture or
an interactive overlay retire the passive preview. A stale original mount cannot
request or update a replacement row.

`update_hover_preview(version, body)` validates original mounts, exact current
version, passive content and tree identities before replacement. It returns the
next opaque version. A static hovering pointer can display accepted streaming
updates after normal host layout; delayed old-version updates are refused.
Escape/click/scroll suppression survives data updates until an actual leave and
new enter. The library does not convert updates into fresh hover requests.

Hover and accepted data updates leave the original editor focus, selection and
marked text untouched. Ordinary editing keys and native text callbacks retain
the existing route. Escape dismisses the preview; a composing editor keeps the
Escape route so the input method can handle it. A click through the card retires
the preview and reaches the original underlying document/control in the same
host; the preview never captures it.

`PopoverPlacement` adds RightStart/RightCenter/RightEnd preferences to the existing
shell. They flip left if it fits and otherwise clamp to the actual viewport/clip.
The default Below path calls the unchanged existing placement function. The
shell owns all placement; consumers provide content constraints and preferences.
No caller supplies popup coordinates or scroll compensation.

The independent public consumer provides two original text layers with the
existing #31 LabelOptions (2 input / 4 output lines, tail ellipsis) and a stock
TextFieldContent Composer. Controls and consumer regressions cover passthrough,
version updates, marked text, Escape, owner changes, clipping/scroll/activation,
interactive-content refusal and placement. These source tests now pass with the
related affected checks: controls83, Popover/Tooltip9 and independent consumer50,
with affected strict compilation checks. The scoped142 passing tests are not a
repository-wide CI result. Compilation used the coordinator-allocated original
source/cache window. Physical Chinese IME, VoiceOver and GPU pixels remain
separate evidence categories and must not be inferred from software-native checks.
Issue #46 remains open until all effective original acceptance criteria pass.

Candidate adaptation (2026-10-30): the isolated source now incorporates the
accepted #42/#30/#44 baseline `2c701999957fe1d41868ea62cab55522c36454c3`
(PR #185). The #46 delta preserves final #44 native ShowMenu admission,
owner/mount retirement and ordinary window-margin dismissal, including the
Control-primary context trigger exception. The shared anchor hit-path extraction
changes no final context-menu eligibility or dismissal policy. The original
frozen candidate and its evidence remain retained. Only source merge, explicit
formatting and patch checks were performed at that historical adaptation stage.
Current scoped and software-native evidence is recorded below; physical and
published-package acceptance remains pending.

The native consumer `examples/composition-consumer/examples/hover_preview.rs`
uses the existing CoreText backend, native key/pointer adapters, renderer and
one original ContentHost. Its opt-in fixed 26-checkpoint review uses stock
horizontal/vertical StackContent, ScrollContent, original turn Buttons and the
same TextFieldContent Composer. Native pointer movement, real button click,
scroll/resize/miniaturize callbacks and synthetic NSTextInputClient preedit have
separate raw journals. It checks original marked text across hover/static stream,
Escape suppression, click-through onto the original B under the passive card,
A/B/version refusal, clip/removal/remount/rebind and narrow viewport cancellation.
The program and guarded runner must be built and reviewed before scheduling a
native window. These are software-native checks, not physical IME or VoiceOver.

Native review driver correction: `AXReplace` is the real selected-text setter,
so full-document reset explicitly selects the known UTF-16 range first. Both
Chinese-marked and unmarked setup checkpoints independently assert exact literal
text/ranges before any later baseline-preservation result can pass. Native resize
can replace tracking areas and emit mouseExited; this is a genuine leave that
retires the preview. The narrow-viewport test explicitly leaves and re-enters
after resize and retains its open/subtree/viewport assertions. SDK retirement
policy is unchanged. Partial native runs must audit reached setup checkpoints
independently even when a later process abort prevents complete-run aggregation.

Current software-native evidence (2026-10-30): the complete guarded preview run
`hover-native-reset-resize-02-runner/attempts/run-20261029T180952.479533Z-5224c13a`
passed all26 fixed checkpoints, exit0/no timeout, with483 native callbacks and475
state records. Program SHA256 is
`60f5c36ac7b57bd795cd4a4f6ff6f53108c3dc9aed3d8856b5302a335f404766`.
The synthetic Chinese marked setup and exact unmarked reset are independently
asserted in source and in the runner, including reached-checkpoint audits if a
later process aborts. The prior failed attempt and its wrong plain baseline are
preserved and are not acceptance for that input. The actual320x260 native viewport
check passed after a genuine leave/reentry; short B content fits to the right,
so this run is not proof of native flip/clamp activation. Explicit placement,
flip and clamp are covered by the controls tests.

The extracted original context-menu hit path also passed its current-source
23-checkpoint/4AX native regression with program
`37fbd9ae5d692cbfffa60274ff192d8affa2e391b1b6802ec324bcd61d3743e2`.
Evidence lives under `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-46/domain-20261030`.
Physical ChineseIME, VoiceOver, GPU pixels, performance, published standalone
consumer/40SHA and coordinator integration CI/delivery remain unverified. This
bounded software evidence does not close Issue46.
