# Issue31 shared readonly label layout core

Root remains the sole implementation owner. The root cause is a missing shared label-layout capability: existing content can wrap naturally but cannot express a positive line limit and tail overflow. The additive CoreTextShaper::shape_label API reuses the existing font registry, typesetting, native CTLine truncation and glyph collection. LabelOptions defaults preserve existing shaping. The complete source and UTF16 identity remain available; synthetic ellipsis glyphs never replace business or accessibility text.

Applicable future integrations are TextContent, ButtonContent and CompoundButton labels. Editable text is intentionally excluded: truncated readonly label geometry is not caret or selection geometry. No new design token, product view or editing state machine was introduced.

Verification on source hashes in design/reviews/label-layout-core-20261021/verified-inputs.json: 2880 public layout combinations; 50 text tests; 426 controls compatibility tests; 173 independent-consumer tests. Two GPU tests compare 432 cases /864 frames against independent native CTLine positions and explicit multiline references. Strict text and scoped consumer Clippy and both format checks passed. Capability inventory and45row composition audit match current inputs. Cleanup removed209992032 bytes of unused test executables while retaining source, evidence and shared libraries.

This is foundation evidence only. The opt-in Content/control contract, shared measure/paint integration, actual component consumer, native/AX compatibility, matched Gold7/ClientKit visual review and independently reproducible public candidate remain pending. Existing control compatibility does not establish those new-path acceptance conditions. No native app or VoiceOver was started for this core unit; no performance test, commit, push or Issue closure was performed.


## Issue #25: semantic foreground on the original label (2026-10-30)

Refs #25 comments 6326892262 and 6348459311. This increment uses the current unified
`Button`; the historical ButtonContent/CompoundButton names above do not describe
new public owners. Ordinary TextContent, original Button text and a custom Button's
existing decorative TextContent accept one immutable SemanticLabel. The consumer
provides complete source, nonzero source/revision identity, sorted disjoint bounded
UTF16 foreground runs; constructor validation rejects scalar/grapheme splits,
overlaps, reversed/empty/out-of-source ranges and capacity violations. Limits are
16384 UTF16 units and 256 runs. Missing ranges inherit original foreground.
Generic Secondary/Positive/Negative roles resolve through the SDK Palette at paint,
including current theme and brand. No product/Git/Agent type or consumer RGB enters
the model.

`Content::with_semantic_label` attaches to the existing label adapter; controlled
`update_semantic_label(expected, next)` checks expected identity/revision, same source
identity, increasing revision and unchanged current component source before changing
either text or attributes. Source identity is caller-controlled, not content hashing.
A custom Button's decorative slot stays presentation-only; the Button keeps its
explicit complete accessible name and its original pointer/key/AX action owner.
When a custom Button's complete source changes, its host also updates that original
Button's explicit accessible name using the existing component API. No extra editor,
focus stop, hit algorithm, action state machine or AX element is introduced.

CoreTextShaper::shape_semantic_label uses the original complete attributed string,
font fallback, typesetter, line builder and ellipsis/clip implementation. Distinct
native foreground attributes carry semantic boundaries into shaping, including legal
grapheme boundaries inside optional fi/ffi ligatures. Marker CGColors only segment
native attributes; the SDK paints Palette colors, never marker colors. Ellipsis
candidates retain source attributes on the native substring before original native
truncation, while synthetic ellipsis has empty source provenance and inherits the
original control foreground. The label adapter partitions only retained glyphs for
painting in original order with unchanged coordinates, fonts and line metrics.
Whole-ligature cluster-start color substitution is forbidden; fi/ffi tests must show
that no retained glyph spans a change in effective foreground.

The narrow TextSurfaceHost::shape_semantic_label trait method defaults to the old
shape_label path for an empty run list, and explicitly returns None for attributes
unsupported by an old backend. Existing plain APIs/implementors remain source
compatible; requested styled services do not silently fall back to monochrome.
Shared app-host, Gallery and independent consumer CoreText backends implement this
additive label method. No TextSurface editing state, rich document/parser/Markdown
or platform input chain changes belong to this increment.

Verification plan: typed boundary/capacity tests; CoreText fi/ffi and Unicode/fallback,
scales and clip/ellipsis; independent consumer exact glyph order/coordinates and role
ink; atomic controlled update rejection; original TextContent/Button/custom label AX
names, single action ownership and Composer marked baseline across revision, theme,
width and font changes. `native semantic-label` reuses the existing native consumer
shell and selftest entry, publishes revisions/actions/Composer text/selection/marked
and captures light/wide and dark/narrow windows. Programmatic AppKit/AX input,
physical IME and VoiceOver evidence remain distinct. No build/native pass is claimed
by this design. The coordinator owns final integration/public pin and Issue status.


