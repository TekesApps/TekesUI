# Standard Content adapter and owned GPU registration

Status: scoped legacy V1 adapter acceptance. Not a complete negotiated ProcessContainer release or an AgentKit editor migration candidate.

## Root causes and shared fixes

The public process base now owns a queued View path, but consumers still needed to build their own Content lifecycle. Also, Renderer GPU registration was a raw ImageId: a Component cannot borrow Renderer inside its Unmounted/Drop callbacks, so removing a node could leave its image registered.

Renderer now provides a non-cloneable GpuImageRegistration. The guard is bound to its Renderer, queues unregistration on drop, and is automatically collected at every actual render entry (including skipped native surface paths). An explicit collection method handles hosts removing content without submitting a frame. Existing register_gpu_image/ImageId APIs remain available. Replacement verifies both the guard owner and the GpuImage owner; IDs coinciding in different Renderers do not bypass ownership. Already submitted work retains wgpu's usual resource lifetime.

ProcessContainer in module-runtime implements the original Component protocol, using the original Content tree rather than a second UI renderer. Its new_v1 constructor is an explicit compatibility profile. The existing ProcessComponent and private queued View adapter own the subprocess, pipe/Mach rights, frame budget and GPU completion.

- Constructor does not spawn. Mounted starts the fixed child; Unmounted closes it, releases the image guard and triggers collection on the next host render. Reinserted content receives a fresh component process/session.
- Original Content measure/arrange, clipping, paint, pointer capture, lifecycle and handled-event propagation apply. The component background uses the existing scale-aware Panel painter and JSON token roles.
- Host GPU attachment is one `prepare(&mut Renderer)` call through `ContentHost::get_mut`/component_mut before Content paint. The existing mutable access marks Content dirty; supplied wake is the existing host callback (native users pass WindowWake). Frame selection/import/replacement/retirement are owned by the adapter, not reimplemented by consumers.
- Backing scale or geometry changes immediately make old geometry ineligible for precise input. Older frames can remain visual previews while current geometry is pending.
- Hide clears UI input/queued frames and preserves the same PID/session/business state. The shared queue inserts a Cancel barrier and discards hidden producer frames so a held frame budget cannot strand cancellation. Show requires a new arranged geometry frame. Close/unmount terminates/reaps instead of hiding.
- Disabled/inactive paths reject input, clear the press and use the original cancellation primitive. Native host focus loss still calls ContentHost.cancel, as for all original components.

## Explicit V1 exception

V1 has no remote per-event consumed/disposition reply, keyboard, scroll, text/IME or accessibility projection. `new_v1` therefore does not intercept pointer input by default. A limited prototype can explicitly opt into `set_opaque_primary_input(true)`, declaring the View an opaque primary-input sink. Handled events then use the original default stop and optional Content bubbling policy; unsupported events remain unhandled. This exception does not prove transparent remote hit testing/bubbling or full Agent editing behavior. Remote interaction capability negotiation remains a delivery blocker, not an excuse to convert an editor to a preview.

AppHost retains its current native navigation-window acceptance until it chooses to migrate to the public queue/Component. Its separate live code is not overwritten here. No other component family is migrated.

## Evidence

`design/reviews/process-container-component-20261021/verify.py` reuses review-cache/build_lock and never calls cargo clean/prepare_cache.

Independent public consumer `process_container_backend --component-review <artifact-directory>` uses the actual ContentHost/ProcessContainer and actual subprocess/Metal import. It verifies constructor/mount ordering; primary input/capture; default unhandled and configurable handled bubbling; changed pixels; logical clipping; 2x stale-geometry rejection; hide/show with the same PID/session/revision and canceled press; disabled/inactive isolation; original command7; child reaping and zero image registrations after removal; fresh PID/session/state on remount; and drop cleanup. before.png, after-primary.png and scale-2x.png are GPU fixture captures, not product screenshots or whole Gold7 visual acceptance.

Separate GPU regression checks verify guard lifetime, stale Scene IDs, replacement, mismatched Renderer ownership, explicit/deferred collection and dropping after Renderer destruction. Existing external-target rendering, 13 runtime/native-Mach tests, four queued GPU and six original GPU cases, headless services and strict Clippy remain separate receipts.

Remaining: AppHost consuming this same adapter; V2 optional View/capability and per-event disposition contracts; supported keyboard/IME/AX integration; external-display native interaction and five-process assembly on the final version; matched Gold7 experience; exact standalone candidate and authorized independent consumer acceptance. No performance, commit/push/Issue closure or frozen-package replacement is performed.
