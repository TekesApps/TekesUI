# View attachment context on the existing emitter

Root cause: a reusable ProcessModule could receive ViewInvalidated messages, but could not learn the attached logical window/View identities. The independent example hardcoded View2, so the same factory in another container would emit an invalidation for the wrong owner and fail its connection. The window toolbar declaration had the same hardcoded example identity.

The existing ComponentEmitter now retains the two IDs from the already negotiated ViewAttach. `view_identity()` returns them without exposing the endpoint or constructing a Window/Renderer. `invalidate_view(revision)` uses that same existing event and returns false for headless consumption, allowing one factory's service logic to run without conditional UI state. Explicit foreign-View invalidations and stale/duplicate revisions are rejected before advancing the output serial, under the existing emitter sink lock. No wire message or public context type is added.

Applies to any optional-View ProcessModule and repeated use of the same factory in multiple containers/windows. Headless services retain no View identity or invalidation output. This does not implement View-scoped toolbar painting, remote input disposition, keyboard/IME/AX, or final native/Gold7 acceptance.

Validation:14 runtime/native-Mach unit tests, including exact sink serial checks for foreign/stale/repeated invalidation; strict runtime all-target Clippy; the real V2 consumer now consumes the same factory at window1/View2 and window91/View73, verifies its own invalidation and intact Ready status, and retains16 actual GPU pixel cases plus the real Component lifecycle test. The independent headless service consumer also passes. Existing Headless/StartView wire shapes are unchanged.

This is a subsequent WIP identity after main1294987/PR40; that published checkpoint is immutable. Evidence and before-source copies are retained under `design/reviews/process-view-context-20261021`. Native and Gold7 experience remain pending.
