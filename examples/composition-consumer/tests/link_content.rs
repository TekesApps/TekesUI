use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(w), r.style().line_height)
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
fn host() -> ContentHost {
    let mut h = ContentHost::new(Content::new(
        ComponentId(1),
        LinkContent::new("Read documentation"),
    ))
    .unwrap();
    h.set_action_events(true);
    h
}
fn paint(h: &mut ContentHost) {
    let f = Rect::new(10., 10., 200., 44.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
}
fn key(h: &mut ContentHost, k: ControlKey, down: bool) {
    h.event(&ContentEvent::Key { key: k, down });
}
#[test]
fn one_public_link_owns_keyboard_pointer_and_ax_actions() {
    let mut h = host();
    paint(&mut h);
    assert!(h.focus(Some(ComponentId(1)), true));
    key(&mut h, ControlKey::Space, true);
    key(&mut h, ControlKey::Space, false);
    assert!(h.drain_actions().is_empty());
    key(&mut h, ControlKey::Enter, true);
    key(&mut h, ControlKey::Enter, false);
    assert_eq!(h.drain_actions().len(), 1);
    let p = h.get(ComponentId(1)).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(p));
    assert!(h.pointer_capture().is_some());
    h.event(&ContentEvent::PointerUp(p));
    assert_eq!(h.drain_actions().len(), 1);
    h.accessibility_action(ComponentId(1), None, AccessibilityAction::Press);
    assert_eq!(h.drain_actions().len(), 1);
    assert_eq!(
        h.semantics()
            .iter()
            .filter(|n| n.semantics.role == ContentRole::Link)
            .count(),
        1
    );
}
#[test]
fn loading_and_disabled_revoke_held_action_without_requiring_a_paint() {
    for loading in [false, true] {
        let mut h = host();
        paint(&mut h);
        let p = h.get(ComponentId(1)).unwrap().frame().center();
        h.event(&ContentEvent::PointerDown(p));
        let c = h
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<LinkContent>()
            .unwrap();
        c.model.button.state.disabled = !loading;
        c.model.button.state.loading = loading;
        h.event(&ContentEvent::PointerUp(p));
        h.accessibility_action(ComponentId(1), None, AccessibilityAction::Press);
        assert!(h.drain_actions().is_empty());
        assert!(h.pointer_capture().is_none());
    }
}
#[test]
fn cancel_and_window_loss_reject_old_release_and_ax_then_allow_new_input() {
    let mut h = host();
    paint(&mut h);
    let p = h.get(ComponentId(1)).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(p));
    h.cancel();
    h.event(&ContentEvent::PointerUp(p));
    assert!(h.drain_actions().is_empty());
    h.event(&ContentEvent::PointerDown(p));
    h.set_window_active(false);
    h.event(&ContentEvent::PointerUp(p));
    h.accessibility_action(ComponentId(1), None, AccessibilityAction::Press);
    assert!(h.drain_actions().is_empty());
    h.set_window_active(true);
    paint(&mut h);
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::PointerUp(p));
    assert_eq!(h.drain_actions().len(), 1);
}
#[test]
fn accessible_name_is_required() {
    assert!(ContentHost::new(Content::new(ComponentId(1), LinkContent::new("  "))).is_err());
}

