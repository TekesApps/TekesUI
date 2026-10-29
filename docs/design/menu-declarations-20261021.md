# Issue 36 — original controlled menu declarations

This increment fills the original Menu's public component contract. It does not add an AgentKit preference model or another menu engine. The first event-capture stage remains documented in menu-action-records-20261021.md; current implementation and validation records are under design/reviews/menu-declarations-20261021.

`MenuContent::set_entries(Vec<MenuEntry>)` atomically declares original item, submenu and separator templates. Each nonseparator has a stable sibling value and a nonempty label; duplicate sibling values reject without changing the previous model. Items may declare `checked: Option<bool>` and `disabled`; submenus may be disabled. None is an ordinary command, Some(false) a checkable off item, and Some(true) a checked item. Selection never automatically changes this controlled check state. The caller owns its business values, checked preferences and any preference revision, as in the read-only ClientKit Sort/Group reference.

The original ContentHost action queue captures owned MenuSelection data at the originating delivery boundary. Read `drain_action_records` after opting in with `set_action_events(true)`; the old Copy ContentAction and `drain_actions` remain supported and consume the same queue. A selected declared leaf carries its full value path, leaf-local index, original label, SDK model revision, input source and original mount identity. No selection occurs on trigger opening, parent submenu navigation, focus, paint, Escape or outside dismissal. Later model changes cannot rewrite an already captured value. Unmount/remount discards undrained records through the existing mount guard. Do not reconstruct a choice from the mutable legacy `selected` field; in a declared tree that field retains the root branch, while the owner semantic value uses the captured leaf label.

Every panel uses the original MenuView/MenuRuntime and its original Button, ScrollArea and OverlayPresence. A private chain only composes these original views and routes to the topmost current panel. Pointer hover, pointer arming, keyboard navigation and semantic actions still run through MenuRuntime. Right/Enter/AX on a submenu requests navigation without a business activation; Left restores the actual active parent. Disabled items and separators are skipped by keyboard navigation and reject activation. Root and child placement use the same existing placement function with transposed axes for right/left placement. Both painting and hit routing honor the supplied viewport/ancestor clip. If a narrow viewport cannot contain two panels side by side, bounded overlap is retained; the child paints and receives hits above the parent, and Left remains available. No portal or new native window is implied.

Changed declarations revoke the old layout, held pointer/scroll intent and popup identities. Equal declarations are a no-op. Each declared opening publishes fresh opaque SemanticPart identities; they occupy a namespace disjoint from valid legacy Vec positions so a late declared token cannot become a positional command after switching model APIs. These are opaque usize keys, not row indices or business IDs. Active rows and check state remain separate facts. Retiring panels retain only original exit painting below new panels, with no live semantic parts or input. Unmount/resource clearing removes retained presentation. Root and private child views receive the same existing host Reduced Motion context. MenuContent advances their original ScrollArea ticks from the renderer motion-context delta, using the same bridge as ScrollContent; the consumer must not separately tick these adapter-owned views. Precise continuation packets follow the original runtime capture even across a parent panel; a new begin transfers ownership by cancelling the other original runtimes. No new motion token, physics or clock service is introduced.

`with_trigger_content(accessible_name, Content)` is a consuming pre-mount builder for one decorative trigger slot. It uses the same original ButtonContent action and the same geometry helpers as CompoundButton, with no second Button state. The accessible name is explicit. Host validation rejects interactive descendants and duplicate component identities. Caller-arranged compact frames may use the existing Exact frame policy. Changes to item declarations preserve the original decorative content instance. Ordinary labels and the original public MenuContent struct literal remain supported; private declaration state lives within already-private MenuView storage. The implementation moved to composition/menu.rs without changing the public composition::MenuContent path.

The original row label adapter reserves a leading column only when check state is declared, and a trailing column only when submenus exist. It reuses size-icon-standard, actual font measurement and spacing.space-tight, so a large glyph cannot consume the caption gap. Both row measurement and drawing use the same columns. Separators reuse the existing backing-scale hairline painter; Button still supplies the original menu row chrome, ink, clipping and focus policy. New and legacy rows share the original surface minimum rule; adding check state must not change popup minimum geometry. Existing token values are unchanged. This is not a page-specific style patch.

