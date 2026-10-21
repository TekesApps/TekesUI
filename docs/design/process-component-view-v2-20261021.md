# Negotiated ProcessComponent View checkpoint

Status: experimental public API and independent real-GPU consumer verified. Not a complete native editor/container release.

`ProcessComponentOptions::with_view(window, view, required)` attaches an optional View to the same session, process owner and service connection. Ordinary headless Start and Ready messages retain their existing shape. A View-capable factory can be used headlessly without constructing a Renderer or producing a frame. StartView/ReadyView bind the host window, View identity and supported native capabilities; requesting unsupported IME/keyboard/AX fails explicitly.

One module actor owns business state. The existing PanelModule prepares a local Scene, moved to a GPU worker inside that child process; Scene is never serialized across processes. GPU allocation begins on the first render job. Services, generation-bound toolbar Invoke and Stop are handled by the actor while the two-frame consumer budget is occupied. The existing owned IOSurface rights, queued View lane, GPU completion retirement and ProcessControl are reused. The actor recovers its atlas before publishing a finished frame, so returned parent credit cannot race actor ownership restoration. Exit wake callbacks run after releasing the state mutex.

`ProcessContainer::new_view` adapts this negotiated path through the existing Component mount/unmount, arrange and prepare contracts. Module ViewInvalidated is coalesced and causes the next Content arrangement to request a fresh frame. Toolbar actions use the independent validated Invoke lane. `new_v1` remains an explicit compatibility entry. Pointer interception is still an explicit opaque opt-in: neither profile claims remote consumed/disposition or transparent hit testing.

Independent replay: build the `process_component_view` example in the shared review cache and run it. It checks headless use of a View-capable definition, toolbar before any frame, Invoke while two surfaces are held, actual imported black pixels in 16 render/retirement cycles, invalidation take-once, Stop with an unretired frame, explicit unsupported IME rejection, and the real shared Component mount/arrange/prepare/command/invalidation/unmount path. This is public API and real GPU evidence, not desktop or Gold7 experience acceptance.

Original headless service, V1 queued/standard Component and six original GPU cases remain regression consumers. Exact logs are retained under `design/reviews/process-component-view-v2-20261021`; source-only checkpoint publication must not promote the old frozen AgentKit delivery.

Remaining: View-scoped/multiple toolbar contributions; remote event disposition; full keyboard/scroll/IME/AX; final AppHost and AgentKit native integration; external-display interaction and matched Gold7 acceptance; final independent consumer verification. No performance test was run.
