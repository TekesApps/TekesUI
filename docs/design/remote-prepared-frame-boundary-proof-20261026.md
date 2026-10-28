# Composition-context finding on accepted Main (2026-10-28)

The original complete Toolbar context comparison now has an independent real Metal observation on accepted production Main `102504d` plus a test-only delta. The actual public ToolbarHost constructor, Window region, Button, TextField, CoreText strings, dimensions and original proof-test body are retained. No Class A extension, shader change or fragment-isolation policy is present in this run.

| Path | Existing contract | Actual observation |
| --- | --- | --- |
| Normal Content tree | Layout freezes the ancestor curve chain; paint traverses the mounted children with that chain and inherited theme. | Source: `composition/panel.rs::freeze_curve`, `push_curve_scope` and `Content::paint_except`; existing matched CI Panel/tree regressions pass. This is not claimed as a new whole-tree native test. |
| Direct full Toolbar paint and Scene replay, both under the same actual Panel shape | `Scene::try_append_fragment` appends commands inside the caller's open clip scopes. It does not create another layout/input owner or an isolated rendered layer. | New exact real GPU comparison passes: all RGBA bytes match, and both input Scenes remain unchanged. |
| Standalone Toolbar pixels masked later by ImageQuad | Pixels retain the standalone raster context. Appending the original commands below an analytic parent instead promotes their raw Rect clip scope under the existing renderer policy. | The same six differing pixels remain, with maximum channel difference 134. This is recorded, not relabeled as an equivalent-context pass. |

The three current raw buffers (standalone original, parent-context direct paint, and parent-masked standalone image) are individually byte-identical to the retained buffers from the failed private axis-extension attempt. Thus this difference already exists with accepted Main rendering and does not require the unaccepted new predicate to appear. The shared fixture extraction changes only the destination Scene parameter; the original proof-test function and all its assertions remain byte-identical.

This finding does not establish that normal UI-tree inheritance is broken, does not demonstrate B2 transport/pointer/AX/text correctness, and does not approve preserving standalone raster semantics during command replay. The current Scene API explicitly inherits caller scope; silently adding isolation to make this reference equal would change that contract. The context-sensitive legacy Rect/analytic transition is the specific compatibility boundary to consider, rather than a Toolbar-specific patch or a new general framework.

**Disposition:** retain all original AC, complete input, red proof/reference evidence and the private unaccepted implementation. Keep #127/#81 open. Before changing the reference or the clipping contract, state which comparison is required by the original boundary guarantee and show the effect on the existing cross-context requirement; no original assertion is removed or relaxed here. The existing public API's command replay and rendered-pixel boundaries must remain explicit. No production redesign is authorized by this diagnosis.

Actual evidence: SSD `artifacts/issue-127/main102504d-parent-context-20261028-01/{preflight.json,main-cpu01,main-gpu01}`; `main-gpu01/cross-attempt-pixel-comparison.json` binds the retained buffers. Target strict/build and this one real GPU test passed under the original build/GPU locks and SSD cache. Unchanged Scene/Render/shader suites were not repeated. Historical failure and acceptance identities below remain unchanged.

## Authorized bounded extension: original complete ToolbarHost (2026-10-28)

The original complete ToolbarHost remains an acceptance gap. Main `82fe2cb` has the finite foundation and its accepted regressions; the complete scene's last actual result remains exit 101. Its first B is background primitive 1. Primitive 9 is a rounded Quad with prepared scissor `[15,107,610,74]`, outside the current parent safe rectangle `[21,21,619,267]`; its physical left edge is also outside that rectangle. Tightening the scissor to the Quad bounds alone does not repair the contract. This is a conservative-domain refusal, not evidence of wrong ordinary rendering or two backgrounds. Later primitives have not been certified by this failed run.

**User authorized this bounded Class A extension on 2026-10-28; implementation and acceptance remain pending:** extend Class A from containment in the single global inset rectangle to containment in a checked full-coverage region of the *same actual parent shape*, initially only the finite axis-aligned rounded-Panel domain required by this exact ToolbarHost. Keep the actual prepared scissors as influence bounds; keep the one-flat-background Class B rule, all controls, layout, text, decorations, input Scene, existing size/scale/one-sample limits and fail-closed behavior. Do not accept arbitrary rounded Quad, baked image or AA coincidence by type alone. No new transform, backend, transport or general proof engine.

