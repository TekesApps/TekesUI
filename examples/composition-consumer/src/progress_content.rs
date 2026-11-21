//! Independent public consumer. Host layout composes the intrinsic-height bar
//! with ordinary Text siblings separately from the original scalar model.
use crate::scroll_updates::{COMPOSER, LATEST, ROOT, SCROLLER, Scenario};
use tekes_ui_controls::{ReadingDocumentVersion, composition::*};
use tekes_ui_tokens::Spacing;
pub const ROW: ComponentId = ComponentId(58000);
pub const LABEL: ComponentId = ComponentId(58001);
pub const BAR: ComponentId = ComponentId(58002);
pub const RESULT: ComponentId = ComponentId(58003);
pub fn build() -> Result<(Scenario, ReadingDocumentVersion), CompositionError> {
    let mut scenario = Scenario::reading_document()?;
    let version = scenario
        .attach_reading("progress-transcript", 1)
        .expect("valid original reading document");
    let progress = ProgressContent::new("Transfer progress", 0.45)
        .with_description("Transferred portion; result is reported separately");
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
/// No new progress reducer: caller supplies its result while the original
/// model owns finite/clamped numerical state and its existing unknown flag.
pub fn publish(host: &mut ContentHost, value: f32, unknown: bool, result: &str) {
    let progress = host
        .get_mut(BAR)
        .unwrap()
        .component_mut::<ProgressContent>()
        .unwrap();
    progress.set_value(value);
    progress.set_indeterminate(unknown);
    host.get_mut(RESULT)
        .unwrap()
        .component_mut::<TextContent>()
        .unwrap()
        .set_text(result);
}
