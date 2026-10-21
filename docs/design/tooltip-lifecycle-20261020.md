# Tooltip lifecycle followup (current all-WIP)

The shared `TooltipView` now rejects inactive hover/focus/Escape, invalidates old anchor/overlay availability immediately on deactivation, and reuses its existing controlled suppression to allow a caller's hide/show cycle to reopen after Escape. No new UI layer, token or model API was added.

The cause crossed all users of this View: late input could arm the next activation, and a controlled-only request waited for a hover/focus owner that did not exist. The Gallery `FeedbackViews` already forwards these methods; it requires no duplicate state fix. An explicit controlled request may resume on activation; hover/focus requires fresh active input. No owned Content adapter or native AX semantics are implied.

Evidence: `design/reviews/tooltip-lifecycle-20261020/progress.json`, source identity `0a193337cab24fe38bcd2e0c8c2851f38b4b7678590cd3d955900442b9c97ea0`. Three public regressions failed before the repair and pass after it. Current checks pass:390 controls,164 normal independent-consumer tests, strict Clippy;144 real CoreText/GPU cases816 frames;12 fresh bounded token receipts5184 observations;44 governance tests,8 composition checks and3 compile bindings. The separate token renderer intentionally has no glyph sink; the GPU consumer draws actual glyphs.

Closed Tooltip renders produce no paint or dynamic lookups. Runtime evidence accepts an empty read list only with an explicit state assertion, zero-read flag and nonempty reason. Contradictory/malformed declarations fail. This extends existing evidence format1 rather than attributing caller-background tokens or claiming complete dependency coverage. Rest/open runtime records remain distinct from fixture, native and experience verification.

Owned unused test binaries (264755614 bytes) and the small GPU-host package were cleaned; sources, images, logs and shared libraries remain. The bd3 structured-reading package was not edited. Its separate active-native receipt is `design/reviews/content-reading-anchor-20261020/native-active.json`:0pt anchor movement after middle prepend/append and narrow/large-type reflow; draft text, UTF16 selection and focus retained; real latest action reaches the limit without taking editor focus. Its runtime was closed and cleaned independently.

Remaining: Tooltip final native lifecycle; physical momentum/IME and applicable hearing on the final combined reading candidate; matched ClientKit/Gold7 visual and full-page acceptance. These scoped functional receipts do not establish visual parity or completion of all Issues.