The default `Component::semantic_part_parent` and read-only ContentHost query preserve the old SemanticPart struct while expressing MenuButton → Menu → MenuItem → child Menu. ContentRole::Menu is additive to its existing nonexhaustive enum. Both the real public native consumer and Gallery's generic composition projection map that role and hierarchy. A wrapper delegating a Menu must forward its original parts, part-parent query and action_payload along with the normal Component methods; the independent wrapper test exercises this without private SDK access. Check/disabled/expanded/focus facts in a semantic snapshot do not establish VoiceOver speech.

Legacy label/value binding and per-item flat setters remain available. The flat setters intentionally do not mutate a declared tree: its owner updates set_entries, keeping one declaration source. Fixed menu item templates do not accept arbitrary interactive Content; the decorative trigger slot is the supported heterogeneous-content exception. MenuEntry/validation errors are support data for the existing Menu, not new registered components. No Swift, GPUI, UI framework, business enum or new design token is introduced.

Validation layers remain separate: public API and original runtime integration tests; actual bounded token probes including declared root/submenu states; source-bound CoreText/Metal screenshots with theme/type-scale/backing-scale, exit and replacement checks; real external native interaction; actual AgentKit consumer and matched ClientKit/Gold7 experience. Early green GPU assertions were rejected as final visual acceptance after image inspection found the missing indicator gap; their logs/images are retained. A first comparison also mixed pointer and AX focus decoration and was corrected to compare the same input source. Current progress.json identifies final source and pending layers. Physical IME, VoiceOver hearing and whole-page visual acceptance are never inferred from synthetic or offscreen checks. Performance is deferred. No commit, push, tag or Issue closure is part of this increment.

## Ordinary AX part activation and composing editors

An ordinary Menu semantic-part `Press` navigates/selects through the original Menu owner without transferring keyboard focus. Host admission must match this existing dispatch behavior: a composing editor does not block that non-transferring part activation. Explicit `Focus`/`SetSelection`, whole-owner `Press` that acquires focus, and prepared actions requesting focus retain their original composition/reveal barriers. The original marked-Composer submenu regression remains the acceptance case; this does not claim native VoiceOver or physical IME validation.


## Current public-consumer and native acceptance — 2026-10-29

