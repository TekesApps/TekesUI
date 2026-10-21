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

use tekes_ui_composition_consumer::compact_actions::*;
use tekes_ui_controls::{ButtonFramePolicy, ButtonLayout, ButtonSize, ControlKey};
use tekes_ui_scene::Point;
fn paint(h: &mut ContentHost) {
    let f = Rect::new(20., 30., 200., 140.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
}
fn act(h: &ContentHost, id: ComponentId) -> &CompoundButton {
    h.get(id).unwrap().component::<CompoundButton>().unwrap()
}
fn event(h: &mut ContentHost, e: ContentEvent) -> Vec<ContentAction> {
    h.event(&e);
    h.drain_actions()
}
fn p(h: &ContentHost, id: ComponentId) -> Point {
    h.get(id).unwrap().frame().center()
}
#[test]
fn exact_frame_hit_ax_and_decoration_are_one_nonoverlapping_action() {
    let mut h = build();
    paint(&mut h);
    let nodes = h.semantics();
    for id in [RELEASE, PRESS, DISABLED] {
        let f = h.get(id).unwrap().frame();
        let b = act(&h, id).action.button.layout().unwrap();
        assert_eq!(f, b.frame);
        assert_eq!(f.size.height, 10.);
        assert_eq!(nodes.iter().find(|n| n.id == id).unwrap().frame, f);
        assert!(!nodes.iter().any(|n| n.id == ComponentId(id.0 + 100)));
    }
    let edge = Point::new(21., 40.);
    assert!(
        !act(&h, RELEASE)
            .action
            .button
            .layout()
            .unwrap()
            .hit_test(edge)
    );
    assert!(
        act(&h, PRESS)
            .action
            .button
            .layout()
            .unwrap()
            .hit_test(edge)
    );
    let a = event(&mut h, ContentEvent::PointerDown(edge));
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].target, PRESS);
    assert_eq!(a[0].source, ContentActionSource::Pointer);
    assert!(event(&mut h, ContentEvent::PointerUp(edge)).is_empty());
}
#[test]
fn standard_layout_and_constructor_defaults_keep_token_minimum() {
    let f = Rect::new(0., 0., 100., 10.);
    let b = ButtonLayout::resolve(
        f,
        None,
        Size::new(10., 18.),
        ButtonSize::Body,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Center,
    );
    assert!(b.frame.size.height > 10.);
    assert_eq!(ButtonFramePolicy::default(), ButtonFramePolicy::Minimum);
    assert_eq!(
        ButtonContent::new("default").pointer_activation_phase,
        ButtonActivationPhase::Release
    );
}
#[test]
fn release_press_repeat_dragout_and_cancel_use_original_arm() {
    let mut h = build();
    paint(&mut h);
    for id in [RELEASE, PRESS] {
        let point = p(&h, id);
        let down = event(&mut h, ContentEvent::PointerDown(point));
        assert_eq!(down.len(), usize::from(id == PRESS));
        assert!(event(&mut h, ContentEvent::PointerDown(point)).is_empty());
        event(&mut h, ContentEvent::PointerMove(Point::new(300., 300.)));
        assert!(event(&mut h, ContentEvent::PointerUp(Point::new(300., 300.))).is_empty());
        let point = p(&h, id);
        event(&mut h, ContentEvent::PointerDown(point));
        event(&mut h, ContentEvent::Cancel);
        assert!(event(&mut h, ContentEvent::PointerUp(point)).is_empty());
        event(&mut h, ContentEvent::PointerDown(point));
        let up = event(&mut h, ContentEvent::PointerUp(point));
        assert_eq!(up.len(), usize::from(id == RELEASE));
    }
}
#[test]
fn keyboard_and_ax_preserve_once_only_source_and_visible_focus() {
    let mut h = build();
    paint(&mut h);
    for id in [RELEASE, PRESS] {
        h.focus(Some(id), true);
        paint(&mut h);
        assert!(act(&h, id).action.button.state.focus_visible);
        for key in [ControlKey::Space, ControlKey::Enter] {
            let mut actions = event(&mut h, ContentEvent::Key { key, down: true });
            actions.extend(event(&mut h, ContentEvent::Key { key, down: false }));
            assert_eq!(actions.len(), 1);
            assert_eq!(actions[0].target, id);
            assert_eq!(actions[0].source, ContentActionSource::Keyboard);
        }
        h.accessibility_action(id, None, AccessibilityAction::Press);
        let a = h.drain_actions();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].target, id);
        assert_eq!(a[0].source, ContentActionSource::Accessibility);
    }
}
#[test]
fn disabled_loading_unmount_and_pending_mount_identity_are_fenced() {
    let mut h = build();
    paint(&mut h);
    let point = p(&h, DISABLED);
    assert!(event(&mut h, ContentEvent::PointerDown(point)).is_empty());
    assert!(event(&mut h, ContentEvent::PointerUp(point)).is_empty());
    h.accessibility_action(DISABLED, None, AccessibilityAction::Press);
    assert!(h.drain_actions().is_empty());
    let point = p(&h, PRESS);
    h.event(&ContentEvent::PointerDown(point));
    let retained = h.remove(PRESS).unwrap();
    h.insert(ROOT, retained).unwrap();
    assert!(h.drain_actions().is_empty());
    paint(&mut h);
    assert!(event(&mut h, ContentEvent::PointerUp(point)).is_empty());
    h.get_mut(PRESS)
        .unwrap()
        .component_mut::<CompoundButton>()
        .unwrap()
        .action
        .button
        .state
        .loading = true;
    paint(&mut h);
    assert!(event(&mut h, ContentEvent::PointerDown(point)).is_empty());
    h.accessibility_action(PRESS, None, AccessibilityAction::Press);
    assert!(h.drain_actions().is_empty());
}
#[test]
fn pointer_cancel_and_window_deactivation_clear_committed_press_without_replay() {
    let mut h = build();
    paint(&mut h);
    let point = p(&h, PRESS);
    assert_eq!(event(&mut h, ContentEvent::PointerDown(point)).len(), 1);
    h.cancel();
    assert!(event(&mut h, ContentEvent::PointerUp(point)).is_empty());
    paint(&mut h);
    assert_eq!(event(&mut h, ContentEvent::PointerDown(point)).len(), 1);
    assert!(event(&mut h, ContentEvent::PointerUp(point)).is_empty());
}
#[test]
fn exact_keyboard_focus_ring_is_not_overpainted_by_adjacent_actions() {
    let mut h = build();
    paint(&mut h);
    h.focus(Some(RELEASE), true);
    let f = Rect::new(20., 30., 200., 140.);
    let mut scene = Scene::new();
    h.render(&mut scene, &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
    let frame = h.get(RELEASE).unwrap().frame();
    let next = h.get(PRESS).unwrap().frame();
    let bleed = proposal::metric("stroke", "c-focus-width").unwrap()
        + proposal::metric("spacing", "c-focus-offset").unwrap();
    let quads = scene
        .primitives()
        .iter()
        .filter_map(|p| {
            if let tekes_ui_scene::Primitive::Quad(q) = p {
                Some(q)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let ring = quads
        .iter()
        .position(|q| q.rect == frame.inset(-bleed) && q.fill.a == 0. && q.border_width > 0.)
        .unwrap();
    let neighbour = quads.iter().position(|q| q.rect == next).unwrap();
    assert!(
        ring > neighbour,
        "adjacent action chrome must not cover original focus ring"
    );
}
