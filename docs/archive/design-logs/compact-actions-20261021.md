# Issue29 compact decorative actions

Root owns the shared implementation; AgentKit owns its rail item identities, layout, ink and business selection. This WIP reuses ButtonLayout, Button arm/cancel/motion, CompoundButton decorative ownership and mount-bound Content actions. No new registered component, Agent model, UI framework or token value is introduced.

## Contract

Set `compound.action.button.frame_policy = ButtonFramePolicy::Exact` explicitly for a caller-arranged compact action. Its original frame, decorative slot, chrome/hit geometry and AX frame stay identical. Exact has no implicit minimum height or content insets. The default Minimum policy and the ordinary static Button::measure token minimum remain unchanged. The caller owns its layout; this option does not turn all normal buttons into small targets.

Set `compound.action.pointer_activation_phase = ButtonActivationPhase::Press` to commit the original pointer action at primary down. Release is the default. Press uses the original Button arm with an already-committed state; repeated down, release, drag-out and cancellation cannot dispatch a second activation. A later cancel does not undo a previously delivered press. Keyboard Space/Enter and AXPress keep original once-only timing and original source/mount/target. Disabled/loading/unmounted targets retain the existing delivery fences.

CompoundButton still has one explicit accessible name and one action node. The decorative slot is not a second AX button and cannot contain an independent interactive descendant. The original skin, hover/press motion, Reduced Motion and visible keyboard focus remain. Original normal Content painting draws an exact focused action branch after adjacent sibling chrome so its shared outsetting focus ring is not covered. This changes no layout, hit/AX order or overlay order. Consumers explicitly owning child painting retain their declared composition order.

A generic second consumer `examples/composition-consumer/src/compact_actions.rs` arranges three consecutive10pt actions plus an ordinary button. Only layout and decorative ink belong to that consumer; it has no pointer, keyboard, capture, cancellation or accessibility state machine. It paints the original controls, rather than an invisible button plus a separate visual mirror.

## Evidence and boundaries

Read `design/reviews/compact-action-20261021/progress.json`, final logs and native receipts for exact current outcomes. The first native review discovered adjacent chrome covering the lower focus ring despite a weaker GPU changed-pixel assertion; it is retained under before-focus-layer and is not accepted. The subsequent regression first failed, then the shared traversal was fixed. GPU checks now require both upper and lower ring pixels to differ from the unfocused frame, in addition to actual pressed/hover/RM rendering and geometry.

Public API, actual component integration, GPU, native, scoped Gold7 and matched ClientKit product-page acceptance remain separate. No VoiceOver listening, physical IME, performance or full-page match is implied by these tests. Previously handed-off545/292f/10b5 packages stay untouched. A new isolated immutable consumer delivery and separately authorized public source/CI are still required.

Final producer validation:7 API /426 controls /216 consumer tests;24 real GPU contexts/192 frames and explicit top/bottom focus-ring pixel checks;17 canonical bounded token probes/6432 cases plus288 exact-action-only token cases (all complete=false). Seven actual active U2790B@2 checkpoints prove original keyboard/AX and physical pointer hold timing,loading/disabled/recovery,exact AX frame and full keyboard focus outline. Native drag-out/physical cancel is not claimed; original deterministic shared arm/mount/cancel tests cover that boundary. VoiceOver stayed off. Owned app/test outputs were closed,unused-checked and cleaned; sources/evidence/common cache retained. Independent actual product integration and public delivery remain pending.
