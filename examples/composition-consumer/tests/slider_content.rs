use tekes_ui_composition_consumer::slider_content::{self, ACTION, EDITOR, RANGE, ROOT};
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), r.style().line_height)
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
impl tekes_ui_controls::TextSurfaceHost for Text {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::model::Paragraph> {
        use tekes_ui_text::traits::Shaper;
        let mut shaper = tekes_ui_text::coretext::CoreTextShaper::new(
            tekes_ui_text::coretext::CoreTextFonts::new(),
        );
        std::rc::Rc::new(shaper.shape(
            text,
            &tekes_ui_text::style::TextStyle::system(role.style().size),
            &tekes_ui_text::model::ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl tekes_ui_controls::TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        _: &mut Scene,
        _: &tekes_ui_text::model::Paragraph,
        _: tekes_ui_scene::Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}
fn render(h: &mut ContentHost) {
    h.render(
        &mut Scene::new(),
        &mut Text,
        Rect::new(0., 0., 320., 320.),
        Rect::new(0., 0., 320., 320.),
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn value(h: &ContentHost) -> f64 {
    h.get(RANGE)
        .unwrap()
        .component::<SliderContent>()
        .unwrap()
        .model
        .value()
}
#[test]
fn original_range_adapter_retains_identity_values_and_semantics_through_composition() {
    let mut h = slider_content::build().unwrap();
    render(&mut h);
    assert_eq!(h.value_range(RANGE), Some(0.0..=100.0));
    assert_eq!(h.value_range(EDITOR), None);
    let node = h.semantics().into_iter().find(|n| n.id == RANGE).unwrap();
    assert_eq!(node.semantics.role, ContentRole::Slider);
    assert_eq!(node.semantics.name, "Volume");
    assert_eq!(node.semantics.value.as_deref(), Some("50"));
    h.focus(Some(RANGE), true);
    for (action, expected) in [
        (AccessibilityAction::Increment, 55.),
        (AccessibilityAction::Decrement, 50.),
        (AccessibilityAction::SetValue("63".into()), 65.),
        (AccessibilityAction::SetValue("999".into()), 100.),
    ] {
        assert!(h.accessibility_action(RANGE, None, action).consumed);
        assert_eq!(value(&h), expected);
    }
    assert!(
        h.accessibility_action(RANGE, None, AccessibilityAction::Increment)
            .consumed
    );
    assert_eq!(value(&h), 100.);
    for invalid in ["NaN", "inf", "no value"] {
        assert!(
            !h.accessibility_action(RANGE, None, AccessibilityAction::SetValue(invalid.into()))
                .consumed
        );
        assert_eq!(value(&h), 100.);
    }
    assert!(
        !h.accessibility_action(RANGE, Some(0), AccessibilityAction::Increment)
            .consumed
    );
    assert!(
        !h.accessibility_action(RANGE, None, AccessibilityAction::Press)
            .activated
    );
    h.reorder(ROOT, &[ACTION, RANGE, EDITOR, ComponentId(8504)])
        .unwrap();
    render(&mut h);
    assert_eq!(h.focused(), Some(RANGE));
    assert_eq!(value(&h), 100.);
    assert!(
        h.event(&ContentEvent::Key {
            key: ControlKey::Home,
            down: true
        })
        .changed
    );
    assert_eq!(value(&h), 0.);
    assert!(
        h.event(&ContentEvent::Key {
            key: ControlKey::Left,
            down: true
        })
        .consumed
    );
    assert_eq!(value(&h), 0.);
    let removed = h.remove(RANGE).unwrap();
    assert_eq!(h.value_range(RANGE), None);
    assert_eq!(h.focused(), None);
    h.insert(ROOT, removed).unwrap();
    render(&mut h);
    assert_eq!(value(&h), 0.);
    assert_eq!(h.value_range(RANGE), Some(0.0..=100.0));
}
#[test]
fn range_drag_is_isolated_and_disable_or_accessibility_update_cancels_old_capture() {
    let mut h = slider_content::build().unwrap();
    render(&mut h);
    let rect = h.get(RANGE).unwrap().frame();
    let p = rect.center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    h.event(&ContentEvent::PointerMove(tekes_ui_scene::Point::new(
        800., p.y,
    )));
    assert_eq!(value(&h), 100.);
    assert!(
        !h.event(&ContentEvent::PointerUp(tekes_ui_scene::Point::new(
            800., p.y
        )))
        .activated
    );
    h.event(&ContentEvent::PointerDown(p));
    h.accessibility_action(RANGE, None, AccessibilityAction::SetValue("25".into()));
    h.event(&ContentEvent::PointerMove(tekes_ui_scene::Point::new(
        800., p.y,
    )));
    assert_eq!(value(&h), 25.);
    h.event(&ContentEvent::PointerUp(p));
    h.event(&ContentEvent::PointerDown(p));
    h.get_mut(RANGE)
        .unwrap()
        .component_mut::<SliderContent>()
        .unwrap()
        .model
        .disabled = true;
    assert!(
        !h.accessibility_action(RANGE, None, AccessibilityAction::Increment)
            .consumed
    );
    render(&mut h);
    assert_eq!(h.focused(), None);
    h.get_mut(RANGE)
        .unwrap()
        .component_mut::<SliderContent>()
        .unwrap()
        .model
        .disabled = false;
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(value(&h), 50.);
    assert!(
        !h.get(RANGE)
            .unwrap()
            .component::<SliderContent>()
            .unwrap()
            .view
            .is_captured()
    );
}
#[test]
fn range_is_an_independent_interaction_and_cannot_be_nested_inside_an_action() {
    let range = Content::new(
        RANGE,
        SliderContent::new(
            "Volume",
            tekes_ui_controls::extended::Slider::new(50., 0., 100., 5.).unwrap(),
        ),
    );
    assert!(matches!(
        ContentHost::new(Content::new(
            ComponentId(8599),
            Button::new(range).with_accessible_name("Bad nesting")
        )),
        Err(CompositionError::NestedInteractive { .. })
    ));
}
#[test]
fn public_slider_content_keeps_original_tokens_and_independent_focus_geometry() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for disabled in [false, true] {
                for focused in [false, true] {
                    let mut h = slider_content::build().unwrap();
                    let palette = tekes_ui_tokens::Palette::new(theme, brand).unwrap();
                    h.get_mut(RANGE)
                        .unwrap()
                        .component_mut::<SliderContent>()
                        .unwrap()
                        .model
                        .disabled = disabled;
                    if focused {
                        h.focus(Some(RANGE), true);
                    }
                    let frame = Rect::new(0., 0., 320., 320.);
                    #[cfg(feature = "token-audit")]
                    let (scene, reads) = tekes_ui_tokens::audit::capture(|| {
                        let mut s = Scene::new();
                        h.render(&mut s, &mut Text, frame, frame, palette).unwrap();
                        s
                    });
                    #[cfg(not(feature = "token-audit"))]
                    let (scene, reads) = (
                        {
                            let mut s = Scene::new();
                            h.render(&mut s, &mut Text, frame, frame, palette).unwrap();
                            s
                        },
                        Vec::<(String, String)>::new(),
                    );
                    let slider = h.get(RANGE).unwrap().component::<SliderContent>().unwrap();
                    let l = slider.view.layout();
                    assert_eq!(slider.model.value(), 50.);
                    let thumb = scene
                        .primitives()
                        .iter()
                        .find_map(|p| {
                            if let tekes_ui_scene::Primitive::Quad(q) = p {
                                (q.rect == l.thumb).then_some(q)
                            } else {
                                None
                            }
                        })
                        .unwrap();
                    assert_eq!(
                        thumb.fill,
                        palette
                            .color(if disabled { "surface-2" } else { "surface-0" })
                            .unwrap()
                    );
                    assert_eq!(
                        thumb.border_color,
                        palette
                            .color(if disabled { "disabled-text" } else { "border" })
                            .unwrap()
                    );
                    assert_eq!(slider.view.is_focused(), focused && !disabled);
                    assert_eq!(h.value_range(RANGE), Some(0.0..=100.0));
                    cases.push(serde_json::json!({"component":"tui.slider","range_state":if disabled{"disabled"}else if focused{"focus"}else{"rest"},"state_asserted":true,"context":"public SliderContent with TextFieldContent sibling","theme":format!("{theme:?}"),"brand":brand,"lookups":reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 48);
    if let Some(path) = std::env::var_os("TEKESUI_SLIDER_COMPOSITION_AUDIT_OUT") {
        #[cfg(not(feature = "token-audit"))]
        {
            let _ = path;
            panic!("token-audit required");
        }
        #[cfg(feature="token-audit")]std::fs::write(path,serde_json::to_string_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"cases":cases})).unwrap()).unwrap();
    }
}
