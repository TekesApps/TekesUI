use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlState,
    composition::{ButtonContent, Component, ComponentId, Content, ContentHost, TextContent},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

#[derive(Default)]
struct Text(Vec<(String, Rect, ButtonTextAlign)>);
impl ButtonTextRenderer for Text {
    fn measure(&mut self, _: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new(40_f32.min(width), role.style().line_height)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        label: &str,
        _: proposal::TextRole,
        frame: Rect,
        align: ButtonTextAlign,
        _: Rgba,
    ) {
        self.0.push((label.into(), frame, align));
    }
}

#[test]
fn ordinary_composed_button_uses_design_default_in_every_existing_state() {
    let states = [
        ControlState::default(),
        ControlState {
            hovered: true,
            ..Default::default()
        },
        ControlState {
            focused: true,
            pressed: true,
            ..Default::default()
        },
        ControlState {
            focused: true,
            focus_visible: true,
            ..Default::default()
        },
        ControlState {
            disabled: true,
            ..Default::default()
        },
        ControlState {
            loading: true,
            ..Default::default()
        },
    ];
    for theme in Theme::ALL {
        for brand in 0..3 {
            for width in [120., 400.] {
                for state in states {
                    let mut button = ButtonContent::new("Apply");
                    button.button.state = state;
                    let mut host = ContentHost::new(Content::new(ComponentId(1), button)).unwrap();
                    let mut text = Text::default();
                    let frame = Rect::new(10., 20., width, 60.);
                    host.render(
                        &mut Scene::new(),
                        &mut text,
                        frame,
                        frame,
                        Palette::new(theme, brand).expect("registered Gallery brand"),
                    )
                    .unwrap();
                    assert_eq!(text.0.len(), 1);
                    let (_, content, align) = text.0[0];
                    assert_eq!(align, ButtonTextAlign::default());
                    assert_eq!(content.center().x, frame.center().x);
                    assert!(
                        content.size.width > 40.,
                        "fixture must expose excess label space"
                    );
                    assert_eq!(
                        host.get(ComponentId(1))
                            .unwrap()
                            .component::<ButtonContent>()
                            .unwrap()
                            .activations,
                        0
                    );
                }
            }
        }
    }
}

#[test]
fn body_text_keeps_its_independent_leading_alignment() {
    let mut text = Text::default();
    let mut body = TextContent {
        text: "Description".into(),
        role: proposal::TextRole::UiBody,
    };
    body.paint(
        &mut Scene::new(),
        &mut text,
        Rect::new(0., 0., 400., 40.),
        Rect::new(0., 0., 400., 40.),
        Palette::from(Theme::LIGHT).into(),
    );
    assert_eq!(text.0[0].2, ButtonTextAlign::Leading);
}