The smallest complete implementation must join the Scene-owned full-coverage predicate, Render's actual prepared-state check and bounded receipt/work accounting, and the original complete Toolbar test. The current predicate intentionally uses a flat-center numerical argument; a wider rounded region needs a defensible shader-operation/AA bound. A CPU geometry inclusion alone is insufficient. Whether the same shader can support that bound is still unverified. If a production shader change is needed, show its exact additional scope and affected regressions before implementing it; this proposal does not silently authorize one. Existing evidence does not establish that fixing primitive 9 will certify every later draw.

The implementation/verification cost is a coordinated Scene/Render change, focused numeric/work/refusal tests and one retained whole-scene GPU comparison (ordinary/proof, single-parent reference, Scene/output preservation). Rerun original shader regressions only if shader inputs change. Do not rerun unrelated Theme/native suites or expand peripheral matrices. No elapsed-time estimate is established. Exit requires the unchanged original complete Toolbar case to pass plus the affected original refusals/atomicity checks; otherwise preserve the next exact unsupported input and keep the Issue open. B2 transport/pointer/AX/text/generation remains separately open in #81 even if this renderer case passes.

The user selected this finite extension rather than leaving the domain permanently frozen. Original AC and the complete Toolbar input remain required. This authorization is not acceptance of the implementation and does not authorize dropping the combination or silently changing the shaders.

# Independent natural empty ToolbarHost B intake (2026-10-28)

The already prepared separate public empty state is now actually verified on Main source: a `ToolbarHost` with its real Window region and no registered command groups paints one ordinary full-target flat background. No primitive is hand-pushed, deleted or cropped. The real zero-padding rounded parent Panel supplies the original geometry; its safe interior excludes the full target. Actual `VerifiedFrameOutput` records `class_b_background_primitive: Some(1)`.

Strict checking passed with the original bitmap-enabled consumer configuration and with the default CI features. The exact archived program passed real Metal GPU proof/ordinary equality, single-parent direct mask/ImageQuad equality, parent before/after equality and whole child Scene immutability. The direct parent mask visibly changes the ordinary full-target corners. Current receive base `9e3481b9885ee040eda361122d6edb36295dd0dc` has exactly the same source tree as the observed `8bb73dc` base, and the complete source map including this owned file matches; the original execution evidence is reused without claiming a fresh run.

This is one independent natural empty-state B positive. It does not replace, edit or certify the original Button/TextField Toolbar combination, which remains failed; it does not certify arbitrary child content, all B materials/edges/resources, B2 or native interaction. #127/#81 remain open. Evidence: `artifacts/issue-127/main8bb73dc-empty-toolbar-candidate-20261028-01/{acceptance-receipt.json,source-equivalence-receipt.json,main-cpu01,main-default-precheck,main-gpu01}`. Production predicate, libraries, shader and dependencies remain unchanged.

---

# Coordinate and bitmap-material intake (2026-10-28)

On Main base `0783f81c4a9d4d80a2062895bacf2c112e2d581b`, the test-only AC3 batch retains the original complete parent/child Host, rounded Button, TextField/CoreText, registered bitmap, decorations and all ordinary/verified/single-parent reference assertions. The original child viewport remains 328×208 logical points and bitmap remains 16×12 logical points. Public asymmetric parent padding produces the measured fractional physical allocation without shrinking the child or changing the renderer predicate.

Actual GPU cases passed: 0.25 backing with half-alpha bitmap; 0.5 and 1.0 backing with opaque bitmap; fractional allocation at 2.0 with half-alpha bitmap; fractional allocation at 4.0 with transparent bitmap. The original parent and richer 2.0 references were rerun because their shared test helpers now accept scale; strict checking and the exact new GPU program also passed. Production libraries, shader, configuration and dependencies did not change, so the prior shader regressions are retained at their original identities.

