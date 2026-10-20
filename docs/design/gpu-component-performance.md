# GPU component performance: reusable rules

Moving skin, morph, route following, wind or waves to GPU removes CPU animation and vertex transfer work. It does not remove render-command validation, allocation, driver encoding, queue submission or presentation work. Measure the running native application before choosing the next optimization.

## Shared frame submission

Prefer `Renderer::render_with_gpu` and `Renderer3D::encode_camera` over a separate `Renderer3D::render_camera` submission on every native frame. All components must use the UI renderer's `GpuContext`. The UI acquires a drawable first, then calls the producer closure once, encodes UI sampling after component passes and submits the whole frame once. Occluded, timed-out and zero-size frames do not call the producer. This prevents consuming deformation work into a discarded encoder when a drawable is unavailable. The host retains its active/reduced-motion clock policy.

```rust,ignore
ui.render_with_gpu(&scene, &mut atlas, clear, |encoder| {
    component.set_environment(environment)
        .map_err(|e| tekes_ui_render::RenderError::Surface(e.to_string()))?;
    component.encode_camera(&view, &camera, encoder)
        .map_err(|e| tekes_ui_render::RenderError::Surface(e.to_string()))
})?;
```

Existing `FrameTiming` CPU encode time includes the producer closure; its GPU timestamps still cover the UI pass only. Do not label that GPU interval as the full component-plus-UI frame.

The producer must encode into the supplied encoder, must not submit it independently, and must use the same device. Successful encoded work must be submitted; do not encode speculatively and discard an encoder after marking deformation state current. On producer failure the host must treat the frame as failed, rather than presenting stale content. Existing `render` / `render_camera` remain supported convenience paths. `render_to_rgba8_with_gpu` uses the same pass ordering for final-image tests; only this explicit snapshot/test path reads pixels back.

## Cache by lifetime and invalidation

`Scene3D` records immutable opaque/MASK draws into a wgpu RenderBundle. Uniform and GPU-deformed vertex buffers stay live, so moving cameras, skin/morph, wind, waves and other shader animation do not rebuild the bundle. Reconfiguring instances replaces bindings/counts and invalidates it; rejected configurations leave the old batch intact. Replacing an asset creates a new renderer. BLEND support remains outside this commit; future transparent draws must retain dynamic back-to-front ordering outside an opaque bundle.

Keep MSAA/depth texture views across frames; rebuild views when their targets resize. Viewport dimensions do not invalidate draw commands because bundles contain neither attachment views nor a baked viewport. Write camera uniforms only when matrices change; a stationary camera now uploads zero camera bytes after its initial 128 bytes. Environment time remains a small dirty uniform. No per-frame vertices, instances or pixels should cross CPU/GPU for analytic animation.

This distinction generalizes to other GPU components: geometry storage, draw commands, attachments, uniforms, dynamic ordering and clock policy have different lifetimes. Cache each at the right boundary; avoid one scene-wide dirty flag that recreates everything.

## Measurement and verification

1. Use the same Release executable profile, asset/config, window size, backing scale and refresh policy. Confirm animation is running; a covered/inactive/paused window is a different state.
2. Sample process CPU-time difference divided by monotonic wall time. `100%` means one logical core. Avoid interpreting a single Activity Monitor/`ps %cpu` snapshot as an interval average. The reusable `scripts/measure-process-cpu.py PID --seconds 12 --output result.json` records both interval average and diagnostic samples, and rejects PID reuse.
3. Compare animation, animation plus camera/model motion, and pause separately. Do not report a frame cap, lower resolution or reduced scene complexity as equal-quality command-path optimization. Keep paused scheduling at zero work.
4. Use actual images as the correctness oracle: compare the old and new executable at a fixed time, plus standalone versus shared-frame output. Cover skin/morph, normals, indexed/flat geometry and MSAA; then test instance replacement, camera changes, target resize, pause and demo switching.
5. Check counters for bundle reuse and zero steady-state vertex/pose/instance uploads. Test producer-before-consumer ordering on real GPU, and verify that invalid/missing/zero-size targets do not call producers.
6. Native CPU samples, offscreen image equivalence and presented-frame cadence are separate evidence. Report only what was measured; this CPU experiment does not establish sustained FPS, GPU utilization, or DirectX performance.

The city-block case and numeric receipts are in `city-block-performance-20261020.json`. The command cache benefits all `Scene3D` consumers, including Gallery; shared submission is a reusable UI hook adopted by the standalone 3D host. Gallery currently retains its existing standalone submission lifecycle.


## City-block native result

On this machine, original Release after restart measured 6.7–8.2% CPU in two short samples. Optimized Release measured 4.9–7.1% across three samples, animation plus model rotation 6.7%, and pause 0%. These are process CPU percentages where 100% is one logical core. The ranges overlap; they do not establish a stable speedup percentage. The earlier 23% observation was not reproduced after restarting the original process, so it is not treated as code-attributable savings.

The deterministic work reduction is two native queue submissions to one, a reusable opaque command bundle, retained attachment views, and zero camera uploads while matrices are unchanged. A fixed 24-second snapshot from the old and new executables is byte-identical. In the full working tree, shared-frame and standalone images match for cube skin/animation, city dynamics and cabin glass/hinges (the cabin and BLEND support remain uncommitted); regressions include cache reconfiguration, target resize and hidden/zero-size producer handling. 116 relevant tests and warnings-denied Clippy passed. No sustained FPS or GPU utilization claim is made.


The scaffold extension uses generic `StagedReveal` to grow fixed-size sections, hold after enclosure completes, then remove them from the top down. Positions and normals keep their original geometry, and only GPU visibility changes. The 112-byte instance layout, immutable draw bundle and zero steady-state instance/vertex upload contracts remain unchanged. The numeric CPU samples above precede this asset extension; its isolated commit checks are recorded separately.
