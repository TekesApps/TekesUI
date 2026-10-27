# Optional public bitmap ImageContent (#117)

The optional `bitmap-images` feature exports `composition::ImageContent`. A host registers already-decoded premultiplied RGBA bytes with its existing `Renderer::register_bitmap_image`; the resulting `BitmapImageLease` is passed to `ImageContent::ready`. The component does not read files, decode pixels, access the network, or create a second resource registry.

`ImageContent::loading(logical_size, semantics)` creates an explicit empty/loading state. `begin_load` returns an owner-bound generation token. The host applies one `ImageLoadOutcome` through the mounted `Content`'s `component_mut::<ImageContent>()`; stale and cross-component tokens are rejected. Failed replacements preserve the last ready pixels. An explicit Missing result releases that source from this component. The host remains responsible for scheduling a redraw.

Unmounting the Content revokes the active request capability and replaces the private source-owner identity. Re-appending the returned Content retains a ready lease if present, but a completion token from its previous mount cannot publish into the new mount.

`ImageFit::Contain` and `Cover` share one arranged geometry. Cover selects a normalized source UV rectangle; Composition supplies the existing parent clip. Alternative text is explicit through `ImageSemantics::Alternative`, while decorative is explicit through `ImageSemantics::Decorative`. The image has no input reducer and composes beneath the existing Button action/focus owner.

The initial private candidate did not satisfy publication. Current local acceptance is recorded below; immutable delivery, real-window display and VoiceOver results remain distinct.

## Current-main local dependency port

The 2026-10-28 candidate ports the original component and assertions to main
`9f827c2b1dd260ddc5a161ecda80bee9032a5457`. The optional `bitmap-images`
feature still gates the complete module and its eight exported types. The
existing Scene/ImagePass/Renderer ownership and retirement path now carries the
original controlled bitmap lease; no alternative image registry or raw-ID
substitute is introduced. Current clip/fragment/KeyedQuad behavior and sticky
composition-error admission remain in place.

Image alt/decorative semantics project locally through the current platform
snapshot and existing Host aliases. The removed historical remote semantic tree
and decoder are not restored: current ProcessComponent/CurveV1 remains reject-only,
and no bitmap lease is serialized or stripped to a raw image ID for IPC. The
historical #120 matched-peer Root evidence remains its original result; a current
main remote Image semantic bridge is not delivered here and remains tracked in
#120 / #5 / #48.

The original eight bitmap consumer tests and all 65 original assertion macros
remain. Its macOS CoreText adapter additionally supplies the current LabelLayout
service. These current sources have not been compiled or run by the port worker;
old Root AC1–5 evidence is not a current-main acceptance claim. Public API type
classification and immutable delivery/consumer receipt remain root-owned.

## Approved non-Gallery API boundary (2026-10-28)

The user approved keeping this component optional and outside Gallery. The same
`public-api-surface.json` records ImageContent as `optional-component` and its
seven owner-local types as `optional-component-support`, with the real
`bitmap-images` Cargo feature and explicit owner API. They have no Gallery IDs
or routes; the existing45 components/5patterns/2examples and52 registered roots
are unchanged. ImageContent is not classified as a shared helper.

The existing generated external binding target follows selected features:
feature-off omits these8 references and records; feature-on actually references
all8 types. Existing registered bindings cannot be reclassified as optional,
unknown Cargo features and missing/cross-feature support owners are rejected.
The manifest participates in existing input freshness. This is a reviewed
feature boundary and real Rust compile check, not a general cfg or source-closure
proof. Default workspace CI remains feature-off; one bounded CI step additionally
checks the bitmap feature, original consumer/library tests and the local Image
projection/native-object target. It opens no window.

The current source port preserves the subsequently accepted Separator exports.
The worker did not validate this source. Current root runtime, GPU, strict and
local AX-object results are recorded below; delivery identities follow the actual PR. The prior candidate integration
HEAD strings and old Root results are not current-main acceptance. Remote Image
semantics, real windows, physical IME and VoiceOver remain distinct scopes.

## Current local acceptance (2026-10-28)

Root ran the original eight consumer cases with real CoreText and offscreen GPU,
including the original curved Panel/Stack/Button/TextField composition and all
65 assertions. Geometry, transparent clipping, two-owner/old-Scene lifetime,
stale/cross-owner completion, remount revocation and one Button action/focus
ownership passed. The original controlled lease/registration/retirement, Scene
fragment/opacity and Controls library regressions and cfg-off/on external public
bindings passed. Selected Controls/Render/Scene strict checks passed.

The first local platform test build exposed a missing historical admission
function. The actual current Window raw-snapshot all-or-empty policy was extracted
without behavior changes into a crate-private function, now called by both
production and the unchanged tests; it is not a test stub or new public API.
Window geometry/closed gates and notification behavior remain in place. Final
platform strict and complete local snapshot/projection library regressions
passed. The real AppKit object-only probe passed Link and Image role/alternative
checks with windows=0 and desktop_actions=0. No real-window, physical IME,
VoiceOver or remote Image bridge is claimed.

The initial platform failures and initial real object result retain their
source/program identities. Only the affected platform checks were rerun after
the extraction; the Image/Renderer/Scene GPU results use unchanged relevant
inputs. Shared catalog/cfg support passed the existing governance suite and its
new optional feature/owner negative cases. Actual source/program/depinfo/results
are retained under SSD artifacts/issue-117/main-fc82-intake-20261028, including
local-admission-fix01. Final-head CI and immutable delivery are still required
before Issue117 AC6 is marked complete.