These cases establish the full-child **Class A** coordinate/registered-bitmap-material path. They do not establish Class B edge-band material positives, the retained full Toolbar, every primitive/material/resource/budget combination, B2 or native acceptance. The target viewport is zero-origin; checked parent shapes are expressed in child-local coordinates and can have negative/fractional origins, as in these actual allocations. #127/#81 remain open. Evidence: `artifacts/issue-127/main-b696e4a-material-matrix-candidate-20261028-01/{acceptance-receipt.json,main-cpu03,main-gpu01}`; delivery identities are maintained in the linked Issue.

---

# Main finite intake status (2026-10-28)

The existing finite A/B chain is now accepted locally on Main base `8e4811090cdee53d7ab00fb58f78c7db2eb5436a` plus this scoped batch. Scene owns `FullCoverageInnerRectPx`; the existing renderer preflight evaluates actual prepared draw state, enforces original budgets and bounded proof/readback capacity, and returns an opaque non-detachable `VerifiedFrameOutput`. The clip shader includes the previously accepted nonpositive-q branch. Existing composition-error/controlled-image preflight order is preserved. The Root-only `register_owned_image` change is excluded.

Actual validation: original Scene/Render and finite tests, strict library/public bitmap consumer checks; six original complete Host/CoreText/controlled bitmap GPU positives/refusals; original CoreText scissor and 450/18 shader regressions. The original full Toolbar positive is still **not passed**: exit 101, `prepared scissor outside parent full-coverage witness`. Its complete input and assertions remain in the independently invocable ignored GPU target. A successful diagnostic of that refusal is not a product pass.

#127 stays open for that original combination and the remaining fractional/material/resource/boundary/budget matrix. #81 still owns B2 image/transport/pointer/AX/text/generation integration. This finite batch is not B2 or native acceptance. Historical private and Gallery Root receipts retain their original identities; current Main evidence is `artifacts/issue-127/main8e48110-finite-candidate-20261028-01/`. The linked Issue records the eventual commit and remote delivery identities.

Below is the original accepted design and its historical stage notes. The current state above supersedes their pending/Root-only wording, without deleting original requirements.

---

# Remote prepared-frame parent-boundary proof

2026-10-26, main + numeric advisor accept this finite dependency direction for #81 B2/#82; exact API/budget contract must be reviewed before production implementation. #80 remains accepted and is not reopened. All-state dedup found no existing prepared-frame proof execution Issue; #81 owns transport/Host integration and #82 the independent consumer.

## Problem and ownership

A flat child image can already contain analytic coverage. Scanning only PushShapeClip/KeyedQuad misses ordinary rounded Quad, stroke, Shadow and analytic-scope PushClip behavior. A caller boolean, ImageId/lease, arbitrary pixel inference or copied consumer primitive rules cannot certify that parent curve AA will not multiply the same boundary again.

The original tekes-ui-render FramePipeline/ClipFrame preflight owner must evaluate the actual Scene and prepared coverage states. Reuse Scene immutable shape provenance, shared normalization/ClipRegion geometry and original renderer budgets; do not implement a second renderer or input service. A new bounded geometry predicate is permitted only if the original geometry owner lacks it and main reviews that precise dependency.

## Finite supported proof classes

A. The complete actual influence footprint of a primitive is provably within the parent chain's full-coverage interior. Conservative shared inner rectangles may establish this; mapping, antialias and texture sampling margins must come from the renderer/geometry owners. This permits inset actual rounded controls, text and unknown-origin images without needing to guess their internal alpha.

B. Known child coverage is exactly one throughout the parent AA band and has no unknown baked boundary or stroke/shadow/clip attenuation there. A real flat background can be clipped by the parent while inset controls remain supported. Material opacity is kept separate from geometric coverage.

### Numeric review correction, 2026-10-26

