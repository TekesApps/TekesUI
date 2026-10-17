# Gallery scrolling and presentation-boundary retest — 2026-10-16

The approved appearance, JSON Design System and component set are unchanged.
All implementation remains Rust / WGSL. The normal driver remains CAD;
CAMetalDisplayLink is still opt-in.

## Environment and acceptance

The background storage window was closed, but the 180s and 600s preflights both
failed the existing CPU gate. Native results below are **shared-desktop
diagnostics**, not low-background acceptance. No process was killed and no
threshold was relaxed. Full ambient process details remain local under `target/`;
`summary.json` contains sanitized aggregates and hashes. 100% CPU means one
logical core, user + system. Disabled GPU counters are unmeasured, never zero.

Each scrolling case used the same signed `6f3bcfc3…` candidate for 60 seconds:

| Page | CAD CPU | CAD actual gaps | Metal CPU | Metal actual gaps |
| --- | ---: | ---: | ---: | ---: |
| ScrollArea | 9.95% | 4 | 10.81% | 7 |
| TextEditor | 9.74% | 2 | 9.49% | 4 |
| Icons | 10.15% | 2 | 13.94% | 305 |

A gap is an actual positive presentation interval greater than 1.5 nominal
refreshes. All six cases failed strict zero-drop acceptance. Background activity
changed between sequential runs, so these numbers do not prove a driver A/B
performance ranking. The first ScrollArea CAD gap was about 28.25s after measured
start, consistent with the unresolved roughly-30s-after-process-launch pattern
also seen in Symbols. It is not yet attributed to a system or application cause.

## Fixed measurement boundaries

The old harness invalidated the Metal link immediately after submitting its last
measured frame. The new harness stops submitting frames, waits for all measured
serials' terminal presentation receipts, then stops the link and begins the full
five-second idle observation. A separate main-queue wake checks the 500ms timeout
even without further display callbacks; scheduling delay may make that check
later than 500ms. Missing, unpresented, duplicate and timeout evidence is retained.
No last frame is removed from the report.

- CAD / Metal two-second native regressions: 242 positive receipts each, no gaps,
  missing or unpresented frames; drain about 33ms / 50ms, idle callbacks/acquires 0.
- Timer-only negative acceptance diagnostic: callback polling deliberately
  disabled; independent wake completed at about 505.5ms, all 242 receipts positive.
  The performance gate rejects this diagnostic flag.
- Twenty-second normal Metal regression (`df21b248…`): 2402 positive frames,
  actual mean 8.333253ms, zero actual gaps / callback misses / missing / unpresented,
  CPU 9.57%, idle callbacks/acquires 0. It does **not** cover the unresolved 30s point.
- Early-close native regression on final `0d19db76…`: only 0.776s idle observed;
  correctly `completed=false`, exit 2. Early closing can no longer certify five
  seconds of idle. A subsequent normal run of the same candidate completed with
  all 242 frames positive and full idle observation.

Exit 0 describes a complete measurement and full idle observation; performance
and presentation success still require the strict summary, not the exit code.
The strict summary also rejects incomplete drain, actual non-120Hz cadence,
unknown integrity/visibility, CPU peaks beyond the period, and short idle evidence.

## Validation and provenance

Final Gallery tests: **162 passed, 1 pre-existing ignored**; platform: **19 passed**;
strict release all-targets Clippy passed; Python acceptance regressions: **8 passed**;
pure Rust source guard passed on 598 managed/untracked files.

`build-measured-drain-final.json` identifies the final signed executable, compiled
binary, source hashes and a complete source-byte archive. The only compiled-source
changes after the `6f3bcfc3…` candidate are `perf.rs`, the Perf-specific branch of
`host.rs`, and selftest support. Normal rendering, controls, tokens and layout are
unchanged. `raw/` preserves compressed native reports/logs and sanitized launch
receipts; `summary.json` records their SHA-256 hashes. Old failures remain intact.

Stable long-running 120Hz / 8.33ms is **not yet accepted**. The next diagnostic is
main-thread sampling around process launch +27…33s, followed by a new quiet-window
matrix when the original background gate can pass.

## Current review app and user constraint

The normal Gallery was relaunched from the final compiled candidate on ScrollArea,
English, default CAD. `gallery-launch.json` records its signed executable SHA/PID.
Native AX and screenshot confirmed the existing appearance, section/item
navigation and remaining-viewport bottom alignment. The user is currently unable
to minimize Codex manually. No further quiet preflight is queued, no application
is stopped, and no safety restriction is bypassed. This is an environment limit
on formal acceptance, not a claim that long-running 120Hz has passed.
