# Icon visual language

The Gold7 C direction was visually approved in issue #57 on 2026-10-23: broad,
controlled curves at ordinary contour turns, rounded terminals, clear counters,
and enough breathing room to read at small sizes. The preceding very small
corner trims were rejected as barely discernible. Do not reuse them as the
approved direction.

## Square file, optical silhouette

Every canonical source has `width="24" height="24" viewBox="0 0 24 24"`.
The SVG **canvas** is square; the silhouette need not be. Fit proportionally into
the shared square 12/16/24pt artboard. Never stretch an arrow or a file to fill the
square, crop away its terminals, or add per-consumer padding to compensate for an
asset. Raster resources are square at the requested backing scale. Consumers use
the existing IconView/renderer metrics and generated size tokens.

## Contour and weight

- Settings uses six rounded teeth and a larger 3.4pt center counter. Back/forward
  use matching 18pt contours and broad quadratic bends. These are the approved
  representative sources, not a mathematical approximation of them.
- Folder and document variants share the approved tab/shoulder and fold geometry;
  meaningful interior marks remain. Error outline and fill share an exterior,
  retain the octagon and keep their different interior paint contracts.
- Soften ordinary frame, conversation, and navigation bends. Preserve path
  endpoints, element/contour order, counters, and semantic details. Rounded closed
  contours include their seam, so the first corner does not remain accidentally
  square. Existing arcs and curves are not flattened or retraced.
- Precision CAD construction, dimensions, crossing-selection borders, perspective
  edges, pointer and pencil tips, sparks, lightning and compass points are explicit
  exceptions. Preserve their identifying geometry. Already rounded artwork and
  sparse round-ended marks may remain unchanged; do not force a redraw.
- Weight is an optical decision. C uses 1.25pt regular strokes at 24pt,
  preserving 1.5pt for dense CAD construction and existing filled silhouettes.
  All paint retains full alpha. The generated `REGULAR_STROKE_WIDTH` and
  `ARTBOARD_SIZE` expose canonical artwork metrics to the native icon painter.
  Do not globally dim or blur icons, or force filled and stroked shapes to have
  the same weight.

The complete per-identity decisions and exact old/new SHA256 values live in
[coverage.json](../../design/reviews/icon-gold7-57/coverage.json). Reproduce the
default artwork from the retained inputs with
`python3 crates/tekes-ui-icons/tools/soften.py`; `--check` makes no changes.

## One identity and existing renderers

Canonical SVG is the source for Rust Glyph, the eight animated Icon variants,
Web URLs and Gallery's canonical SVG cache. The revision keeps all 177 names,
paths, public APIs, retired-name behavior and animation layer references. It
updates catalog hashes and the icon distribution metadata together. Historical
sources and failed candidates remain evidence rather than fallback defaults.

Gallery also has ten existing primitive icon identities (`check`,
`chevron-right`, `chevron-down`, `close`, `plus`, `search`, `folder`, `file`,
`info`, `spinner`). They are not extra canonical SVG names. Their existing quad
renderer now uses rounded icon strokes and broad chevron bends, retaining those
identities and original layout/clip. Non-square slots center a proportional
square artboard; they never stretch the artwork. Axis-aligned strokes use one rounded quad,
preserving exact pixel symmetry; diagonal strokes and bends reuse the existing
native primitive path. No second renderer or consumer ticker was introduced.
The user explicitly assigned this file's icon-painting portion to this work;
other shared component/state work remains with its existing owner.

Color always comes from the owning Theme/Surface result, including hover,
pressed, selected, disabled and focus. Artwork never adds a brand/status color.
An action's paired icon foreground remains authoritative over passive status
tone. Continue using the original cache/resource leases and animation lifecycle.
Changing the SVG bytes does not require a new focus, event or AX owner.

## Acceptance boundary

Review all canonical identities at 12/16/24pt and 1×/2× through the actual shared
renderer, with four modes, representative brands, normal/narrow allocations and
150% text in real components and whole Gallery pages. Record old/new source and
compiled-artwork identities. Separate numerical/pixel checks from visual judgment,
native observation, system-generated input, physical input and VoiceOver. Neither
an enlarged SVG nor a test count proves optical acceptance.

The scoped #57 receipt records the final checks and their limits. Physical IME,
VoiceOver, performance and general platform-wide Gold7 maturity are not certified
by a static artwork revision. Existing #47 and #12 retain their broader service
and whole-product acceptance responsibilities.