The fixed public API/fixture input is
[`7ba2da34ed9846a7726899d612a7136787dd77e0`](https://github.com/TekesApps/TekesUI/tree/7ba2da34ed9846a7726899d612a7136787dd77e0).
The ordinary marked-editor admission repair
[`33341bd5d5377be1cc1a66da88fbef308f5171ff`](https://github.com/TekesApps/TekesUI/commit/33341bd5d5377be1cc1a66da88fbef308f5171ff)
is remotely readable and an ancestor of that input. The minimal second consumer
is [`examples/composition-consumer/src/declared_menu.rs`](https://github.com/TekesApps/TekesUI/blob/7ba2da34ed9846a7726899d612a7136787dd77e0/examples/composition-consumer/src/declared_menu.rs):
it declares the original compact trigger, Sort/Group submenu values, controlled
checks/disabled item/separator and original editor, and enables original action
records. Native `menu-tree` consumes `drain_action_records()` and retains the
owned path/revision/source/mount. It does not poll `MenuContent.selected` to
reconstruct choices. No private SDK module or consumer menu engine is required.

This batch adds only a fixture-specific, repeatable assertion script to the
existing `native menu-tree` entry. `TEKES_COMPOSITION_SELFTEST=menu-tree` enables
it; the normal entry remains available without that setting. MenuContent still
owns layout, hover, pointer/keyboard/capture, clipping, drawing and AX parts.
The script reads original public laid-out frames to send AppKit events. Its
business model changes use `set_entries`, and do not implement menu routing.
No shared Controls, Panel, protocol/runtime/render or Theme production code was
changed by this batch.

| Original #36 acceptance | Actual finite evidence |
| --- | --- |
| Delayed A/B capture survives later selection/model/focus/paint | Current `menu_action_records`8 and declaration tests preserve each original immutable path/revision once. Native Pointer `sort/titleAscending` and Keyboard `group/folder` share original mount10/revision3; both remain unchanged after later model replacement and the third AX choice |
| Open/dismiss produces no choice; old popup/model/mount actions rejected | Current original declaration/action tests cover open, Escape/outside, replacement and unmount/remount. Native opening/Escape/outside keeps the count; the fixture separately checks a retired part token through the original public semantic boundary. It does not fabricate a Press on a retired native AX object |
| Nested pointer/keyboard, disabled/check/separator facts, original Tab/IME/capture | Current19 declaration and7 item-state tests pass; native Pointer Sort and Keyboard Group choices, checked-versus-active distinction, actual published native AX submenu Press and original editor retention pass. Stage36008 uses public `MenuView::show(0)` as programmatic setup; the subsequent native AX part actions preserve the same synthetic marked-editor baseline/focus |
| Original edge/ancestor-clip/scroll/overlay ownership | Current original tests include `semantic_menu_surfaces_stay_inside_real_host_clip_at_edges_and_narrow_width`, precise child capture across its parent, old-child retirement and host-clock rebound. Because Host capture routing changed since the old portable input, the unchanged original `captured_pointer_release_reaches_owner_before_new_overlay` was additionally rerun and passed. New native evidence is limited to actual408pt resize/two bounded original Menu surfaces and one native wheel changing original long-child geometry without selecting. Original16-context/304-frame GPU evidence retains its old identity; it is not relabelled as a current all-clip/modal proof |
| Second public consumer, fixed input, component tests and native window evidence | Fixed public fixture/input above, current35 original contracts, native example build/strict and17 ordered active native phases with three reviewed original PNGs. Native semantic actions, programmatic setup and synthetic text input remain distinct from physical IME and VoiceOver |

Current prechecks ran on main
`eef2f6fdd70fae780263af3a42fd87d2895a5926` plus the two received native-entry
paths and coordinator-owned audit metadata. The original34 Menu public tests
and the additional original overlay-capture test passed. Final native example
build and scoped strict passed after a fixture-only correction. All command
source manifests remained unchanged. The correction changes only the new
`support/menu_review.rs`, so the35 passing contract inputs were reused.

The coordinator then ran the exact retained program with SHA256
`059415f0f9a1216673430ed028f2a0d6d25968fa802a98f490ab1810af9ddf92`.
It passed all17 ordered active phases and exited normally without timeout,
using the original desktop occupancy/HID>=60s and45s timeout guards. The
coordinator reviewed the raw journal, original snapshots and all three PNGs.
Editor mount12 stayed stable; during stages36008–36011 its original focus18203
and full `TextEditBaseline` stayed unchanged. The three original action records
are Pointer `sort/titleAscending`, Keyboard `group/folder`, and Accessibility
`sort/recentActivity`. Actual AppKit key events carry their nonempty native
characters, including Escape U+001B. The window resized to408pt; the original
long submenu received one `Event::Scroll`, changed first-row geometry and
produced no additional choice. Close followed the existing window path.

Evidence is retained on SSD under
`artifacts/issue-36/main65765cc-menu-consumer-20261029-01/`:
`precheck-final.json`, `main-rust01`, `main-rust02-overlay`,
`main-rust03-native`, `native01/coordinator-review.json`,
`native01/evidence/menu36-journal.jsonl`, and the three original PNGs.
`ac4-original-references.json` binds the old and current clip/capture evidence
without extending any result's scope. `overlay-routing-reuse-decision.json`
records why the one affected original capture test was rerun.

Preparation and failure boundaries remain explicit. Two static source-comparison
scripts stopped on assertions while interpreting formatting/import ordering;
they are not runtime Menu failures and are not reported as successful empty
diffs. Raw diffs are retained. The first compiled fixture was never launched:
reading the existing native AX helper showed that it can only resolve current
published elements, so the retired-key check was correctly expressed at the
original public semantic boundary; actual native AX Press remains in the
marked-editor stages. The corrected fixture was rebuilt and passed strict.
The initial composition audit failed because a five-line insertion moved an
unchanged native evidence anchor from747 to752. The coordinator corrected only
the source digest and line reference, preserved the anchor/claim and failed log,
and reran the audit successfully. This batch does not edit those shared audit
records from its isolated candidate.

Historical b430dae/183e native, portable and GPU results above retain their own
source/program identities. Downstream AgentKit adoption and whole-page matching
are not additional requirements for these five original public-library ACs.
No physical HID/IME, VoiceOver hearing or performance result is claimed here.
Normal remote CI, PR delivery and final Issue closure remain coordinator-owned;
local/native acceptance is not itself a remote publication claim.