#[path = "support/coretext.rs"]
mod coretext;
#[test]
fn real_declared_auxiliary_link_preserves_composer_marked_text_and_focus() {
    use tekes_ui_composition_consumer::link_content::*;
    let mut h = build().unwrap();
    let f = Rect::new(0., 0., 420., 240.);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    h.render(&mut Scene::new(), &mut text, f, f, Theme::LIGHT.into())
        .unwrap();
    h.focus(Some(COMPOSER), true);
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        replacement: None,
        clauses: vec![],
    });
    assert!(h.focused_text().unwrap().input.marked.is_some());
    let p = h.get(LINK).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::PointerUp(p));
    assert_eq!(process_actions(&mut h).len(), 1);
    assert_eq!(h.focused(), Some(COMPOSER));
    assert!(h.focused_text().unwrap().input.marked.is_some());
    assert!(
        !h.get(LINK)
            .unwrap()
            .component::<LinkContent>()
            .unwrap()
            .model
            .button
            .state
            .focused
    );
    h.accessibility_action(LINK, None, AccessibilityAction::Press);
    assert_eq!(process_actions(&mut h).len(), 1);
    assert_eq!(h.focused(), Some(COMPOSER));
    assert!(h.focused_text().unwrap().input.marked.is_some());
    assert!(
        !h.get(LINK)
            .unwrap()
            .component::<LinkContent>()
            .unwrap()
            .model
            .button
            .state
            .focused
    );
    h.accessibility_action(LINK, None, AccessibilityAction::Focus);
    assert_eq!(h.focused(), Some(COMPOSER));
    h.event(&ContentEvent::Unmark);
    h.accessibility_action(LINK, None, AccessibilityAction::Focus);
    assert_eq!(h.focused(), Some(LINK));
}

#[test]
fn removing_and_remounting_original_link_cannot_replay_a_held_gesture_or_queued_action() {
    use tekes_ui_composition_consumer::link_content::*;
    let mut h = build().unwrap();
    let f = Rect::new(0., 0., 420., 240.);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    h.render(&mut Scene::new(), &mut text, f, f, Theme::LIGHT.into())
        .unwrap();
    let p = h.get(LINK).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(p));
    let retained = h.remove(LINK).unwrap();
    h.event(&ContentEvent::PointerUp(p));
    h.accessibility_action(LINK, None, AccessibilityAction::Press);
    assert!(process_actions(&mut h).is_empty());
    h.insert(ROOT, retained).unwrap();
    h.render(&mut Scene::new(), &mut text, f, f, Theme::LIGHT.into())
        .unwrap();
    h.event(&ContentEvent::PointerUp(p));
    assert!(process_actions(&mut h).is_empty());
    let p = h.get(LINK).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::PointerUp(p));
    assert_eq!(process_actions(&mut h).len(), 1);
}

#[test]
fn natural_link_size_keeps_underline_below_the_shaped_text_line_at_each_type_scale() {
    use tekes_ui_composition_consumer::link_content::*;
    for scale in [1., 1.5, 2., 3.] {
        let mut h = build().unwrap();
        let mut text = coretext::Text::new(scale);
        text.services = true;
        let f = Rect::new(0., 0., 560., 440.);
        let mut scene = Scene::new();
        h.render(&mut scene, &mut text, f, f, Theme::LIGHT.into())
            .unwrap();
        let frame = h.get(LINK).unwrap().frame();
        let label = &h
            .get(LINK)
            .unwrap()
            .component::<LinkContent>()
            .unwrap()
            .label;
        let inset = proposal::metric("spacing", "space-regular").unwrap();
        let shaped = text.measure(
            label,
            proposal::TextRole::UiBody,
            (frame.size.width - 2. * inset).max(0.),
        );
        let ink = tekes_ui_tokens::surface_palette::resolve_surface(
            Theme::LIGHT.into(),
            "canvas",
            "link",
            "rest",
        )
        .unwrap()
        .text;
        let underline = scene
            .primitives()
            .iter()
            .rev()
            .find_map(|p| match p {
                tekes_ui_scene::Primitive::Quad(q)
                    if q.fill == ink
                        && q.rect.min_y() >= frame.min_y()
                        && q.rect.max_y() <= frame.max_y() =>
                {
                    Some(q.rect)
                }
                _ => None,
            })
            .unwrap();
        assert!(
            underline.min_y() >= frame.center().y + shaped.height * 0.5,
            "scale {scale}: underline {underline:?} intersects shaped line height {} in {frame:?}",
            shaped.height
        );
    }
}

