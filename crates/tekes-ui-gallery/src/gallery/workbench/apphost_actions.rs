//! Original button icon fixture helper.
use tekes_ui_controls::{ButtonTextRenderer, composition::*, layout::MeasuredChild};
use tekes_ui_scene::{Rect, Scene, Size};

pub(super) fn icon_button(state: tekes_ui_controls::ControlState) -> Button {
    let mut button = Button::new(Content::new(
        ComponentId(122001),
        IconContent::new("tekes-new").expect("canonical plus glyph"),
    ))
    .with_accessible_name("Create item");
    button.button.state = state;
    button
}
#[derive(Debug)]
pub(super) struct IconFixture(Button);
impl IconFixture {
    pub(super) fn new(state: tekes_ui_controls::ControlState) -> Self {
        Self(icon_button(state))
    }
}
impl Component for IconFixture {
    fn requires_icon_services(&self) -> bool {
        true
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        self.0.children()[0]
            .component::<IconContent>()
            .expect("original icon slot")
            .icon_error()
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.0
            .measure(&mut super::apphost_basic::FixtureText(text), available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.0.arrange(text, frame, clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.0.paint(
            scene,
            &mut super::apphost_basic::FixtureText(text),
            frame,
            clip,
            theme,
        );
    }
}