The R8 finite Class A source was integrated with exact six-file preimages; its private Scene/Render tests and GPU samples passed. This does not complete its mathematical acceptance. The actual shared `sd_round_rect` still calls `length(vec2(0))` when both components of `q` are nonpositive. The earlier statement that this region avoids the square-root operation was incorrect. The [WGSL accuracy rules](https://www.w3.org/TR/WGSL/#floating-point-accuracy) and [gpuweb CTS floating-point model](https://github.com/gpuweb/cts/blob/main/src/webgpu/util/floating_point.ts) do not establish the exact-zero premise used by that argument. A guard margin alone does not resolve that missing bound.

Main authorized the renderer owner to prepare a private minimal shared-function correction: when `all(q <= vec2<f32>(0.0))`, return `max(q.x,q.y)-r` directly, preserving the existing expression elsewhere. Source review and affected production GPU regressions must pass before accepting the integrated proof. Historical R8 results keep their original source/program identity.

For the next Class B slice, review a real ordinary Quad's final prepared instance with zero radii/border/shadow, no child clip, and full-target scissor. A target-aligned rectangle can have exact full coverage at every single-sample pixel center, including its outermost centers at distance `-0.5`, once the shared-function correction is established. This is separate from material alpha. Verify exact scaled geometry, shader operation order and the existing one-pixel vertex expansion's NDC/raster envelope. All other drawables must independently satisfy an accepted proof class; one background cannot authorize the rest of the Scene. This is an implementation direction, not an accepted Class B capability.

A positive must use an original Host-produced actual Panel Scene with its real decorations retained. Unsupported or unprovable same/near/coincident boundaries remain explicit refusals, not partial acceptance or local-only fallback. Deferred/compositing protocols are outside this first shared Issue.

## API and atomicity requirements

The boundary check runs with the same actual Scene/atlas/image registry, output size/scale, validated coordinate mapping and parent chain as the original preparation. Its process-local receipt is private and bound to the prepared frame that is actually drawn; receipt/frame/scene/resource substitution and later mutation must not turn a rejected frame into certified output. It is not a cross-process signature. #81's original transport/output owner separately binds the resulting frame to Gp/Gc/proposal/allocation and the existing trust boundary.

Refuse unknown primitive/transform/provenance, unprovable partial line/arc coincidence, unsafe stroke/shadow/glyph/image influence near the parent AA band, invalid geometry and exhausted proof budget. Reject before the original GPU/atlas submission side effects; keep the previously accepted output intact. Preserve all existing shader-work/CPU/instance/upload/device limits; define a separate real bounded proof-work charge, not an invented reuse of the shader-work number.

## Acceptance and independent scope

1. Freeze exact owner API, lifetime/receipt identity, geometry/coverage predicates, supported domain and executable proof-work/byte/depth limits with main/advisor; no consumer classifier or assertion minting.
2. Implement through the original renderer prepare/draw seam, preserving pipeline behavior and old paths. The same checked prepared frame must produce the returned pixels.
3. Real Host Panel positive with internal rounded controls/text/image; exact pixel evidence against an appropriate single-parent reference for integer/fractional origin/backing, opaque/transparent material as applicable. No shortened/replaced Scene to manufacture support.
4. Explicit negatives for coincident/near outer AA, ordinary rounded Quad, analytic PushClip, inner stroke, Shadow, unknown baked Image touching the band, unknown transform/primitive, stale/substituted receipt/resources and all proof limits. Rejection retains original Scene/atlas/output identity and produces no partial frame.
5. Main accepts source/diff/actual commands/program/depinfo/raw pictures/failures/not_run, synchronizes this design and related #81/#82/#45. No full DS/native/public-release prerequisite; local proof does not complete B2 transport or semantic authority.

Implementation only in a private SSD candidate after precise API acceptance; main remains sole live writer. Original review-cache/build_lock/environment and GPU lock, cargo +1.95.0 --locked --offline. No new target/cache, clean/install, recursive snapshots, stopped-chat messages, #38 interruption, push/merge/release or worktree cleanup.

## Current bounded acceptance, 2026-10-26

Main integrated the six finite Class A Rust changes and the shared SDF early-return correction. The integrated Scene 33 / Render 46 tests and related strict checks passed. The corrected shader also passed the actual Root CoreText scissor and original 450/18 GPU suites, plus a separate 140-case Metal q-boundary matrix across the five proof scales and odd/even targets. These are scoped source/program receipts, not a universal backend theorem or complete Issue #127 acceptance.

The actual Root Panel/TextField positive and coincident refusal preserve raw pixels and actual Host geometry. A richer Panel/Button/controlled Image/TextField/CoreText case exposed ImageContent's missing Local curve declaration before proof; that original composition seam belongs to #117 and has now passed its integrated Root test. The richer byte-equivalence case subsequently passed on the separately frozen Class B SDK; that SDK result is not relabeled as Root Class B acceptance.

The private Class B R2 source has passed Scene 33 / Render 57 tests and strict checks. Its multiple-background, offset/narrow and opaque-image refusals passed actual GPU checks. A ToolbarHost containing controls remains refused: the error is emitted when a later Quad fails Class A after a B background already exists, before testing whether that later Quad itself qualifies for B. It therefore does not establish that two flat backgrounds exist. Preserve that complete input and refusal; inspect the same prepared state's index/scissor/instance before drawing conclusions. A distinct naturally empty ToolbarHost has compile/strict evidence only and is parked as an independent case; it cannot substitute for this original composition. No failed input is cropped, replaced or promoted to success.

Finite Class B R3 is now integrated into Root as the reviewed three-file dependency batch, with only an additional rustdoc correction describing A or a single B. After the approved source-cache recovery, actual Root Render 58 tests, targeted strict checks and the Root-bound consumer compile/strict passed. The actual GPU richer Panel/Button/TextField/CoreText/Bitmap positive and true-double-B, offset/narrow and opaque-image negatives passed on that same Root source; the unchanged original Toolbar case remained a separately recorded support-domain refusal (exit 101), not a product pass. Evidence: `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-127/main-class-b-r3-root-post-recovery-20261026-01/runs/post-recovery-{cpu,gpu}-01/result.json`. Root evidence is distinct from private R2/R3 and local main. The original acceptance ledger, remaining primitive/resource/budget negatives and fractional/material witnesses remain in #127; #81 still owns production transport, publication and semantic authority.


### Original ToolbarHost diagnostic and supported domain

Main ran the unchanged original failing ToolbarHost case on the separately frozen diagnostic SDK after compile/strict passed. Under the existing build lock followed by GPU lock, the same refusal produced exactly one prepared-state trace: primitive index 9 is an ordinary rounded Quad, while the first B background is index 1. The Quad has actual scissor `[15,107,610,74]`; its parent conservative safe rectangle is `[21,21,619,267]`, so the horizontal containment checks both fail. The final Quad's logical left edge is `7.2` at scale `2` (`14.4px`), also outside the safe rectangle's `21px` left edge. Merely replacing the scissor with the Quad bounds would not establish this proof. Source and runtime inputs remained unchanged.

This is a limitation of the implemented finite Class A domain, which requires the complete prepared scissor inside each parent's conservative full-coverage safe rectangle. The abstract influence-footprint class above is a design objective; the current implementation does not prove every visually inset rounded control. The existing error is separately misleading: it tests whether a B already exists before establishing whether the current non-A Quad qualifies for B. The next private correction must distinguish a genuine second full-target flat background from a non-A/non-B rounded Quad, preserving proof-work charges, atomic refusal and the unchanged input. Neither case becomes accepted through that diagnostic correction.

Actual receipts: `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-127/toolbar-host-class-b-diagnostic-consumer-qa-20261026-r1/runs/main-diagnostic-{compile,gpu}-01/result.json`. This is diagnostic evidence, not product acceptance or a Root Class B pass. Original ToolbarHost combination remains not passed; a future finite proof extension would require the renderer's complete raster influence envelope and an accepted Scene-owner full-coverage predicate, with numeric/AA/budget evidence. It is not part of the error-classification fix.


The private R3 error-classification correction subsequently passed exactly the two affected Render negative tests (one actual test each), Render lib strict and the original consumer target's compile/strict. Replaying the unchanged original ToolbarHost test from its newly archived R3 binary under both original locks still exited 101, now with the accurate `prepared scissor outside parent full-coverage witness` refusal. The diagnostic harness expected and recorded that rejection; it is not a product-pass result. Those private receipts do not establish Root acceptance; current Root acceptance is recorded in the Current bounded acceptance section above. Prior positive GPU receipts retain their original R2 identities. Evidence is in `.../issue-127/toolbar-host-class-b-support-classification-20261026-r3/runs/main-{focused-validation,original-case-diagnostic}-01/result.json` under the SSD artifacts root.
