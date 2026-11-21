//! Original button icon fixture helper.
use tekes_ui_controls::{ButtonTextRenderer, composition::*, layout::MeasuredChild};
use tekes_ui_scene::{Rect, Scene, Size};

pub(super) fn icon_button(disabled: bool) -> Button {
    Button::new(Content::new(
        ComponentId(122001),
        IconContent::new("tekes-new").expect("canonical plus glyph"),
    ))
    .with_accessible_name("Create item")
    .with_disabled(disabled)
}
#[derive(Debug)]
pub(super) struct IconFixture {
    button: Button,
    state: tekes_ui_controls::ControlState,
    applied: bool,
}
impl IconFixture {
    pub(super) fn new(state: tekes_ui_controls::ControlState) -> Self {
        let button = icon_button(state.disabled)
            .with_loading(state.loading)
            .with_selected(state.selected);
        Self {
            button,
            state,
            applied: false,
        }
    }
}
impl Component for IconFixture {
    fn requires_icon_services(&self) -> bool {
        true
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        self.button.children()[0]
            .component::<IconContent>()
            .expect("original icon slot")
            .icon_error()
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.button
            .measure(&mut super::apphost_basic::FixtureText(text), available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.button.arrange(text, frame, clip);
        if !self.applied {
            self.applied = true;
            if self.state.hovered {
                self.button
                    .event(&ContentEvent::PointerMove(frame.center()));
            }
            if self.state.pressed {
                self.button
                    .event(&ContentEvent::PointerDown(frame.center()));
            } else if self.state.focused {
                self.button.set_focus(true, self.state.focus_visible);
            }
        }
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.button.paint(
            scene,
            &mut super::apphost_basic::FixtureText(text),
            frame,
            clip,
            theme,
        );
    }
}
