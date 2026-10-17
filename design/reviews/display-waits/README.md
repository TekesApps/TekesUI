# Display queue and provided-drawable investigation — 2026-10-16

**Stable 120Hz / 8.33ms is not accepted.** The ordinary Gallery still uses its original view CADisplayLink, FIFO and three drawables. The Metal display-link branch is an explicit opt-in experiment and has a known failing live-resize case; do not enable it by default.

| Run | Duration | CPU, one core | Acquire mean / max | Actual long intervals | Visibility |
| --- | ---: | ---: | ---: | ---: | --- |
| CAD / 3 drawables / 24 linear icons | 20s | 9.46% | 5.370 / 23.547ms | 326 | All callbacks key and visible |
| CAD / 2 drawables / 24 linear icons | 20s | 8.72% | See raw data | 309 | See raw visibility |
| Metal provided drawable / deferred startup smoke | 5s | 11.74% | 0.017 / 0.111ms | 5 | 543/598 callbacks non-key; not acceptable as foreground evidence |

All are shared-desktop runs, not an isolated A/B. Both complete CAD runs had zero missing/duplicate/non-presented receipts. The Metal smoke had three non-presented drawables and is not a strict pass. CPU is process user+system, 100% = one logical core. A long interval exceeds 1.5 nominal refresh periods (12.5ms). Raw JSON and SHA manifests are under `raw/`; timelines preserve separate callback/acquire/present timestamps. Sanitized run receipts exclude ambient process command lines.

The queue-depth change did not solve the waiting. The deferred Metal smoke consumed 778 supplied drawables including warmup and performed zero ordinary acquisitions; idle added zero callbacks/acquisitions. This supports that the provided-drawable bridge removes the specific acquisition wait; it does not prove zero dropped refreshes or lower overall CPU under equivalent conditions.

The first Metal experiment never reached measurement: startup callbacks reentered another event handler, were skipped, and the link stopped providing frames. Its 80-second timeout is preserved in `run-receipts.json`; local diagnostic stacks/logs are in `target/display-waits/`. Retaining queued drawables and deferring creation allowed the five-second smoke to complete. A later ordinary GUI check found paused links retained prefetched resources and failed to resume; stop/invalidate/recreate plus a callback lifetime guard fixed real-window pause/resume. Active live resize still stalled subsequent updates, so this remains disabled by default.

Current measurement fixes: only unique nonzero serials from rendered frames are matched; skipped render callbacks, duplicates and missing receipts are separate; presentation intervals bridge non-presented receipts by first selecting positive timestamps; visibility/key transitions and queue delay are recorded; the animation's expected presentation target and CPU submission deadline are distinct. These fixes prevent a good average or repeated old receipt from masquerading as stable 120Hz.

GPU cache/CPU workload evidence is separate: [symbols-gpu-cache](../symbols-gpu-cache/README.md). A quiet-window corrected CAD matrix is coordinated by the animation task after the final candidate is frozen; do not compile, capture or animate another preview during that matrix.


## Corrected quiet-window preflight outcome

The frozen CAD / three-drawable five-case matrix was not started. Its preflight waited up to 600 seconds but never obtained the required consecutive 15 seconds of low background CPU; exit code 3, gate_passed=false, 197 preflight samples and zero during/native samples. This is an **unmeasured** result, neither a performance pass nor a measured workload failure. Sanitized receipt: `design/reviews/display-waits/quiet-preflight.json`; the full ambient process record remains local under `target/symbol-perf-corrected/`. The animation task is awaiting the user's background-cleanup status before retrying. No threshold was relaxed and no unrelated process was stopped.
