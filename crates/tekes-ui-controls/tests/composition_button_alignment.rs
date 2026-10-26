use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlState,
    composition::{ButtonContent, Component, ComponentId, Content, ContentHost, TextContent},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

#[derive(Default)]
struct Text(Vec<(String, Rect, ButtonTextAlign)>, Vec<Rgba>);
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
        ink: Rgba,
    ) {
        self.0.push((label.into(), frame, align));
        self.1.push(ink);
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

#[test]
fn public_button_rendered_text_and_background_meet_declared_contrast() {
    // Existing contrast-pairs.json: text 4.5 (standard), 7 (increased).
    // Inspect the public component's actual draw ink and Scene fill, not a
    // separate Surface packet. Disabled text is excluded by that contract.
    fn luminance(c: Rgba) -> f64 {
        let channel = |v: f32| {
            let v = f64::from(v);
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
    }
    use tekes_ui_controls::paint::ButtonVariant;
    use tekes_ui_scene::Primitive;
    for theme in Theme::ALL {
        for brand in [0, 1] {
            for variant in [
                ButtonVariant::Quiet,
                ButtonVariant::Primary,
                ButtonVariant::Panel,
            ] {
                for (hovered, pressed) in [(false, false), (true, false), (true, true)] {
                    let mut button = ButtonContent::new("Apply");
                    button.variant = variant;
                    button.button.state.hovered = hovered;
                    button.button.state.pressed = pressed;
                    let mut host = ContentHost::new(Content::new(ComponentId(1), button)).unwrap();
                    let mut scene = Scene::new();
                    let mut text = Text::default();
                    let frame = Rect::new(10., 20., 120., 60.);
                    host.render(
                        &mut scene,
                        &mut text,
                        frame,
                        frame,
                        Palette::new(theme, brand).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(text.1.len(), 1);
                    let fill = scene
                        .primitives()
                        .iter()
                        .find_map(|p| match p {
                            Primitive::Quad(q) => Some(q.fill),
                            _ => None,
                        })
                        .expect("actual public Button background");
                    let ink = text.1[0];
                    assert_eq!(
                        fill.a, 1.0,
                        "translucent fill requires actual parent compositing"
                    );
                    assert_eq!(
                        ink.a, 1.0,
                        "translucent text requires actual fill compositing"
                    );
                    let (fg, bg) = (luminance(ink), luminance(fill));
                    let ratio = (fg.max(bg) + 0.05) / (fg.min(bg) + 0.05);
                    let minimum = if theme.is_increased() { 7.0 } else { 4.5 };
                    assert!(
                        ratio >= minimum,
                        "{theme:?}/{brand}/{variant:?}/hover={hovered}/press={pressed}: {ratio} < {minimum}"
                    );
                }
            }
        }
    }
}
