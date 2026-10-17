# Independent alignment source review

Independent read-only source/geometry-risk review across 45 registered component routes and focused follow-up fixes. No native app, screen, build or test suite invoked by this reviewer. Final source hashes captured after parent freeze; parent owns final suite, offscreen pixel and input validation.

final source snapshot after parent-confirmed last icon-card freeze; limited icon-card follow-up complete

## Findings history

- **AR-01 / data-table / resolved_by_source_review**: Original header measured before sort glyph decoration, with fixed first row y+40; wrapped header overlapped rows. Build final header_labels before measure; shared header_band drives all header frames; row_start=header_band+6 and content_height consume row_start.
- **AR-02 / empty-state / resolved_by_source_review**: Title measured at unconstrained content_width but drawn at min(width,420); short frame clamped action upward over detail. title_width=min(content_width,420) used for both; action remains after detail when it cannot fit. Parent clips overflow; no overlap fallback.
- **AR-03 / number-field / resolved_by_source_review**: Standalone plus/minus buttons requested Leading alignment. Both use button_text_align(BUTTON_CONTENT), whose current text_alignment is center.
- **AR-04 / tree / resolved_by_source_review**: Branch chevrons were glyph prefixes in the row label. First replacement retained stale icon_slots on hidden early return. Node label measured/drawn separately from fixed branch icon slot; icon_slots.clear() precedes all early exits.
- **AR-05 / tree / resolved_by_source_review**: New separate render_icons pass does not retain the frame/own_clip used by render. Offscreen branch rows remain in icon_slots and can paint outside Tree frame. TreeIconSlot retains button_layout.visible. render_icons intersects original row visible with caller clip, skips invisible rows, and keeps partial-row clipping; slots clear at render entry.
- **AR-06 / list/tree Gallery adapter / resolved_by_source_review**: New instruction helper measured wrapped caption height but painted via unwrapped one-line line helper. place_instruction_below_rows now shapes proposal_label(Caption,Some(bounds.width)) and paints proposal_wrapped_in at same width/role; starts after measured row bottom; omits note if viewport cannot contain it.
- **AR-07 / toast Gallery adapter and production view / resolved_by_source_review**: Toast dismiss/message dimensions and fixed fixture controls could clip/overlap after large text growth; measured multiline fixture captions still used one-line painting. ToastView::measure/render use same dismiss/action and message width. Gallery frame consumes measured height, controls start after toast bottom, note follows max(action,timer) bottom; captions exceeding legacy line slot use same-width wrapped painting.
- **AR-08 / icon catalog Gallery cards / resolved_by_source_review**: Parent PNG review found a localized three-line caption colliding with the preview after type scaling; card height, scroll extent and lazy preparation needed shared scaled geometry. scaled_tile_height=148*normalized_type_scale; scaled_row_step adds existing 16pt gap. build_catalog, extent_at_scale and visible_legacy_at_scale share tile_rect_at_scale/scaled_row_step. Existing extent/visible_legacy wrappers pass 1.0. Gallery prepare and extent derive scale directly from Workbench model, while build sets TextEngine from the same model before drawing. Caption/title are measured and wrapped at matching widths.

## Policy consumption

Production geometry consumers: Disclosure render_icon_label consumes Row.main_axis/cross_axis through layout_measured_children; IconView consumes IconArtboard.main_axis/cross_axis through the helper. Text consumers: NumberField stepper consumes ButtonContent.text_alignment, ChartView consumes ChartLabel.text_alignment. Column and TextRow remain helper/unit-test only. Row.text_alignment, IconArtboard.text_alignment, ButtonContent.main_axis/cross_axis and ChartLabel.main_axis/cross_axis are not production consumers. FirstBaseline has helper/tests only; first-line-box indicator alignment in Checkbox/RadioGroup/Switch is not typographic baseline alignment. No 45-component adoption claim.

## 45 component source matrix

