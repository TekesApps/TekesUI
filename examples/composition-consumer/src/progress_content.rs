//! Independent public consumer. Host layout allocates the bar; ordinary Text
//! siblings express business results separately from the original scalar model.
use crate::scroll_updates::{COMPOSER, LATEST, ROOT, SCROLLER, Scenario};
use tekes_ui_controls::{
    ButtonTextRenderer, ReadingDocumentVersion, composition::*, layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Size};
pub const ROW: ComponentId = ComponentId(58000);
pub const LABEL: ComponentId = ComponentId(58001);
pub const BAR: ComponentId = ComponentId(58002);
pub const RESULT: ComponentId = ComponentId(58003);
#[derive(Debug)]
struct ProgressRow {
    children: Vec<Content>,
}
impl Component for ProgressRow {
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
        .attach_reading("progress-transcript", 1)
        .expect("valid original reading document");
    let mut progress = ProgressContent::new("Transfer progress", 0.45);
    progress.description = Some("Transferred portion; result is reported separately".into());
    scenario.host.insert(
        ROOT,
        Content::new(
            ROW,
            ProgressRow {
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
/// No new progress reducer: caller supplies its result while the original
/// model owns finite/clamped numerical state and its existing unknown flag.
pub fn publish(host: &mut ContentHost, value: f32, unknown: bool, result: &str) {
    let progress = host
        .get_mut(BAR)
        .unwrap()
        .component_mut::<ProgressContent>()
        .unwrap();
    progress.model.set(value);
    progress.model.indeterminate = unknown;
    host.get_mut(RESULT)
        .unwrap()
        .component_mut::<TextContent>()
        .unwrap()
        .text = result.into();
}
