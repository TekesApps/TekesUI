//! Original ContentHost/DialogContent fixture shared by data and native probes.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::{
        Button, ComponentId, Content, ContentHost, DialogContent, ListContent, SwitchContent,
    },
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};

pub const ROOT: ComponentId = ComponentId(9100);
pub const INVOKER: ComponentId = ComponentId(9101);
pub const DIALOG: ComponentId = ComponentId(9102);
pub const HEADER: ComponentId = ComponentId(9103);
pub const CHOICE: ComponentId = ComponentId(9104);
pub const CANCEL: ComponentId = ComponentId(9105);

pub fn host() -> ContentHost {
    ContentHost::new(Content::new(
        ROOT,
        ListContent::new(
            "Window content",
            vec![
                Content::new(INVOKER, Button::new("Open modal")),
                Content::new(
                    DIALOG,
                    DialogContent::new(
                        "Modal scope",
                        Content::new(HEADER, ListContent::new("Modal header", vec![])),
                        Content::new(CHOICE, SwitchContent::new("Modal choice", false)),
                        Content::new(CANCEL, Button::new("Dismiss modal")),
                    ),
                ),
            ],
        ),
    ))
    .unwrap()
}

pub fn paint(host: &mut ContentHost) {
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
            Size::new(
                (value.chars().count() as f32 * 8.).min(width),
                role.style().line_height,
            )
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    let frame = Rect::new(0., 0., 800., 600.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
