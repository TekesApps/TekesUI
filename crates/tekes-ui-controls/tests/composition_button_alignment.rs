use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::{Button, Component, ComponentId, Content, ContentHost, TextContent},
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
    use tekes_ui_controls::composition::ContentEvent;
    for theme in Theme::ALL {
        for brand in 0..3 {
            for width in [120., 400.] {
                for state in [
                    "rest",
                    "hovered",
                    "pressed",
                    "focus_visible",
                    "disabled",
                    "loading",
                ] {
                    let button = match state {
                        "disabled" => Button::new("Apply").with_disabled(true),
                        "loading" => Button::new("Apply").with_loading(true),
                        _ => Button::new("Apply"),
                    };
                    let mut host = ContentHost::new(Content::new(ComponentId(1), button)).unwrap();
                    let mut text = Text::default();
                    let frame = Rect::new(10., 20., width, 60.);
                    let palette = Palette::new(theme, brand).expect("registered Gallery brand");
                    host.render(&mut Scene::new(), &mut text, frame, frame, palette)
                        .unwrap();
                    match state {
                        "hovered" => {
                            host.event(&ContentEvent::PointerMove(frame.center()));
                        }
                        "pressed" => {
                            host.event(&ContentEvent::PointerMove(frame.center()));
                            host.event(&ContentEvent::PointerDown(frame.center()));
                        }
                        "focus_visible" => {
                            assert!(host.focus(Some(ComponentId(1)), true));
                        }
                        _ => {}
                    }
                    if matches!(state, "hovered" | "pressed" | "focus_visible") {
                        text = Text::default();
                        host.render(&mut Scene::new(), &mut text, frame, frame, palette)
                            .unwrap();
                    }
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
                            .component::<Button>()
                            .unwrap()
                            .activations(),
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
    let mut body = TextContent::new("Description");
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
fn search_clear_chrome_returns_to_rest_after_leave_or_cancellation() {
    use tekes_ui_controls::composition::{ContentEvent, SearchFieldContent};
    use tekes_ui_scene::{Point, Primitive};
    let frame = Rect::new(20., 30., 360., 40.);
    for theme in [Theme::LIGHT, Theme::DARK] {
        for exit in ["editor", "outside", "cancel", "blur"] {
            let mut search = SearchFieldContent::new("Find", "query");
            search.arrange(&mut Text::default(), frame, frame);
            let clear = search.semantic_parts()[0].frame.center();
            let paint = |search: &mut SearchFieldContent| {
                let mut scene = Scene::new();
                search.paint(
                    &mut scene,
                    &mut Text::default(),
                    frame,
                    frame,
                    Palette::from(theme).into(),
                );
                scene
                    .primitives()
                    .iter()
                    .rev()
                    .find_map(|p| match p {
                        Primitive::Quad(q) if q.rect.contains(clear) => Some(q.fill),
                        _ => None,
                    })
                    .expect("original Clear button background")
            };
            let rest = paint(&mut search);
            search.event(&ContentEvent::PointerMove(clear));
            assert_ne!(paint(&mut search), rest, "fixture must show Clear hover");
            match exit {
                "editor" => {
                    search.event(&ContentEvent::PointerMove(Point::new(40., 50.)));
                }
                "outside" => {
                    search.event(&ContentEvent::PointerMove(Point::new(-1., -1.)));
                }
                "cancel" => assert!(search.event(&ContentEvent::Cancel).changed),
                "blur" => search.set_focus(false, false),
                _ => unreachable!(),
            }
            assert_eq!(paint(&mut search), rest, "{theme:?}: {exit}");
            assert_eq!(search.query(), "query");
        }
    }
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
    use tekes_ui_controls::ButtonVariant;
    use tekes_ui_scene::Primitive;
    for theme in Theme::ALL {
        for brand in [0, 1] {
            for variant in [
                ButtonVariant::Quiet,
                ButtonVariant::Primary,
                ButtonVariant::Panel,
            ] {
                for (hovered, pressed) in [(false, false), (true, false), (true, true)] {
                    use tekes_ui_controls::composition::ContentEvent;
                    let button = Button::new("Apply").with_variant(variant);
                    let mut host = ContentHost::new(Content::new(ComponentId(1), button)).unwrap();
                    let mut scene = Scene::new();
                    let mut text = Text::default();
                    let frame = Rect::new(10., 20., 120., 60.);
                    let palette = Palette::new(theme, brand).unwrap();
                    host.render(&mut scene, &mut text, frame, frame, palette)
                        .unwrap();
                    if hovered {
                        host.event(&ContentEvent::PointerMove(frame.center()));
                    }
                    if pressed {
                        host.event(&ContentEvent::PointerDown(frame.center()));
                    }
                    if hovered || pressed {
                        scene = Scene::new();
                        text = Text::default();
                        host.render(&mut scene, &mut text, frame, frame, palette)
                            .unwrap();
                    }
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

#[cfg(feature = "inspection")]
#[test]
fn search_inspection_preserves_search_identity_and_original_editor_properties() {
    use tekes_ui_controls::composition::{SearchFieldContent, TextFieldContent};
    let search = SearchFieldContent::new("Find", "query");
    let field = TextFieldContent::new("Draft", "value");
    let search_properties = search.inspection_properties().unwrap();
    let field_properties = field.inspection_properties().unwrap();
    let value = |properties: &[tekes_ui_controls::composition::InspectionProperty], name| {
        properties
            .iter()
            .find(|property| property.name == name)
            .unwrap()
            .value
            .clone()
    };
    assert_eq!(value(&search_properties, "component"), "SearchFieldContent");
    assert_eq!(value(&field_properties, "component"), "TextFieldContent");
    assert_eq!(value(&search_properties, "label"), "Find");
    assert_eq!(value(&search_properties, "text"), "query");
}
