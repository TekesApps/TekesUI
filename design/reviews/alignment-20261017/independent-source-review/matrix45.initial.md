# Independent alignment source review

Read-only source and geometry-risk review. No native app, screen, build or test suite invoked. Source was concurrently changing. Findings require final source reread.

| Component | Source | Code observations | New pixels | Real native interaction |
|---|---|---|---|---|
| Stack | `crates/tekes-ui-controls/src/layout_view.rs` | Existing equal-width/equal-height Stack API remains; COLUMN/TEXT_ROW policy not integrated into it. | 未测 | 未测 |
| Grid | `crates/tekes-ui-controls/src/layout_view.rs` | Fixed cell layout remains, ancestor clip owns overflow; not all responsive states tested. | 未测 | 未测 |
| Separator | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| SplitView | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| ScrollArea | `crates/tekes-ui-controls/src/scroll_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Disclosure | `crates/tekes-ui-controls/src/layout_view.rs` | Shared measured row now consumed; no baseline claim: icon and paragraph boxes use center alignment. | 未测 | 未测 |
| Surface / Panel | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Toolbar | `crates/tekes-ui-controls/src/feedback_view.rs` | Equal width buttons; fixed Gallery surface/frame36 may clip wrapped/very large labels. Default single-line clipping was retracted after actual token check. | 未测 | 未测 |
| Text | `crates/tekes-ui-controls/src/text_surface.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Icon | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Button | `crates/tekes-ui-controls/src/widgets.rs` | Shaped measurement and draw use same constrained content width; policy BUTTON_CONTENT remains unconsumed. | 未测 | 未测 |
| IconButton | `crates/tekes-ui-controls/src/layout_view.rs` | Tint follows disabled/foreground semantics; IconView manually centers artboard; ICON_ARTBOARD policy not consumed. | 未测 | 未测 |
| Link | `crates/tekes-ui-controls/src/layout_view.rs` | Gallery label includes external-arrow glyph suffix; not part of a fixed independent icon slot. | 未测 | 未测 |
| TextField | `crates/tekes-ui-controls/src/text_surface.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| TextArea | `crates/tekes-ui-controls/src/text_surface.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| SearchField | `crates/tekes-ui-controls/src/text_surface.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| ComboBox | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Slider | `crates/tekes-ui-controls/src/selection.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Checkbox | `crates/tekes-ui-controls/src/widgets.rs` | Control aligned to actual first line height through renderer adapter, not paragraph midpoint; baseline API not used. | 未测 | 未测 |
| RadioGroup | `crates/tekes-ui-controls/src/selection.rs` | Indicator follows first line box center; no typographic baseline claim. | 未测 | 未测 |
| Switch | `crates/tekes-ui-controls/src/selection.rs` | Labeled presentation checks first line height; no typographic baseline claim. | 未测 | 未测 |
| SegmentedControl | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Field | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Select | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| NumberField | `crates/tekes-ui-controls/src/feedback_view.rs` | Finding AR-03: standalone plus/minus action labels request Leading alignment in 48pt buttons. | 未测 | 未测 |
| Tabs | `crates/tekes-ui-controls/src/composite.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Breadcrumb | `crates/tekes-ui-controls/src/layout_view.rs` | Text slash separator reviewed as semantic text; remaining width grows/shrinks per measured item. | 未测 | 未测 |
| CommandPalette | `crates/tekes-ui-controls/src/layout_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| List | `crates/tekes-ui-controls/src/collections_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Tree | `crates/tekes-ui-controls/src/collections_view.rs` | Branch glyph replacement observed during review; latest patch has stale icon_slots risk on early hidden render. Recheck after worker finalizes. | 未测 | 未测 |
| DataTable | `crates/tekes-ui-controls/src/collections_view.rs` | Finding AR-01: final decorated header labels differ from measurement; first row remains hardcoded y+40 regardless of shaped header height. | 未测 | 未测 |
| Chart | `crates/tekes-ui-controls/src/collections_view.rs` | Measured labels now use same final bar/category widths and centered text alignment; positive/negative rows checked in source only. | 未测 | 未测 |
| Menu | `crates/tekes-ui-controls/src/composite.rs` | Per-row Button measure drives vertical pitch; viewport-aware popup and shared hit layouts inspected. | 未测 | 未测 |
| Tooltip | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Popover | `crates/tekes-ui-controls/src/feedback_view.rs` | Spacing formula now preserves content height for unconstrained measure/render; viewport clamp still requires runtime narrow/multiline verification. | 未测 | 未测 |
| Dialog | `crates/tekes-ui-controls/src/composite.rs` | Title measurement and clip separated from measured actions; all real input states untested. | 未测 | 未测 |
| Toast | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Banner | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| StatusBadge | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Progress | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| EmptyState | `crates/tekes-ui-controls/src/feedback_view.rs` | Finding AR-02: title measured at content_width but drawn at min(content_width,420); action is clamped upward for short viewports and can overlap detail. | 未测 | 未测 |
| Skeleton | `crates/tekes-ui-controls/src/feedback_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| Markdown | `crates/tekes-ui-controls/src/collections_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| CodeBlock | `crates/tekes-ui-controls/src/collections_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |
| DiffView | `crates/tekes-ui-controls/src/collections_view.rs` | No additional deterministic alignment defect established in this source pass; all-state acceptance not claimed. | 未测 | 未测 |

## Evidence limits

Row preset consumed by Disclosure render_icon_label. ChartLabel.text_alignment consumed by ChartView. No production usage found for Column/TextRow/ButtonContent/IconArtboard; FirstBaseline is helper/unit-test only. Generated API availability does not establish component adoption.

No all-state pass is claimed. Before comparison source is target/alignment-review-20261017/before-sources. Prior snapshot-final is baseline inventory, not correctness approval.
