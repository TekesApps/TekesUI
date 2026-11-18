# `*View` / Multi-Paradigm Control Consolidation Roadmap

> Tracking issue: `TekesApps/TekesUI#334` (Parent `#332`, Audit `#311`)
> Status: Stage 1 executed in v0.3.34 Batch 4C; Stages 2–3 tracked for subsequent releases.

## Architectural Goal

TekesUI currently exposes three generations of UI types across `tekes_ui_controls`:

1. Stateless / state-only model structs (`ButtonModel`, `SplitView`, `Stack`, `Switch`, ...)
2. Imperative `*View` presenter/layout adapters (`layout_view::*View`, `feedback_view::*View`, `collections_view::*View`)
3. Managed `ContentHost` tree components (`composition::*Content`, `composition::Button`)

The long-term public surface centers on `composition::*` (plus `layout`, `extended` pure geometry/models where still shared), with legacy `*View` presenters progressively internalized once external Gallery exhibits and consumer tests migrate to `*Content`.

---

## Stage 1 — Completed in v0.3.34 (`#334`)

1. **Dead Duplicate Deletion**:
   - Removed unused `tekes_ui_settings::SelectableListContent` (`crates/tekes-ui-settings/src/sidebar_list.rs`).
   - Removed unused `tekes_ui_controls::collections_view::DiffViewPresenter` alias.
2. **Low-Level Paint & Precheck Internalization**:
   - Changed `tekes_ui_controls::paint` from `pub mod paint` to private `mod paint` while keeping `ButtonVariant` and `SurfaceKind` at the crate root, exposing typed painting helpers on `SurfaceView`, `FocusRingMetrics`, and `SkeletonView`, and eliminating runtime `.expect(...)` panics from `paint.rs`.
   - Internalized `RequiredFocusEvidence` (`pub(crate)`) and all 13 host transaction precheck types (`pub(crate)`), replacing the 6 transaction precheck methods on `pub trait Component` with declarative hooks (`has_stable_hooks`, `supports_prepared_lifecycle`, `can_accept_window_active_transitions`, `ignores_pointer_actions`, `delegated_owner`, `delegated_owner_mut`) and `composition/precheck_dispatch.rs`.
3. **Root Namespace & Canonical Module Consolidation**:
   - Removed duplicated `*Content` / focus-contract re-exports from `tekes_ui_controls` root (`BreadcrumbContent`, `CommandPaletteContent`, `TabItem`, `TabsContent`, `ChartContent`, `ChartContentError`, `DisclosureContent`, `DisclosureIntent`, `RequiredFocusResult`, `RequiredFocusSpace`), keeping them solely under `tekes_ui_controls::composition`.
   - Moved `*View` / `*Layout` / `*Event` re-exports out of `tekes_ui_controls` root into their domain view modules (`feedback_view::{Dialog*, Menu*}`, `layout_view::{Tabs*, RadioGroup*, SelectionEvent, Slider*, Switch*}`).
4. **Module Decomposition**:
   - Split the 7 monolithic multi-thousand-line files (`composition.rs`, `layout_view.rs`, `composite.rs`, `collections_view.rs`, `feedback_view.rs`, `widgets.rs`, `extended.rs`) and `composition/text_field.rs` tests into focused `<stem>/*.rs` submodules (`< 1,600` lines each).

---

## Stage 2 — View-Leak Decoupling

Before remaining internal layout/popup adapters can become `pub(crate)` without triggering `private_interfaces` errors or breaking `tekes-ui-gallery` / `examples/composition-consumer`:

1. **`FilteredMenuView` (`layout_view`)**:
   - Replace `CommandPaletteContent::options(&self) -> &FilteredMenuView` with semantic query accessors (`options_layout(&self) -> Option<&MenuLayout>`, `options_presented(&self) -> bool`), and encapsulate `ComboBoxView.options` / `CommandPaletteView.options` so `FilteredMenuView` moves to `pub(crate)`.
2. **`SplitViewLayout` (`extended::SplitViewLayout` & `layout_view::SplitViewLayout`)**:
   - Unify the duplicate `extended::SplitViewLayout` and `layout_view::SplitViewLayout` structs, replace `SplitViewContent::layout(&self) -> Option<SplitViewLayout>` with semantic geometry queries (`divider_frame()`, `divider_hit()`, `pane_bounds(...)`), and demote `SplitViewLayout` to `pub(crate)`.
3. **`ChartViewLayout` (`collections_view`)**:
   - Return `ChartGeometry` / `Rect` queries directly from `ChartView` and `ChartContent`, demoting `ChartViewLayout` to `pub(crate)`.

---

## Stage 3 — `workbench.json` Secondary `*View` Binding Retirement & Full `*View` Internalization

1. **Gallery Story Migration**:
   - Migrate the remaining Gallery component exhibits in `crates/tekes-ui-gallery/src/gallery/workbench/` from imperative `*View` presenters to `ContentHost` + `composition::*Content` owners, updating `design/gallery/component-coverage.json` anchors.
2. **Design Registry Transition**:
   - Retire secondary `*View` `api_bindings` in `design/gallery/workbench.json` once `component-coverage.json` state observations run through `*Content`.
3. **Final Demotion**:
   - Demote legacy `*View` structs in `layout_view`, `feedback_view`, and `collections_view` to `pub(crate)` once `workbench.json` and `examples/*` no longer reference them externally.
