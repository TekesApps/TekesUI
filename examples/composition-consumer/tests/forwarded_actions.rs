use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
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

use tekes_ui_composition_consumer::forwarded_actions::*;
use tekes_ui_controls::{ButtonFramePolicy, ControlKey};
use tekes_ui_scene::Point;
fn setup() -> ContentHost {
    let mut h = build();
    let f = Rect::new(20., 30., 240., 160.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
    h
}
fn event(h: &mut ContentHost, e: ContentEvent) -> Vec<ContentAction> {
    h.event(&e);
    h.drain_actions()
}
#[test]
fn public_paint_declaration_forwards_original_button_without_state_predicates() {
    let mut h = setup();
    for id in [DIRECT, WRAPPED_RELEASE, WRAPPED_PRESS] {
        assert!(!action(&h, id).focus_decoration_above_siblings());
        h.focus(Some(id), true);
        assert!(action(&h, id).focus_decoration_above_siblings());
        if let Some(wrapper) = h.get(id).unwrap().component::<ForwardedAction>() {
            assert!(wrapper.focus_decoration_above_siblings());
        }
        for state in 0..3 {
            let b = &mut action_mut(&mut h, id).button;
            b.state.disabled = state == 0;
            b.state.loading = state == 1;
            b.frame_policy = if state == 2 {
                ButtonFramePolicy::Minimum
            } else {
                ButtonFramePolicy::Exact
            };
            assert!(!action(&h, id).focus_decoration_above_siblings());
        }
        let b = &mut action_mut(&mut h, id).button;
        b.state.disabled = false;
        b.state.loading = false;
        b.frame_policy = ButtonFramePolicy::Exact;
        h.focus(Some(id), false);
        assert!(!action(&h, id).focus_decoration_above_siblings());
    }
    let normal = h.get(NORMAL).unwrap().component::<Button>().unwrap();
    assert_eq!(normal.button.frame_policy, ButtonFramePolicy::Minimum);
}
#[test]
fn real_pointer_release_and_press_keep_original_once_only_capture() {
    for (id, on_down) in [(WRAPPED_RELEASE, false), (WRAPPED_PRESS, true)] {
        let mut h = setup();
        let p = h.get(id).unwrap().frame().center();
        let down = event(&mut h, ContentEvent::PointerDown(p));
        assert_eq!(down.len(), usize::from(on_down));
        let up = event(&mut h, ContentEvent::PointerUp(p));
        assert_eq!(up.len(), usize::from(!on_down));
        let a = down.into_iter().chain(up).collect::<Vec<_>>();
        assert_eq!(a[0].target, id);
        assert_eq!(a[0].source, ContentActionSource::Pointer);
        assert_eq!(
            h.get(id)
                .unwrap()
                .component::<ForwardedAction>()
                .unwrap()
                .captures,
            1
        );
        assert!(event(&mut h, ContentEvent::PointerUp(p)).is_empty());
    }
}
#[test]
fn half_open_rows_and_original_geometry_are_unchanged() {
    let mut h = setup();
    let f = h.get(WRAPPED_RELEASE).unwrap().frame();
    let next = h.get(WRAPPED_PRESS).unwrap().frame();
    assert_eq!(f.max_y(), next.min_y());
    let p = Point::new(f.center().x, f.max_y());
    assert!(
        !action(&h, WRAPPED_RELEASE)

            .button
            .layout()
            .unwrap()
            .hit_test(p)
    );
    assert!(
        action(&h, WRAPPED_PRESS)

            .button
            .layout()
            .unwrap()
            .hit_test(p)
    );
    let a = event(&mut h, ContentEvent::PointerDown(p));
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].target, WRAPPED_PRESS);
    assert!(event(&mut h, ContentEvent::PointerUp(p)).is_empty());
}
#[test]
fn original_cancel_disabled_loading_and_unmount_keep_input_ownership() {
    for id in [WRAPPED_RELEASE, WRAPPED_PRESS] {
        for mode in 0..4 {
            let mut h = setup();
            let p = h.get(id).unwrap().frame().center();
            // Keyboard activation always remains release-based, independent of pointer phase.
            h.focus(Some(id), true);
            assert!(
                event(
                    &mut h,
                    ContentEvent::Key {
                        key: ControlKey::Space,
                        down: true
                    }
                )
                .is_empty()
            );
            match mode {
                0 => {
                    event(&mut h, ContentEvent::Cancel);
                }
                1 => action_mut(&mut h, id).button.state.disabled = true,
                2 => action_mut(&mut h, id).button.state.loading = true,
                _ => {
                    h.remove(id).unwrap();
                }
            }
            assert!(
                event(
                    &mut h,
                    ContentEvent::Key {
                        key: ControlKey::Space,
                        down: false
                    }
                )
                .is_empty()
            );
            assert!(event(&mut h, ContentEvent::PointerUp(p)).is_empty());
        }
    }
}
#[test]
fn keyboard_ax_names_and_semantic_order_are_original_single_actions() {
    let mut h = setup();
    let nodes = h.semantics();
    let actions = nodes
        .iter()
        .filter(|n| n.semantics.role == ContentRole::Button)
        .map(|n| n.id)
        .collect::<Vec<_>>();
    assert_eq!(
        actions,
        vec![DIRECT, WRAPPED_RELEASE, WRAPPED_PRESS, AFTER, NORMAL]
    );
    for (id, name) in [
        (WRAPPED_RELEASE, "Wrapped release row"),
        (WRAPPED_PRESS, "Wrapped press row"),
    ] {
        let node = nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!(node.semantics.name, name);
        h.focus(Some(id), true);
        h.event(&ContentEvent::Key {
            key: ControlKey::Space,
            down: true,
        });
        h.event(&ContentEvent::Key {
            key: ControlKey::Space,
            down: false,
        });
        let a = h.drain_actions();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].target, id);
        assert_eq!(a[0].source, ContentActionSource::Keyboard);
        h.accessibility_action(id, None, AccessibilityAction::Press);
        let a = h.drain_actions();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].source, ContentActionSource::Accessibility);
    }
}
