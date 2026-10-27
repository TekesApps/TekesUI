//! Independent public API consumer; the original mixed reader/Composer Host.
use crate::scroll_updates::{COMPOSER, LATEST, ROOT, SCROLLER, Scenario};
use tekes_ui_controls::{ReadingDocumentVersion, composition::*, extended::EmptyState};
use tekes_ui_scene::{Rect, Size};
pub const MESSAGE: ComponentId = ComponentId(78010);
pub const ACTION: ComponentId = ComponentId(78011);
pub fn model(action: Option<&str>) -> EmptyState {
    EmptyState {
        title: "No documents · 暂无文档".into(),
        detail: "Create a document to begin. 👩🏽‍💻".into(),
        action: action.map(str::to_owned),
    }
}
pub fn build() -> Result<(Scenario, ReadingDocumentVersion), CompositionError> {
    let mut scenario = Scenario::reading_document()?;
    let version = scenario
        .attach_reading("empty-state-transcript", 1)
        .expect("valid original document");
    scenario.host.insert(
        ROOT,
        Content::new(
            MESSAGE,
            EmptyStateContent::new("Documents", model(Some("Create")), ACTION)?,
        ),
    )?;
    scenario
        .host
        .reorder(ROOT, &[SCROLLER, MESSAGE, LATEST, COMPOSER])?;
    Ok((scenario, version))
}
/// The public Host's finite-parent precondition is enforced before mutation.
/// Host clean_size alone would not be an atomic validation API.
pub fn set_allocation(current: &mut (Rect, Rect), frame: Rect, clip: Rect) -> bool {
    fn valid(r: Rect) -> bool {
        [
            r.min_x(),
            r.min_y(),
            r.max_x(),
            r.max_y(),
            r.size.width,
            r.size.height,
        ]
        .into_iter()
        .all(f32::is_finite)
            && r.size.width >= 0.
            && r.size.height >= 0.
    }
    if !valid(frame) || !valid(clip) {
        return false;
    }
    *current = (frame, clip);
    true
}
pub fn set_available(current: &mut Size, next: Size) -> bool {
    if !next.width.is_finite() || !next.height.is_finite() || next.width < 0. || next.height < 0. {
        return false;
    }
    *current = next;
    true
}
