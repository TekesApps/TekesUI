//! Real public NumberField model/View shared by normal Content focus nodes.
use tekes_ui_composition_consumer::number_field::*;
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::ShapeOptions,
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text(CoreTextShaper);
impl ButtonTextRenderer for Text {
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        let style = role.style();
        let p = self.0.shape(
            text,
            &TextStyle::system(style.size).with_weight(style.weight),
            &ShapeOptions {
                width: Some(width as f64),
                line_spacing: 0.,
            },
        );
        Size::new(p.width as f32, p.height as f32)
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
fn render(host: &mut ContentHost) {
    let frame = Rect::new(8., 8., 344., 464.);
    host.render(
        &mut Scene::new(),
        &mut Text(CoreTextShaper::new(CoreTextFonts::new())),
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn key(host: &mut ContentHost, key: ControlKey, down: bool) {
    host.event(&ContentEvent::Key { key, down });
}
#[test]
fn current_model_enables_cached_stepper_via_the_actual_consumer_before_paint() {
    let mut host = build().unwrap();
    render(&mut host);
    assert!(set_value(&mut host, LIMIT, 2.));
    assert!(host.focus(Some(LIMIT_INCREMENT), true));
    // The View still contains its previous upper-limit paint state here.
    key(&mut host, ControlKey::Enter, true);
    assert_eq!(value(&host, LIMIT), 3.);
    key(&mut host, ControlKey::Space, true);
    host.event(&ContentEvent::Cancel);
    key(&mut host, ControlKey::Space, false);
    assert_eq!(value(&host, LIMIT), 3.);
    render(&mut host);
    assert!(
        host.accessibility_action(LIMIT_INCREMENT, None, AccessibilityAction::Press)
            .consumed
    );
    assert_eq!(value(&host, LIMIT), 4.);
    assert!(
        !host
            .accessibility_action(LIMIT_INCREMENT, None, AccessibilityAction::Press)
            .consumed
    );
    assert_eq!(value(&host, LIMIT), 4.);
}
#[test]
fn semantic_buttons_follow_actual_view_bounds_and_tab_skips_unavailable_children() {
    let mut host = build().unwrap();
    render(&mut host);
    assert_eq!(
        host.focus_targets()
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        vec![DECREMENT, INCREMENT, LIMIT_DECREMENT, AFTER]
    );
    for target in [DECREMENT, INCREMENT, LIMIT_DECREMENT, AFTER] {
        key(&mut host, ControlKey::Tab { reverse: false }, true);
        assert_eq!(host.focused(), Some(target));
    }
    key(&mut host, ControlKey::Tab { reverse: true }, true);
    assert_eq!(host.focused(), Some(LIMIT_DECREMENT));
    let p = host.get(INCREMENT).unwrap().frame().center();
    assert!(host.event(&ContentEvent::PointerDown(p)).capture_pointer);
    host.event(&ContentEvent::PointerUp(p));
    assert_eq!(value(&host, QUANTITY), 3.);
    assert_eq!(value(&host, DISABLED), 2.);
    let nodes = host.semantics();
    assert_eq!(
        nodes.iter().find(|n| n.id == INCREMENT).unwrap().parent,
        Some(QUANTITY)
    );
    assert!(
        nodes
            .iter()
            .find(|n| n.id == LIMIT_INCREMENT)
            .unwrap()
            .semantics
            .disabled
    );
}
