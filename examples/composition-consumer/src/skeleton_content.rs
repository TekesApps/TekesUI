//! Independent public consumer. Host layout allocates the placeholder; ordinary Text
//! siblings express business results separately from the original scalar model.
use crate::scroll_updates::{COMPOSER, LATEST, ROOT, SCROLLER, Scenario};
use tekes_ui_controls::{
    ButtonTextRenderer, ReadingDocumentVersion, composition::*, layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Size};
pub const ROW: ComponentId = ComponentId(59000);
pub const LABEL: ComponentId = ComponentId(59001);
pub const BAR: ComponentId = ComponentId(59002);
pub const RESULT: ComponentId = ComponentId(59003);
#[derive(Debug)]
struct SkeletonRow {
    children: Vec<Content>,
}
impl Component for SkeletonRow {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(available.width, 72_f32.min(available.height)))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        // This fixture reserves 12pt. It is a parent allocation, not SDK intrinsic size.
        let label = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, 24.);
        let bar = Rect::new(frame.min_x(), frame.min_y() + 28., frame.size.width, 12.);
        let result = Rect::new(frame.min_x(), frame.min_y() + 44., frame.size.width, 28.);
        for (child, allocation) in self.children.iter_mut().zip([label, bar, result]) {
            child.arrange(text, allocation, clip);
        }
    }
}
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
            SkeletonRow {
                children: vec![
                    Content::new(LABEL, TextContent::new("Transfer")),
                    Content::new(BAR, progress),
                    Content::new(RESULT, TextContent::new("Awaiting verification")),
                ],
            },
        ),
    )?;
    scenario
        .host
        .reorder(ROOT, &[SCROLLER, ROW, LATEST, COMPOSER])?;
    Ok((scenario, version))
}
/// Caller-owned business message; Skeleton has no loading/result state machine.
pub fn publish(host: &mut ContentHost, result: &str) {
    host.get_mut(RESULT).unwrap().component_mut::<TextContent>().unwrap().text = result.into();
}
/// Validate external parent input before replacing the accepted allocation.
pub fn set_allocation(retained: &mut Size, candidate: Size) -> bool {
    if !candidate.width.is_finite() || !candidate.height.is_finite()
        || candidate.width < 0. || candidate.height < 0. { return false; }
    *retained = candidate;
    true
}