#[test]
fn window_margin_preserves_complete_link_focus_only_with_the_actual_window_clip() {
    use tekes_ui_composition_consumer::link_content::*;
    let viewport = Rect::new(0., 0., 720., 640.);
    let allocation = Rect::new(16., 16., 688., 608.);
    let focus = tekes_ui_tokens::surface_palette::resolve_surface(
        Theme::LIGHT.into(),
        "canvas",
        "link",
        "rest",
    )
    .unwrap()
    .focus
    .unwrap();
    for (clip, expected_clipped) in [(allocation, true), (viewport, false)] {
        let mut h = build().unwrap();
        let mut text = coretext::Text::new(1.);
        text.services = true;
        h.render(
            &mut Scene::new(),
            &mut text,
            allocation,
            clip,
            Theme::LIGHT.into(),
        )
        .unwrap();
        h.focus(Some(LINK), true);
        let mut scene = Scene::new();
        h.render(&mut scene, &mut text, allocation, clip, Theme::LIGHT.into())
            .unwrap();
        let mut clips = vec![viewport];
        let mut observed = Vec::new();
        for primitive in scene.primitives() {
            match primitive {
                tekes_ui_scene::Primitive::PushClip(rect) => {
                    clips.push(clips.last().unwrap().intersection(rect).unwrap_or_default());
                }
                tekes_ui_scene::Primitive::PopClip => {
                    clips.pop();
                }
                tekes_ui_scene::Primitive::Quad(q)
                    if q.border_color == focus && q.border_width > 0. =>
                {
                    observed.push(q.rect.intersection(clips.last().unwrap()) != Some(q.rect));
                }
                _ => {}
            }
        }
        assert_eq!(
            observed,
            vec![expected_clipped],
            "clip must describe the actual window, not the inset layout allocation"
        );
    }
}

#[test]
fn wrapped_label_line_box_is_contained_by_its_own_rounded_ground() {
    use tekes_ui_composition_consumer::link_content::*;
    for scale in [1., 1.5, 2., 3.] {
        for width in [240., 420.] {
            let mut h = build().unwrap();
            h.get_mut(LINK)
                .unwrap()
                .component_mut::<LinkContent>()
                .unwrap()
                .label =
                "gyp glyph JPEG typography gyp glyph JPEG typography gyp glyph JPEG typography"
                    .into();
            let mut text = coretext::Text::new(scale);
            text.services = true;
            let viewport = Rect::new(0., 0., width, 560.);
            let allocation = Rect::new(16., 16., width - 32., 528.);
            let mut scene = Scene::new();
            h.render(
                &mut scene,
                &mut text,
                allocation,
                viewport,
                Theme::LIGHT.into(),
            )
            .unwrap();
            let link = h.get(LINK).unwrap().component::<LinkContent>().unwrap();
            let frame = h.get(LINK).unwrap().frame();
            assert_eq!(link.view.layout().frame, frame);
            let slot = link.view.layout().content;
            let size = text.measure(&link.label, proposal::TextRole::UiBody, slot.size.width);
            let bounds = Rect::new(
                slot.min_x(),
                slot.center().y - size.height * 0.5,
                size.width,
                size.height,
            );
            let ground = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    tekes_ui_scene::Primitive::Quad(q) if q.rect == frame => Some(q),
                    _ => None,
                })
                .unwrap();
            let radius = ground.corner_radii.top_left;
            for x in [bounds.min_x(), bounds.max_x()] {
                for y in [bounds.min_y(), bounds.max_y()] {
                    let dx = (radius - (x - frame.min_x()).min(frame.max_x() - x)).max(0.);
                    let dy = (radius - (y - frame.min_y()).min(frame.max_y() - y)).max(0.);
                    assert!(
                        dx * dx + dy * dy <= radius * radius + 0.001,
                        "scale {scale}, width {width}: line-box corner ({x}, {y}) lies outside rounded ground"
                    );
                }
            }
        }
    }
}
