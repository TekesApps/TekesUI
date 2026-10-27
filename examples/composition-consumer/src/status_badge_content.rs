//! Public passive solid badge with ordinary Text, independent actions and original Composer.
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
struct StatusRow {
    children: Vec<Content>,
}
impl Component for StatusRow {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(available.width, 86_f32.min(available.height)))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        // This fixture allocates 28pt; this is not an SDK intrinsic height.
        let label = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, 24.);
        let bar = Rect::new(frame.min_x(), frame.min_y() + 28., frame.size.width, 28.);
        let result = Rect::new(frame.min_x(), frame.min_y() + 58., frame.size.width, 28.);
        for (child, allocation) in self.children.iter_mut().zip([label, bar, result]) {
            child.arrange(text, allocation, clip);
        }
    }
}
pub fn build() -> Result<(Scenario, ReadingDocumentVersion), CompositionError> {
    let mut scenario = Scenario::reading_document()?;
    scenario
        .host
        .get_mut(COMPOSER)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .submit_policy = TextSubmitPolicy::Enter;
    let version = scenario
        .attach_reading("badge-transcript", 1)
        .expect("valid original reading document");
    let badge = StatusBadgeContent::new(
        tekes_ui_controls::extended::StatusBadge::Info,
        "Awaiting verification",
    )?;
    scenario.host.insert(
        ROOT,
        Content::new(
            ROW,
            StatusRow {
                children: vec![
                    Content::new(LABEL, TextContent::new("Transfer")),
                    Content::new(BAR, badge),
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
/// Consumer boundary: reject malformed parent input before calling ContentHost,
/// whose existing clean_size policy otherwise sanitizes it. No SDK Host changes.
#[derive(Debug, PartialEq)]
pub enum RenderError {
    InvalidAllocation,
    Host(CompositionError),
}
pub fn render_checked(
    host: &mut ContentHost,
    scene: &mut tekes_ui_scene::Scene,
    text: &mut dyn ButtonTextRenderer,
    frame: Rect,
    clip: Rect,
    palette: tekes_ui_tokens::Palette,
) -> Result<(), RenderError> {
    for r in [frame, clip] {
        if !r.min_x().is_finite()
            || !r.min_y().is_finite()
            || !r.max_x().is_finite()
            || !r.max_y().is_finite()
            || !r.size.width.is_finite()
            || !r.size.height.is_finite()
            || r.size.width < 0.
            || r.size.height < 0.
        {
            return Err(RenderError::InvalidAllocation);
        }
    }
    host.render(scene, text, frame, clip, palette)
        .map_err(RenderError::Host)
}
pub fn publish(
    host: &mut ContentHost,
    model: tekes_ui_controls::extended::StatusBadge,
    label: &str,
    name: Option<String>,
    description: Option<String>,
    decorative: bool,
) -> Result<(), CompositionError> {
    host.get_mut(BAR)
        .unwrap()
        .component_mut::<StatusBadgeContent>()
        .unwrap()
        .update(model, label, name, description, decorative)
}
