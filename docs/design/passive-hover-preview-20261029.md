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
The source and independent consumer in PR #186 were delivered at merged40SHA
`251110c705eb06fed23f9152444bf487ae2420c8`, with CI40929784687 successful
and checkpoint `checkpoint-20261030-02`. Current scoped/software-native evidence
is recorded below. Physical IME/VoiceOver and separate package-installation
acceptance are not established by that source delivery.

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

Prior software-native evidence (2026-10-30): the complete guarded preview run
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
PR #186 source/consumer delivery and its successful integration CI are complete
at the40SHA above. Physical ChineseIME, VoiceOver, GPU pixels, performance and
separate package-installation acceptance remain unverified. The placement
follow-up below has its own source/program/evidence identity and still requires
coordinator integration/delivery. Bounded software evidence does not close Issue46.


## Native placement activation follow-up

The separate opt-in `TEKESUI_HOVER_PREVIEW_NATIVE_SELFTEST=placement` fixture
retains the original26 review as mode `1`. It uses the same native host, public
passive preview, real CoreText labels and stock horizontal Stack. A passive
measured spacer changes only the original anchor allocation (width0/430/90pt);
long bounded two/four-line content produces real horizontal placement pressure.
No consumer popup position or overlay paint engine is added.

The seven ordered checkpoints46101..46107 record numerical anchor/popup/host
viewport geometry.46101 proves the preferred right side really fits and uses
the current6pt gap.46102 retires after anchor movement.46103 proves right
placement overflows while left fits and the actual popup is exactly left of the
original anchor.46104 retires after moving the anchor again.46105 uses a real
320x260 native window after actual leave/reentry and proves both preferred sides
overflow, the popup differs from either preferred x and its right edge equals
the host viewport edge. All open cases retain RightStart vertical alignment,
visible original anchor, original body/input/output nodes and no Dismiss/focus
owner.46106/7 verify Escape/leave retirement. Every phase requires the exact
original Composer text, selection0..8, no marked text, original focus and no
business activation. These are fixed test assertions, not positions fed to the
runtime. This follow-up now builds and passes strict checks; its exact same
archived program passed placement7 and the original26 native regression below.

### Physical IME and VoiceOver execution prerequisites

Physical tests are a separate coordinator-scheduled desktop window with an
operator physically using this Mac, no concurrent desktop writer, an identified
foreground app/program/source and the normal resource guards. The operator must
confirm a Chinese composition input source in macOS, a working keyboard/pointer
and the actual uncommitted candidate/marked range in the original Composer.
Run a manual launcher with automatic selftest scheduling disabled: the example
supports this by leaving the selftest environment variable unset. The existing
26/7 scripted guards are not manual acceptance launchers; an approved manual
launch/evidence/close recipe must be bound before that window. Type and select
physically, record OS-produced NSTextInputClient events plus operator actions,
then hover/leave the original turn with the physical pointer while comparing
original focus/text/selection/marked state. Synthetic Preedit/AXSelect or a
passing automation log cannot establish the physical-input provenance. Escape
must be compared to the same input method's ordinary composition route.

VoiceOver requires a separately allocated window, authorized activation of the
real macOS VoiceOver service and an operator who can confirm its speech/braille
output and navigation. Record the running program, enabled service, actual
VoiceOver commands/output, original Composer focus/selection, passive subtree
hierarchy and absence of a Dismiss/action/focus trap. AX selectors and semantic
node queries remain distinct evidence. Neither physical input-source/operator
availability nor VoiceOver activation/output has been verified in this batch;
no system setting is changed. Any necessary OS permission/service setup is an
explicit precondition of that scheduled manual window. These gaps keep their
existing acceptance status and do not block the independent placement fixture.

The placement spacer is a semantic-hidden layout-only component. Its initial
zero width is valid layout, but exporting it as a native AX node would invalidate
the complete raw AX packet under the existing positive-frame admission contract.
The first attempted placement run stopped before46101 because that invalid node
withdrew the Composer; it is not flip/clamp failure evidence. The fixture hides
only this decorative spacer; all original anchor/editor/preview nodes and fixed
placement assertions remain required. The source/strict failures and native raw
attempt are retained.


Placement software-native acceptance (2026-10-30): archived program
`cb834e06ef76acce3e572422836ee3619cedbc387d4bae4d5d4670985c5dc5b9`
passed all7 new checkpoints under the existing complete guards, exit0/no timeout,
with178 native callbacks and176 journal states. The actual247.890625pt-wide
popup preferred x93.495117 at right-fit. At anchor x454 the preferred right
x523.495117 overflowed the704pt viewport edge, while left x200.109375 fit and
matched actualpopup x: native flip was exercised. At anchor x114 in the real
320x260 window, right x183.495117 and left x-139.890625 both failed; actualpopup
x56.109375 plus width247.890625 equals the304pt host edge: native clamp was
exercised. Checkpoints46102/104 and raw states36/89 confirm original anchor
movement first retired the popup, before fresh native reentry reopened it.
The same program passed original26 again, with486 callbacks and478 states;
fixed Chinese-marked/plain text, ranges, focus, action and remount checks passed.

Raw attempts are `placement7-r2-runner/attempts/run-20261029T190541.261272Z-b0ef286d`
and `original26-r2-runner/attempts/run-20261029T190552.017397Z-19aaf965` under
`/Volumes/HP P900/DEV/TekesUI/artifacts/issue-46/placement-20261030`.
The initial AX admission attempt and strict failures remain preserved. No SDK
placement/admission/retirement rule was weakened. This fills the bounded native
flip/clamp activation gap; actual physical IME, VoiceOver and pixel acceptance
remain separate, and their execution prerequisites above still apply.

For the still-open physical/VoiceOver criteria in #44, use its delivered
`context_menu` native example under the same separately bound manual launcher,
with `TEKESUI_CONTEXT_MENU_NATIVE_SELFTEST` unset. Real unmarked secondary/
Control-primary input on original B must keep A's draft/selection/focus and emit
only original B's value. A physically active Chinese marked session must exercise
the existing Composing refusal without changing the original input. Actual
VoiceOver must discover/invoke the original owner's ShowMenu action and navigate
real menu roles/states/items; Press/AXShowMenu programmatic selectors alone are
not VoiceOver operation. These are executable manual prerequisites/checks for
that later window, not results of the placement7 or synthetic26 runs.
