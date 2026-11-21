//! Independent public consumer. Host layout composes the intrinsic-height placeholder
//! with ordinary Text siblings separately from the original scalar model.
use crate::scroll_updates::{COMPOSER, LATEST, ROOT, SCROLLER, Scenario};
use tekes_ui_controls::{ReadingDocumentVersion, composition::*};
use tekes_ui_scene::Size;
use tekes_ui_tokens::Spacing;
pub const ROW: ComponentId = ComponentId(59000);
pub const LABEL: ComponentId = ComponentId(59001);
pub const BAR: ComponentId = ComponentId(59002);
pub const RESULT: ComponentId = ComponentId(59003);
pub fn build() -> Result<(Scenario, ReadingDocumentVersion), CompositionError> {
    let mut scenario = Scenario::reading_document()?;
    let version = scenario
        .attach_reading("skeleton-transcript", 1)
        .expect("valid original reading document");
    let progress = SkeletonContent::default();
    scenario.host.insert(
        ROOT,
        Content::new(
            ROW,
            StackContent::vertical(
                Spacing::Tight.value(),
                vec![
                    Content::new(LABEL, TextContent::new("Transfer")),
                    Content::new(BAR, progress),
                    Content::new(RESULT, TextContent::new("Awaiting verification")),
                ],
            ),
        ),
    )?;
    scenario
        .host
        .reorder(ROOT, &[SCROLLER, ROW, LATEST, COMPOSER])?;
    Ok((scenario, version))
}
/// Caller-owned business message; Skeleton has no loading/result state machine.
pub fn publish(host: &mut ContentHost, result: &str) {
    host.get_mut(RESULT)
        .unwrap()
        .component_mut::<TextContent>()
        .unwrap()
        .set_text(result);
}
/// Validate external parent input before replacing the accepted allocation.
pub fn set_allocation(retained: &mut Size, candidate: Size) -> bool {
    if !candidate.width.is_finite()
        || !candidate.height.is_finite()
        || candidate.width < 0.
        || candidate.height < 0.
    {
        return false;
    }
    *retained = candidate;
    true
}
