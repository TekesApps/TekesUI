# Existing icon animation review

The original eight SVGs remain the artwork source. Animation annotations select
elements / contours, stable layer IDs and viewBox anchors. SVG hashes and the
signed preview binary receipt are in `preview-build.json`.

Passed: 26 Gallery Motion tests, one icon lookup test, one real GPU image-update
test; strict original-SVG endpoint pixel parity across eight icons, four themes
and two scales; all 8 × 14 effect rasterizations. Gallery / icons strict Clippy
and pure Rust boundary checks passed.

Native CUA checks in the task show the actual opened Motion → Symbols page,
NewGoal breathing, pause at progress 0.24 with zero speed, resume then reverse
with negative speed, Router DrawOn along paths, NewGoal → GoalCompleted common
ring replacement and Settings Wiggle. The user then navigated in the preview;
automation stopped changing its page. The interrupted final parameter-drag
attempt is not counted as a passed native check. CPU hit-routing and parameter
behavior remain covered by tests.

This review does not claim every native combination, physical input, large-scale
icon performance, 120Hz compliance, or Apple private choreography equivalence.
