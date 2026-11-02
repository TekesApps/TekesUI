# #12 Filter / Async / Danger narrow state-page capture

This is a test-only candidate for the existing representative Gallery patterns.
It fills the gap left by `state_capture` filtering `kind=component`; it does not
change that original matrix or its historical outputs. The original contract is
#12's full-page Light/Dark/IC, backing 1x/2x, narrow/large-type visual review and
preserved golden evidence, plus the existing combinations in
[docs/gallery.md](../../../docs/gallery.md#组合场景和-p0-盘点). G1 whole-page reading
and G7 existing business combinations remain bounded to the captured routes.

The candidate uses the original Gallery route, TextDemo, state owner, layout,
CoreText/Metal renderer and existing scroll stabilization helpers. It introduces
no production component, state machine, generic capture framework or native
input driver. Candidate preparation is not compilation, execution or visual
acceptance.

## Exact scope

All frames use 720 x 512 logical points, type 150%, brand 0, reduced motion,
four existing themes and backing 1x/2x. Top and bottom are actual page positions;
no mid-position evidence is claimed. Each case starts from a fresh Gallery.

| Route | Cases from the existing controller | Frames |
| --- | --- | ---: |
| `pattern-filter/playground` | NOTES matches two original files; no-match query; real Clear restores all three | 48 |
| `pattern-async/playground` | Edited input in Ready, Pending, first Failed, retry Saved | 64 |
| `pattern-danger/playground` | Dialog open, Cancelled/Ready, Confirm/Pending, first Resolve/Failed, reconfirm/retry Saved | 80 |

Total: 192 original PNGs. Modal-open positions are set while the page is closed,
then the original dialog is opened at that position. This does not claim that a
modal permits the user to scroll its blocked underlay. All controller-driven
input is test-fixture evidence, not native or physical input.

The manifests keep actual route, theme, type/backing scale, offset/range,
controller state and relevant original geometry. Filter result names/counts are
fixed expected oracles, not extracted rendered text; individual image review
must check the actual visible results. Pixel presence is only a render
check; every original PNG still requires individual reading review, with file
hash, observed clipping/spacing/contrast and pass/fail recorded separately.
Ordinary viewport clipping must be distinguished from intrinsic text truncation.

## Source and resource boundaries

Only the new `workbench/pattern_reading_capture.rs` and its test-only module
wiring in `workbench.rs` belong to this implementation. This README and the
adjacent preparation receipt are the only documentation changes. Apply the
small module hunk to MAIN; never replace the whole Workbench file with this
older candidate baseline. A/B/C/Progress source and historical images are not
modified or recaptured.

MAIN's sole executor must admit the actual source and grant the original
build/cache/GPU window. Reuse `scripts/tekesui_build.py` build_lock/environment,
the registered SSD review cache and original GPU guard. Use Cargo +1.95.0,
`--locked --offline`; archive the actual Cargo JSON test executable and SHA.
No new cache/target, storage registration, source timestamp change or scheduler.
The reviewed wrapper's helper functions may be reused; its historical `m.run`
source gates and hardcoded Master commands must not be invoked.

The three ignored capture tests are independently executable so a failure only
requires its affected route to be repaired and rerun. Fresh route output folders
must not exist; no test deletes historical output. Main owns the exact grant,
source/program identities and deadline. Stop on failure and retain the raw
source, logs and PNGs; do not retry inside a grant or hide failed attempts.

## Acceptance and remaining work

First compile the admitted candidate, run the affected existing Filter row/query,
Async retry and Danger cancel/recovery regressions, then execute the three
explicit ignored captures and Gallery strict checks in the same coordinated
batch. Do not run repository-wide CI per fixture. Source, environment, program,
raw status, manifests/PNG hashes and actual lock/process release must be retained.
Only after individual review may each finite visual result be marked passed.

This candidate does not certify physical display, keyboard/mouse, native
TextCommand/focus/AX, IME/candidate windows, VoiceOver, timing/performance,
other widths/brands/states, full Gold7 or registry maturity. Deferred work is
not restored. The historical Danger wide images retain their old receipt-derived
source/program identity and are not relabeled as this candidate or current MAIN.

## First actual window and bounded repair

CPU108/GPU109 compiled candidate `43f052a` and passed scoped strict. The four
regression commands each executed **zero tests** because the card used `tests`
instead of the actual `interaction_tests` module; exit zero is not a pass. All
three original capture tests ran once and failed, producing zero PNGs. The raw
programme, original runner, logs, source and actual release are preserved under
`artifacts/issue-12/pattern-reading-main-window-20261102-r1/owner-run`. No retry
or MAIN repair occurred in that grant.

The repair keeps the same 192-frame contract and exact log oracles. Real input
commits record `input: value changed`; Filter/Async now include that event. The
original event log retains only its last six events, so Danger retry success
asserts that exact suffix. Danger draws its phase label through `Self::phase`
but does not retain a `phase_frame`; the manifest records the actual null field
and this limitation rather than inventing geometry. Phase text and separation
still require actual image review.

The corrected card names the four `interaction_tests` exactly and requires one
actual pass, zero failures and zero ignored tests per command. The next binding
must record the actual driver process group with `os.getpgrp()`; the original
run's missing driver-group field is not filled from its PID. New Rust requires
a new Main grant for affected compile/strict, the four unexecuted regressions
and the three failed captures. All old failures remain failures.