### Semantic-label scoped verification

The final controlled SSD receipt is `semantic-label-20261030-01/verification-ssd02/verification-results.json`.
All 10 planned commands passed: 22 scoped tests, native example build, app-host/Gallery
check and text/controls/consumer/app-host/Gallery strict checks including test code.
The accepted Gallery helper, registered SSD cache owned by MAIN, original build/GPU
locks, cleared SDKROOT and SSD TMPDIR are recorded with exact source/config/toolchain
and archived program hashes. Earlier local-target observations and the first SSD
environment deviation are retained and superseded for controlled acceptance; no
cache or failure evidence was deleted.

`native-ssd01/native-result.json`, `frame-history.jsonl`, raw logs and
`domain-native-review.json` record nine observed native states. Original Button
pointer and Enter actions followed by custom Button AXPress produced exactly 2/1
actions. Three native UTF16 state checks retained Composer text, selected/marked
17..19, and focus while labels advanced revision and the window became narrow/dark
with type scale1.5. All three complete label AX names remained; the decorative slot
introduced no AX owner. Two actual window PNGs received scoped visual inspection.
This specific label fits at native260 width; native ellipsis is not demonstrated by
those images. Clip/ellipsis and fi/ffi boundary evidence comes from the scoped tests.
Programmatic AppKit/AX evidence is distinct from physical IME and VoiceOver, which
remain not_run. Final public metadata, integration CI, published pin and Issue
acceptance remain with the parent; this evidence does not claim release or full #25
closure. Source/runtime and failure evidence are frozen before window release.


### Native ellipsis coverage follow-up (prepared, not yet run)

The earlier native260 label fit completely; that was a coverage limit, not an
observed production failure. The isolated semantic-ellipsis fixture repeats the
original exact Unicode source and its generic foreground runs six times. It uses
the original TextContent, unified Button and decorative TextContent slot; one
actual CoreText layout supplies glyphs and omitted-source provenance. Native
assertions require three actual truncated presentations, one synthetic ellipsis,
legal source/grapheme cuts, no hidden glyph residue and omission spanning at least
two semantic roles. Original names/actions/Composer marked checks remain in place.

The new mode targets External explicitly and rejects an actual built-in screen.
It needs a short foreground window because external displays do not isolate focus;
no120Hz claim is made. The prepared package separately describes user-participating
physical IME and VoiceOver steps; these remain not_run. Only affected fixture,
consumer and public-binding checks will run after an allocated build window; the
unchanged earlier tests/evidence are reused in their original scope.

Metadata integration additionally requires five explicit composition support-type
classifications and an additive generator resolver seam for the existing text
crate's explicit public re-exports. It resets visibility at the foreign crate
root; invented public crate segments and private modules remain rejected. Static
candidate projections, two focused resolver tests and the45component source audit
have passed. Generated Rust public bindings/new fixture/native evidence remain
uncompiled/not_run until serialized integration. No route/component identity or
family native acceptance is promoted by source metadata.


### External-screen native ellipsis verification

The programmatic native supplement completed on the actual external U2790B display;
`semantic-ellipsis-20261030-01/native01/native-result.json` binds the exact archived
program/source/configuration and the original guards. No rebuild or repeated old
native suite was used. The program exited normally, its hash and runtime inputs
remained unchanged, and the own process/build/GPU locks were released immediately.

Ten captured frames all identify an external screen. Actual177UTF16 source omitted
ranges start at133/129 (wide),44/41 (narrow), and29/24 (narrow at type scale1.5).
Every recorded layout actually truncates across at least two foreground roles,
with one synthetic ellipsis at the legal cut, no hidden glyph residue and no
retained glyph crossing a role boundary. All three complete AX names remain,
the decorative slot has no extra AX owner, and original Button pointer/Enter plus
custom AXPress action counts remain2/1. Three native UTF16 checks retain Composer
text, selection/marked17..19 and focus across revision/narrow/dark/scale updates.

Both actual window PNGs were inspected: all three original label presentations
visibly show ellipsis in light/wide and dark/narrow views, retaining semantic ink
and inherited ellipsis foreground. `native01/domain-native-review.json` and
`evidence-manifest.json` retain the finite review and raw artifacts. This completes
the earlier missing software-native ellipsis coverage. Physical IME, real
VoiceOver,120Hz and performance remain not_run; their separate user-participating
steps are not replaced by programmatic AppKit/AX evidence. Public delivery and the
whole Issue25 acceptance remain parent-owned.
