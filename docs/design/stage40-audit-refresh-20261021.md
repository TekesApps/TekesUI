# PR 40: current API, token and composition audit refresh

The checkpoint CI correctly rejected old runtime-token evidence. The original source is the active all-WIP worktree, compared to the exact SHA-256 bytes recorded by the previous audit. New source was reviewed before rebinding: 17 matching historical snapshots yielded concrete deltas; app.rs had no matching snapshot, so its current lifecycle/chrome implementation and diff against the tracked baseline were reviewed directly. The local delta inventory is `design/reviews/stage40-audit-refresh-20261021/audit-source-deltas.json`. Old receipts remain unchanged.

## Actual shared repair

ToolbarControl implemented Component, but first keyboard entry left its internal focus unset. Existing registration tests clicked a button before pressing Space, masking the missing entry path. The existing public Component focus contract now chooses the first enabled visible action or overflow trigger. Left/Right/Home/End navigate the same valid stops. Changing the stop cancels the previous ButtonContent press; focus loss cancels pending input, while pointer-down presentation updates do not cancel a newly held button. Registration and layout changes reconcile the same stable IDs and valid visible stops. No new interaction state machine or abstraction layer is added.

This applies to all ToolbarControl consumers of the original Component contract, including content toolbars; it does not retroactively prove AppHost native keyboard integration. Legacy ToolbarView remains available and is not claimed by the new Control probe. Window/process contributions remain separate experimental infrastructure. Toolbar still accepts bounded action/icon descriptors, not arbitrary caller Content slots. Per-action native semantic parts and VoiceOver hearing remain unverified.

The independent public `toolbar_contract` target uses actual ContentHost, without any pointer click. It verifies Tab entry plus Space; disabled-item skipping; Home/End; canceling old Space on navigation; a disabled registration update; keyboard access to overflow; opening overflow without emitting an action; Escape and restoring a wider layout. Original toolbar pointer/drag and registration tests remain separate regression checks.

## Reviewed source changes and limits

- Content ordinary events now distinguish consumption from propagation. Topmost-target ancestry, explicit handled bubbling, capture/action ownership, and modal/IME/focus barriers were reviewed with `pointer_contract` regression cases. Default handled events stop; unhandled events bubble. No sibling-behind routing is introduced for primary targets.
- SplitView scale-aware decorative ink retains logical panes/hit regions; cursor regions are clipped, suppressed while constrained/inactive, and cleared on page/focus changes. Original separator and splitter/Gallery tests cover these paths; physical native hover is still pending.
- Native window code retains original mouse-down for a one-shot eligible AppKit titlebar drag, validates current window/button/activity, and releases it on up/deactivation/close. Unified chrome is opt-in and uses native layout/safe areas. Cursor selectors have availability fallbacks. Existing native evidence is bounded; this source review does not promote native acceptance.
- TextSurface intrinsic measurements retain derived Paragraph data without publishing edit/scroll/IME geometry. CoreText cached caret offsets are compared to native offsets across Unicode widths. DisplayParagraph is explicit readonly viewport output; editing still uses complete Paragraph geometry. These are correctness contracts; no performance benchmark is inferred.

## Fresh runtime evidence

Eighteen real public-API probes were executed after the final Toolbar source change, in the existing review-cache under its build lock. Fresh results are under `design/reviews/stage40-audit-refresh-20261021/probes-toolbar-final/`; old observations are preserved. Field and NumberField use their existing probe hooks and bind current source inputs as well. The new ToolbarControl probe renders 72 asserted cases: six states, four themes and three existing brands, through actual ContentHost and existing ButtonContent/Menu painters. Observation scope explicitly excludes legacy ToolbarView, complete state/alias coverage, font/icon pixels, native/VoiceOver/Gold7 experience and performance.

The original 45-component inventory is unchanged. The three public Toolbar types are classified under the existing route and compiled by the generated external API binding test. The composition matrix now describes the bounded adapter and its remaining limitations; no row is promoted to native acceptance. Current API/source/token evidence is distinct from final native and Gold7 acceptance on a matched published candidate.

## Independent host source review

Rust AppHost independently read the new focus/stop/cancel code and found no new conflict with existing command IDs, pointer arms, registration, nav-busy or visibility/swap cancellation. This was a source review, not a rerun of public tests/native input. It identified a pre-existing host gap at window.rs KeyInput routing: ordinary keyboard events go to the current Panel toolbar; navigation toolbar only receives them while its menu is open, and visibility/swap toolbars receive no Tab/KeyInput routing. Those window controls therefore remain keyboard/AX-unaccepted. The current running demo was not silently replaced.

Full-workspace testing also exposed an old Gallery post-proposal count of176 after a new canonical swap icon was added. The test now compares exact Glyph names and uniqueness, explicitly checking the swap icon, rather than weakening the inventory guard or changing a number blindly. Capability bindings similarly compare exact registry paths and require ToolbarControl rather than freezing the old total80.
