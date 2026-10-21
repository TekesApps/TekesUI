# Issue31 shared label layout integration

Root is the sole shared implementation owner. The root cause is missing readonly label policy, rather than Button input logic or Agent business state. The latest Issue31 remains open. ClientKit source was rechecked at c289bc9d33edcd83a782ac17927a8c65a027eccd: SummarySectionHeaderButton uses one line, native tail truncation and the complete original accessibility title. No Agent view/state or Swift implementation was copied into TekesUI.

The public opt-in is:

```rust
use tekes_ui_controls::composition::{ComponentId, Content, TextContent};
use tekes_ui_text::{LabelOptions, LabelOverflow};
let title = Content::new(ComponentId(1), TextContent::new(full_original_title))
    .with_label_layout(LabelOptions::single_line(LabelOverflow::TailEllipsis))?;
```

Apply the same builder to ButtonContent. For CompoundButton, apply it to the decorative TextContent slot; the original compound action retains its explicit complete accessible name. TextContent/ButtonContent struct literals and the default natural wrapping path remain compatible. Arbitrary containers and editable surfaces reject direct readonly label policy with UnsupportedLabelLayout. Such geometry must never replace editing selection/caret geometry. CompositionError adds MissingLabelServices and UnsupportedLabelLayout: exhaustive external enum matches require adaptation; no binary compatibility is claimed.

The implementation reuses CoreTextShaper, the existing font registry/rasterizer and TextSurfaceHost::push_visible. Hosts optionally implement TextSurfaceHost::shape_label and return Rc<LabelLayout>, applying the existing design role's family, size, weight and scaled minimum line height. The private Content adapter keeps one bounded derived layout entry. Paint borrows the exact measured Paragraph; it never reshapes at the resulting glyph width or replaces original source/AX text. Tail retains glyph bearings within the original component clip; Clip explicitly clips the label slot. The host's established integer-point label origin remains unchanged. JSON design/source/interaction-contracts.json is the semantic authority; existing token values, sets and token API remain unchanged.

Two genuine regressions were found and fixed during actual integration:

- Intrinsic Stack converts native f64 advances into f32 slots. Rounding downward incorrectly wrapped/elided a fitting last grapheme. Intrinsic width now rounds upward at the existing label service's1/64pt fit precision, bounded by the supplied slot. The independent real Stack regression failed before the fix;9 scale/width/repeated-layout cases now pass. Native wide-window before/after images are retained.
- Dialog slots measure descendants during arrange. A previous missing-capability error could block recovery before that arrange. ContentHost now discards derived layouts and old capability failures at every frame entry, as well as new measure passes. A real three-slot Dialog regression failed before the fix and now passes with a restored host. Original component state, focus and events are retained.

The authoritative local results are in design/reviews/label-content-integration-20261021/post-dialog-recovery:

-426controls,179independent consumer and50text tests; strict all-target Gallery and consumer Clippy.
-144 actual public TextContent/ButtonContent/CompoundButton combinations across themes, three font scales and repeated narrow/wide layouts. The glyph sink checks exact measured-object identity and full AX names; it is not native rendering evidence. Additional tests cover capability loss/recovery, scale invalidation, original activation/disabled/loading behavior and unsupported placement.
-432 actual Gallery cases/864GPU frames across normal, hover, pressed, visible focus, disabled/loading, four themes, two backing scales and three type scales. They match an independent original-renderer literal `A…` reference while the opt-in side retains `A\nB` as source/AX name. The separate core tests cover mixed-language/grapheme/native truncation geometry.
-17 canonical actual token observations,6432bounded cases, rerun against the final shared sources. complete=false remains explicit. Earlier receipts and native observations are historical; no maturity or whole-library native acceptance is inferred.

The minimal consumer is examples/composition-consumer/src/label_content.rs, selected by `native label` or TEKESUI_COMPONENT_CASE=label. scripts/component-review.py accepts this case. The current round uses the same lock/cache/owned-cleanup policy with a separately named app, preserving an older open input-family round. Final native readback includes button counts and actual state. Command+1/2/3 selects1x/1.5x/2x type scale; Command+4/5/0 selects disabled/loading/normal in this test host only. These hooks do not change shared production control logic.

Remaining acceptance: isolated545 actual consumer/native and matched Gold7/ClientKit product-page experience, fixed public source/CI and receiver verdict. Root22caa native key/pointer/focus/AX and narrow/wide three-scale checks have passed as detailed below; they are not relabelled as545 acceptance. Native snapshots record actual window_active; tool Raise/title click on earlier versions remained inactive and cannot prove input acceptance. VoiceOver stays off. No performance tests, commit, push, tag, merge or Issue closure were performed. Existing frozen AgentKit packages were not modified.

Before copies, failed checks, regression logs, pictures, scoped source snapshots and exact native binary/source identities remain in this review directory. Scoped evidence snapshots are explicitly not standalone distribution packages. Cleanup removes only unused owned outputs; a live or pending native app is kept until verification finishes.


Final root native receipt:26 actual active external U2790B@2 checkpoints on identity22caa4f3b5a3a52ac7d9c5e18db575abf70950ca05c28ea52f914293dc39ebf0. Two original actions end with4activations each after keyboard, coordinate pointer and AX paths; disabled/loading/outside-hit attempts do not increment.6width/type-scale contexts preserve full names and one action owner per Button. A separate public-API replay matches104actual node frames exactly, including native fallback-font rounding (1.5x row is25pt, not a forced25.5pt). Source-replay logs and native-acceptance.json remain separate from the independent CoreText/GPU pixel oracles. Tool-provided native screenshots are JPEG, renamed to .jpg without re-encoding; GPU screenshots remain lossless PNG. The native focus ring was visually reviewed as continuous at narrow width. Gold7 scoped results are in gold7-label-delta.json; full-page and matched ClientKit visuals remain pending. VoiceOver was confirmed off; no hearing claim is made. The app exited and its23MB owned bundle and4.8MiB replay package outputs were cleaned; source/evidence/shared libraries and older pending native rounds remain.

The isolated545 candidate has now passed exact-archive sender clean-extract replay:50text/388controls/176consumer tests,432coreGPU cases/864frames,strictClippy and native example build. All476source files and locks remained unchanged,all local dependencies resolved inside the extract,and owned outputs were cleaned. Its21path delta and explicit scope/identity boundaries are documented in design/reviews/label-portable-candidate-20261021-attempt2/HANDOFF.md. This candidate and evidence were sent to the authorized Rust AgentKit thread for independent actual integration/native review without changing its current freeze.