| Component | Source | Current source observation | Independently accepted pixels | Real native interaction |
|---|---|---|---|---|
| Stack | `crates/tekes-ui-controls/src/layout_view.rs` | Existing equal-width/equal-height Stack API remains; COLUMN/TEXT_ROW policy not integrated into it. | 未测 | 未测 |
| Grid | `crates/tekes-ui-controls/src/layout_view.rs` | Fixed cell layout remains, ancestor clip owns overflow; not all responsive states tested. | 未测 | 未测 |
| Separator | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| SplitView | `crates/tekes-ui-controls/src/layout_view.rs` | Gallery explanatory caption uses measured same-width wrapped slot after actual pane bounds. No full-state/native acceptance. | 未测 | 未测 |
| ScrollArea | `crates/tekes-ui-controls/src/scroll_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Disclosure | `crates/tekes-ui-controls/src/layout_view.rs` | Row main/cross geometry consumed; icon and paragraph boxes use center alignment. FirstBaseline not involved. | 未测 | 未测 |
| Surface / Panel | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Toolbar | `crates/tekes-ui-controls/src/feedback_view.rs` | Shared measurement/draw routing inspected; no additional deterministic defect established. Earlier default 4px clipping hypothesis retracted after actual size-action-button=24 check. | 未测 | 未测 |
| Text | `crates/tekes-ui-controls/src/text_surface.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Icon | `crates/tekes-ui-controls/src/layout_view.rs` | IconView consumes IconArtboard main/cross via measured helper. AR-08 catalog fix source-verified: card height 148*type_scale and row step height+16 shared by drawing, scroll extent and lazy asset preparation; Workbench model scale prevents stale first-frame TextEngine scale. No independent pixel/full-state acceptance. | 未测 | 未测 |
| Button | `crates/tekes-ui-controls/src/widgets.rs` | Shaped measurement/draw use same constrained width. General Button does not consume ButtonContent; current consumer is NumberField stepper only. | 未测 | 未测 |
| IconButton | `crates/tekes-ui-controls/src/layout_view.rs` | IconView consumes IconArtboard main/cross; Gallery explanatory caption uses available body width and actual paragraph height. No baseline or all-state claim. | 未测 | 未测 |
| Link | `crates/tekes-ui-controls/src/layout_view.rs` | Gallery explanatory caption measured/wrapped at available body width. External arrow remains text suffix; no new deterministic defect established in scoped follow-up. | 未测 | 未测 |
| TextField | `crates/tekes-ui-controls/src/text_surface.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| TextArea | `crates/tekes-ui-controls/src/text_surface.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| SearchField | `crates/tekes-ui-controls/src/text_surface.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| ComboBox | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Slider | `crates/tekes-ui-controls/src/selection.rs` | Gallery explanatory caption uses measured same-width wrapped slot below control. No full-state/native acceptance. | 未测 | 未测 |
| Checkbox | `crates/tekes-ui-controls/src/widgets.rs` | Control aligned to actual first line height through renderer adapter, not paragraph midpoint; baseline API not used. | 未测 | 未测 |
| RadioGroup | `crates/tekes-ui-controls/src/selection.rs` | Indicator follows first line box center; no typographic baseline claim. | 未测 | 未测 |
| Switch | `crates/tekes-ui-controls/src/selection.rs` | Labeled presentation checks first line height; no typographic baseline claim. | 未测 | 未测 |
| SegmentedControl | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Field | `crates/tekes-ui-controls/src/feedback_view.rs` | Production Field label/slot/message geometry inspected. Parent final Gallery phase/event-log occupation repair is hash-tracked and validated by parent, not independently re-reviewed. | 未测 | 未测 |
| Select | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| NumberField | `crates/tekes-ui-controls/src/feedback_view.rs` | AR-03 fixed: minus/plus consumes ButtonContent.text_alignment (currently Center); value remains Leading. Parent final Gallery note-wrap change is hash-tracked, not independently re-reviewed. | 未测 | 未测 |
| Tabs | `crates/tekes-ui-controls/src/composite.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Breadcrumb | `crates/tekes-ui-controls/src/layout_view.rs` | Text slash separator reviewed as semantic text; remaining width grows/shrinks per measured item. | 未测 | 未测 |
| CommandPalette | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| List | `crates/tekes-ui-controls/src/collections_view.rs` | AR-06 fixed in source: Gallery instruction is measured and painted wrapped with same width/role after actual row bottom; omitted when it cannot fit. No all-state/native acceptance. | 未测 | 未测 |
| Tree | `crates/tekes-ui-controls/src/collections_view.rs` | AR-04/05 fixed in source: canonical branch icon separate from label; slots clear before early exit; separate icon pass retains original visible row clip and intersects caller clip. No all-state/native acceptance. | 未测 | 未测 |
| DataTable | `crates/tekes-ui-controls/src/collections_view.rs` | AR-01 fixed in source: final decorated header label is measured and drawn; shared header_band drives row_start and content_height. Sort arrow remains a text glyph. No pixel or native acceptance. | 未测 | 未测 |
| Chart | `crates/tekes-ui-controls/src/collections_view.rs` | Measured labels use same final bar/category widths and ChartLabel.text_alignment; main/cross fields unused. No pixel acceptance. | 未测 | 未测 |
| Menu | `crates/tekes-ui-controls/src/composite.rs` | Per-row Button measure drives vertical pitch; viewport-aware popup and shared hit layouts inspected. | 未测 | 未测 |
| Tooltip | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Popover | `crates/tekes-ui-controls/src/feedback_view.rs` | Spacing formula now preserves content height for unconstrained measure/render; viewport clamp still requires runtime narrow/multiline verification. | 未测 | 未测 |
| Dialog | `crates/tekes-ui-controls/src/composite.rs` | Title measurement and clip separated from measured actions; all real input states untested. | 未测 | 未测 |
| Toast | `crates/tekes-ui-controls/src/feedback_view.rs` | AR-07 fixed in source: measured production toast frame, dismiss/message same-width geometry, Gallery controls/notes follow actual bottom; long captions wrap at measured width. | 未测 | 未测 |
| Banner | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| StatusBadge | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Progress | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| EmptyState | `crates/tekes-ui-controls/src/feedback_view.rs` | AR-02 fixed in source: title_width shared by measure/draw; constrained action remains below detail and may be clipped. No all-size visual acceptance. | 未测 | 未测 |
| Skeleton | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Markdown | `crates/tekes-ui-controls/src/collections_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| CodeBlock | `crates/tekes-ui-controls/src/collections_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| DiffView | `crates/tekes-ui-controls/src/collections_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |

## Evidence limits

- code: 45-route source mapping/pattern pass and named follow-up fixes; no claim that all contracts or states were exhaustively reviewed
- pixels: No new pixels independently accepted by this reviewer; parent is running final offscreen render validation
- interaction: No native input/screen operation by reviewer; parent owns final real-input offscreen fixtures
- old_snapshots: snapshot-final is before baseline inventory only, not correctness evidence
- hash_scope: Source hash refreshed for known reviewed files plus parent-final workbench.rs/icons.rs. Hash refresh itself does not imply new code review or passing tests.

No open deterministic finding remains from this independent source review. This does not mean 45 components or all states passed acceptance. Initial observations remain in matrix45.initial.json; findings retain all named history. Before source: target/alignment-review-20261017/before-sources.
