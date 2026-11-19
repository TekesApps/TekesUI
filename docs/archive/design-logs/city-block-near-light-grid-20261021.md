# Near-view GPU light grid — 2026-10-21

The reusable Scene3D local-light module now builds a conservative 32×32 world-XZ grid on the GPU after evaluating attached lights. Each cell stores two 32-bit masks covering all 64 supported lights, with no candidate truncation. Fragments visit set bits in the original light order, then use the existing exact radius, cone, normal, attenuation and soft-shadow calculations. Camera/model rotation uses the same world-space light recipe.

Bounds include every animated light sphere. Sphere-to-cell tests use an outward numerical margin; positions outside the bounds clamp to edge cells, retaining extra candidates. Lights still use the existing per-light conservative shadow lists. Grid bounds and masks are refreshed in ordered compute dispatches; no CPU readback, per-frame list upload or additional render target is used. Extra fixed GPU storage: 8,208 bytes (8.016 KiB). The geometry, textures, interiors, lighting limits, MSAA, native resolution and pixel-LOD thresholds are unchanged.

Validation: full 64-light / 96-shadow reference shader, motion, dimming, fades, reconfiguration through 1/31/32/33/63 lights and zero lights. Every comparison stays within 1/255 per channel, with a fixture confirming visible shadow coverage. All 22 Scene3D unit tests (including explicit GPU tests), 4 component tests, the city night/no-frame-upload regression, strict all-target Clippy and native Release build passed.

## Native 60-second measurements

Both builds: built-in Retina, 3024×1730 window, 2960×1362 render target, 4×MSAA, night, city animation and continuous rotation, reset plus four zoom-in steps, automatic display cadence, background benchmark enabled. Before/after screenshot sizes match for each run. No build or GPU image tests ran during either recording.

| Metric | Before | After |
|---|---:|---:|
| Native FPS | 65.61 | 99.04 |
| GPU begin-to-end mean | 24.99 ms | 14.07 ms |
| GPU active time mean | 24.82 ms | 13.88 ms |
| Window GPU activity | 99.62% | 99.34% |
| CPU average | 7.58% | 11.85% |
| CPU one-second P95 | 10.66% | 15.65% |
| Process footprint at end | 536.84 MiB | 538.30 MiB |

Native FPS increased 51.0%; GPU begin-to-end mean decreased 43.7%. The view is closer to 120 FPS but does not sustain it. GPU activity remains near full because the uncapped scene uses the saved work to render more frames. CPU utilization rises with the higher submitted frame count; 100% CPU denotes one logical core. Process footprint cannot be called dedicated VRAM.

Requested recording duration is 60 seconds each; rolling GPU counter coverage is 59.665573 / 59.788210 seconds. CPU covers complete 60-second windows. Desktop activity is shared, and start phases may differ slightly; this is not isolated-lab evidence. The native layer counter supplies FPS; GPU timing counter intervals may overlap, so their reciprocal is not a valid FPS ceiling.

The app was restored to normal home zoom, automatic cadence, auto-rotation off and background benchmark off after sampling. Other WIP is retained. The commit scopes only the reusable light-grid optimization, its reference test and this performance report; native measurements use the complete working city/night demo assets, not the smaller asset/config snapshot bundled in the scoped commit alone.

Measured source/asset/binary hashes and full counters are in the companion JSON. Receipts, screenshots, reference test logs and executables are retained under `target/verification-receipts/city-block-near-light-grid-20261021`.

Cleanup removed 219.54 MiB of exported raw Instruments traces and the two validated crates' unused incremental caches, after file-handle checks and an exclusive target build lock. Source/WIP, dependency snapshots, executables, XML/JSON, logs and screenshots retained.

CPU interpretation: the two uncapped runs render different frame counts. Dividing average process CPU time by native presented FPS gives approximately 1.155 / 1.196 CPU ms per presented frame (about 3.6% apart). This includes all process CPU work and is only an approximate normalization; CPU and rolling GPU counters have slightly different time windows. The results are consistent with more frames requiring more host submissions, not evidence that the spatial filter moved work onto the CPU. Equal-FPS CPU cost would require a matched capped run; it was not measured here.
