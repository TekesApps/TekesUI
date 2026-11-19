# Issue28 — stable item navigation (in progress)

Current follow-up: the stable composition item command is now implemented and verified separately; see content-item-reveal-20261021.md. The full Issue28 remains open pending its final evidence and delivery. The historical viewport round below records only the shared prerequisite, not a substitute rectangle-only consumer solution. Root is the single implementation owner; AgentKit continues its independent consumer work on earlier immutable inputs.

## Compared existing behavior

ClientKit RunChrome connects the original turn command port/reveal/jump callbacks and keeps indicator actions in Kit; no Swift implementation or Agent state is being copied into this Rust foundation. Current TekesUI already has ReadingDocumentVersion attachment/revision tokens, Content mount identity, structured reading history/follow intent, ancestor Content traversal and ScrollArea geometry. Those will be reused by the item entry point.

The missing viewport capabilities are placement, a comfort zone, explicit navigation motion and deterministic input competition. Existing immediate reveal/reveal_focus handle their legacy purpose; using focus as navigation would steal the composer owner and is not acceptable.

## Shared prerequisite implemented

`ScrollArea::reveal_with(Rect, ScrollRevealOptions)` reuses the original viewport, axis policies, clamping, cancellation, wheel stream and private ProgressTransition. It adds no second scroll model or timer. Public options select nearest/centered placement, existing Insets and animation. Public result reports changed/clamped/animated; typed errors distinguish malformed target/margins and absent viewport layout.

Nearest moves the minimum distance into the comfort zone; an oversized item spanning it stays stable. Centered honors asymmetric margins. Edges and arithmetic use finite checks plus f64 intermediate calculations to avoid finite-f32 overflow. Invalid commands have no effects on an in-flight navigation or gesture.

Motion reads existing JSON motion.duration.geometry and the existing standard reveal-easing binding. No new motion/style token values. The host calls tick as before. Reduced Motion snaps immediately; changing preference during animation finishes at the clamped target. Another reveal retargets from the current position. Explicit navigation clears the old gesture/elastic ownership and fences old momentum. Fresh non-momentum wheel input wins even without a phase-began event; subsequent new momentum may continue. Geometry changes, lifecycle cancellation and legacy reveal stop the new stale animation.

Legacy immediate reveal and keyboard focus-ring rules remain separate. Only new explicit navigation uses the new animation state. Public struct defaults remain through ScrollArea::default; no original fields are removed.

Eight independent public consumer tests cover placement/comfort-zone geometry, edge clamping, oversized items, host-clocked retargeting, Reduced Motion/change, momentum/fresh-input ordering, invalid-request atomicity, axis/lifecycle/geometry changes and extreme finite input. These tests deliberately exercise the viewport primitive in document coordinates. They are **not** proof that an application can yet pass a stable ContentId without coordinates.

## Remaining after the viewport round (superseded by the composition follow-up)

1. Add the focus-preserving ContentHost entry point using original reading owner/version and stable item ID. Validate attachment/revision, declaration, mount identity and actual layout before mutation. A→B→A, stale versions, removed/replaced IDs, another owner, transaction-in-progress and unlaid-out targets need explicit outcomes.
2. Plan ancestor viewport transforms inside the library; only move scroll ancestors on the target's actual path. nearest/centered, comfort margins and edge clamping must work with a mixed text/panel/button tree where the target is not first-level.
3. Prove clipping behavior, including nested viewports and intrinsically clipped/unavailable target geometry; do not treat an arbitrary clipped item as a valid same-ID replacement or equate ordinary offscreen content with an invalid target.
4. Preserve the original composer focus, text, directed selection, undo state and preedit exactly; no focus-based jump, synthetic click or IME commit/cancel. Update reading-history intent explicitly, including an already-visible target.
5. Make layout/version/unmount/user-input interruption deterministic throughout navigation, with Reduced Motion and native momentum checks.
6. Complete the independent actual tree consumer, real GPU rendering, external-screen native actions and scoped Gold7 / matched ClientKit experience evidence. A green viewport test does not complete these layers.
7. Freeze a new isolated candidate and obtain independent consumer acceptance. Do not rewrite545 or292f; public delivery/CI requires its own precise authorization.

The source/DS/token refresh and cleanup receipts are under `design/reviews/stable-item-reveal-20261021`. No performance tests, commit, push, tag, Issue closure or component-registry expansion in this round.
