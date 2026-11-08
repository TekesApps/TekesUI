# AppHost long-text move/reflow repair — Issue #247

This review covers the local Gallery AppHost fixture, not the deferred T43
performance matrix. Original failure records and program identities remain intact.

## Changes

- A successful native titlebar handoff owns subsequent uncaptured left-drag
  events. These events no longer invalidate content geometry or request a full
  body paint. Actual component capture, hover, explicit redraw, asynchronous
  content, resize and animation retain their existing routes.
- Natural and bounded shaping share one PreparedText per layout miss. Exact
  paragraph results are then reused within the current draw, keyed by full text,
  role, type-scale bits and width bits. New frames and font-size changes clear
  the table. Position, color, clip, raster scale and Scene output are not cached.
- Default-off diagnostics separate Host CPU phases, layout work, renderer
  acquisition and drawable-reported presentation. CPU Instant and Core Animation
  timestamps are not subtracted from each other.

## Native evidence

R7, R8 and R9 use the same document, 1100×640 window, scale 2 and internal display
whose current mode reports 120Hz. Focus narrows from 365.333 to 270.411 points.
R5/R6 were on an external display whose current mode was 30Hz; they are historical
profile observations, not a 120Hz baseline. Mode metadata is not measured FPS.

| Sample | Divider Dragged count | Handler median / max | Body median |
| --- | ---: | ---: | ---: |
| R7 original release | 7 | 20.328 / 27.404ms | 15.408ms |
| R8 prepare once | 8 | 16.463 / 16.612ms | 12.536ms |
| R9 frame reuse | 15 | 7.180 / 11.947ms | 3.371ms |

These are bounded single-run observations, not p99 or a repeated statistical
matrix. Input/Redraw delivery counts differ. R9 has 169 layout requests per draw,
130–131 actual hits and 38–39 misses. Its divider action's 23 drawables all have
positive reported presentation times: 19 of the 22 intervals are approximately
8.333ms and three are approximately 16.667ms. R8 had three zero-presentation
records in its selected action range and a maximum positive interval of 58.333ms.
This does not establish a compositor root cause or physical scanout timing.

The separate R9 pure-window-move action changes x=212 to x=257 while size and
scale stay fixed. Thirteen Dragged events cause no body draw, with only the down
and up boundary redraws retained. Focus geometry does not change. Recorded text
translates with the window rather than reflowing.

All 343 move and 188 divider decoded frames were checked for whole-region blank
content in a fixed crop that remains inside Focus; none was blank. Captured action
frames were also reviewed for disappearance and old/new layout alternation;
none was observed. Continuous width changes and final layout are correct in this
fixture. Capture does not sample every display refresh, so this is not an absolute
no-flicker or every-frame-120Hz guarantee. The 11.947ms handler maximum and three
16.667ms intervals remain recorded rather than relabeled as meeting 8.33ms.

## Verification and provenance

The serial `apphost-frame-layout-021` batch passed all 94 AppHost library tests,
strict AppHost/Gallery Clippy and the release build. Tests exercise the original
CoreText Paragraph/Scene oracle, real Scroll/Stack reuse and reflow, current
position/color/scale output, frame/font invalidation and real Toolbar/SplitView
capture preservation. Generated capability consistency, composition audit and
affected formatting checks passed without promoting capability declarations.

R9 signed program SHA256:
`44e2e85dd6c20cf8bf40ca91e7088a1c6e4adf893f9116b63d6d1e204b2c8d11`.
Its archived executable SHA256:
`52875564494e78e8f1958d73553e6f94e4db7509296a545356e2236e998dac1e`.
It is the pre-version-bump diagnostic candidate, not a renamed v0.3.2 binary.
Version-only publication changes reuse these behavior results with exact source
file identity checks; the final release build has its own identity.

Raw evidence is retained under
`artifacts/issue-247/apphost-frame-layout-20261108-r9/`:
`LOG-REVIEW.json`, `native-stdout.log`, `move/events.json`, `divider/events.json`
and both original videos. Coordinator receipts include
`outputs/apphost-flicker-247/R9-PRESENT-REVIEW.json`, `R9-PIXEL-REVIEW.json`, and
the action contact sheets. Both native programs and held input were released.

Public CI, release and Dock adoption are recorded separately in Issue #247 and
the publication receipt. This review does not close unrelated #229/#56 conditions.
