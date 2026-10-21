# Shared queued View transport and GPU retirement

Status: bounded foundation acceptance; the standard Content ProcessContainer and V2 optional View protocol remain incomplete. This is not a migration-ready or release candidate.

The existing AppHost queue/worker pattern is now implemented in module-runtime. The public lifecycle owner is ProcessComponent; its private V1 View adapter reuses ProcessControl, ModuleProcess, bounded pipe framing, ReceivedFrame and the original IOSurface/Mach import. No consumer process manager, UI framework or new styling layer is added.

## Public API and ownership

- `ProcessComponent::spawn_v1_view(module, host_epoch, wake)` returns immediately after scheduling a background launcher. Every component has a distinct instance/session; the explicit V1 adapter uses a dedicated original Mach endpoint and the compiled PanelModule catalogue. V1 does not carry V2 session fields on its old wire format.
- `enqueue_view(Request)` validates finite geometry, stable identity, monotonic sequence/viewport and the 256-entry bound. Adjacent queued Layout requests coalesce; commands and cancellation remain ordering barriers.
- `drain_view_frames()` transfers immutable frame guards. At most two frame slots, including a producer in flight, may exist. CPU import does not return capacity.
- `ViewFrame::import(context)` consumes its received right once and returns `ViewImage`. It rejects a closed/dead owner. Retain the image guard while its GPU image can appear in future submissions. Unregister/replace its Renderer image before dropping the old guard.
- `ViewImage` exposes metadata and the existing GpuImage. Drop schedules the consumer queue completion callback and a common background retirement queue. Capacity is returned only after actual GPU completion. Device polling is nonblocking and observes cancellation; no UI-thread GPU wait is introduced. On close the worker stops, immutable GPU resources already submitted remain subject to wgpu's submission lifetime, and cancellation does not grant another producer frame.
- `close()`/Drop clear queued requests/frames, cancel the child and wake frame-budget waits. Background transport owns waitpid. A retained old frame cannot be imported after close. Unexpected worker exit is detected even when idle or blocked on capacity.
- `view_capabilities()` and `require_view_capabilities()` expose/reject the limited V1 profile: primary pointer only; no keyboard, scroll, text/IME, accessibility or toolbar independent of rendering. V2 services on this adapter fail explicitly. Existing editors must remain on their supported input path.

Headless V2 components keep their original service protocol. They create no View/container/render resources. This staged V1 bridge is not a substitute for the final negotiated optional View contract.

## Consumers and acceptance

The independent public-API executable `process_container_backend --queue-review` uses ProcessComponent without AppHost/Gallery. It verifies two frames held without returning capacity on import; four actual Metal cases at 1x/2x against expected pixels; GPU completion before budget reuse; 78 Layout replacements retaining sequence80 followed by command81; 256 queued commands; monotonic/instance rejection; close with frames retained; close of a producer that received its request and never emits output; old-frame rejection; idle exit detection and explicit unsupported-capability rejection. Logs and executable identity are in `design/reviews/process-view-queue-20261021`.

`verify.py` in that directory uses the shared review-cache and its build lock; it never invokes prepare_cache/cargo clean. The 13 existing unit/native-Mach checks, six legacy GPU cases, headless process consumer and strict all-target Clippy are separate receipts.

AppHost owns its window.rs migration. It is currently completing an independently authorized native navigation-window phase. Do not replace its implementation halfway through that acceptance. After the phase, remove its local Inbox/request worker/frame ack/GPU completion loop and retain only ProcessComponent plus current ViewImage; layout/navigation/Toolbar stay consumer-owned. Its previous three-View and 16-case navigation receipts are useful baselines, not proof of the new queued integration.

## Remaining work

1. Consume the public queue in AppHost after its current native phase, proving Single/Three with the same implementation as the independent consumer.
2. Implement the actual Content Component adapter with original measure/arrange/clip/event/focus/mount/unmount and framework wake/invalidation. Resolve Renderer registration ownership so removal cannot leave registered images behind. A caller-written texture state machine is not completion.
3. Implement explicit V2 optional View negotiation and all supported input contracts. Unsupported editing/IME/AX consumers stay unconverted.
4. Verify hide/show versus close, capture/focus cancellation, current-geometry gates and frame retirement under real external-display/native/GPU scenarios; then Gold7 and final exact candidate handoff to both authorized sessions.

No performance measurement, Git mutation, Issue closure or frozen-package replacement is part of this phase.
