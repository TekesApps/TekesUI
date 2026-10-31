use std::rc::Rc;
use tekes_ui_composition_consumer::selectable_list_content::*;
use tekes_ui_controls::{composition::*, extended::ListItemKey, layout_view::IconRenderer};
use tekes_ui_scene::{ImageId, Point, Rect, Scene};
use tekes_ui_tokens::{Rgba, Theme};
#[path = "support/coretext.rs"]
mod coretext;
struct Icons;
impl IconRenderer for Icons {
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        panic!("prepared icon required")
    }
    fn prepare_icon(
        &mut self,
        _: &str,
        _: f32,
        _: f32,
        _: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        Ok(IconImage::new(ImageId(1), Rc::new(())))
    }
}
fn render(h: &mut ContentHost, text: &mut coretext::Text, width: f32, clip: Rect, theme: Theme) {
    let f = Rect::new(0., 0., width, 700.);
    h.render(
        &mut Scene::new(),
        &mut WithIconRenderer::new(text, &mut Icons),
        f,
        clip,
        theme.into(),
    )
    .unwrap();
}
fn parts(h: &ContentHost) -> Vec<SemanticPart> {
    h.semantics()
        .into_iter()
        .find(|n| n.id == LIST)
        .unwrap()
        .parts
}
#[test]
fn real_composer_preedit_binding_selection_and_barriers_survive_background_list_updates() {
    let mut h = build().unwrap();
    h.set_action_events(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let f = Rect::new(0., 0., 500., 700.);
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    assert!(h.focus(Some(COMPOSER), true));
    h.event_with_text(
        &ContentEvent::Preedit {
            text: "ni".into(),
            selection: 1..2,
            clauses: vec![],
            replacement: None,
        },
        &mut text,
    )
    .unwrap();
    let binding = h.focused_text_binding();
    let before = h.semantic_text_input_snapshot(COMPOSER).unwrap();
    let mount = h.semantic_mount(ComponentId(202));
    let row = parts(&h)
        .into_iter()
        .find(|p| matches!(p.semantics.role, ContentRole::ListItem))
        .unwrap();
    for a in [AccessibilityAction::Focus, AccessibilityAction::Press] {
        assert!(!h.accessibility_action(LIST, Some(row.index), a).consumed);
    }
    h.event(&ContentEvent::Key {
        key: tekes_ui_controls::ControlKey::Tab { reverse: false },
        down: true,
    });
    assert_eq!(h.focused(), Some(COMPOSER));
    h.event(&ContentEvent::Key {
        key: tekes_ui_controls::ControlKey::Enter,
        down: true,
    });
    assert!(
        h.drain_action_records()
            .iter()
            .all(|r| r.action.kind != ContentActionKind::Submit)
    );
    assert!(h.is_composing());
    reorder(&mut h).unwrap();
    h.set_inherited_surface("canvas").unwrap();
    h.get_mut(LIST)
        .unwrap()
        .set_surface(Some("canvas"))
        .unwrap();
    render(
        &mut h,
        &mut text,
        90.,
        Rect::new(0., 20., 90., 180.),
        Theme::DARK,
    );
    h.get_mut(LIST).unwrap().set_surface(None).unwrap();
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    let after = h.semantic_text_input_snapshot(COMPOSER).unwrap();
    assert_eq!(
        (after.text, after.selection, after.marked),
        (before.text, before.selection, before.marked)
    );
    assert_eq!(h.focused_text_binding(), binding);
    assert_eq!(h.semantic_mount(ComponentId(202)), mount);
    assert!(h.is_composing());
    assert!(h.drain_action_records().is_empty());
    let row = parts(&h)
        .into_iter()
        .find(|p| matches!(p.semantics.role, ContentRole::ListItem))
        .unwrap();
    let point = Point::new(row.frame.min_x() + 5., row.frame.min_y() + 5.);
    h.event_with_text(&ContentEvent::PointerDown(point), &mut text)
        .unwrap();
    assert_eq!(h.focused(), Some(LIST));
    assert!(!h.is_composing());
    assert!(
        h.drain_action_records()
            .iter()
            .all(|r| r.action.kind != ContentActionKind::Submit)
    );
}
#[test]
fn real_composer_preedit_survives_list_reader_unmount_without_submit_or_binding_transfer() {
    let mut h = build().unwrap();
    h.set_action_events(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let f = Rect::new(0., 0., 500., 700.);
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    assert!(h.focus(Some(COMPOSER), true));
    h.event_with_text(
        &ContentEvent::Preedit {
            text: "zhong".into(),
            selection: 2..4,
            clauses: vec![],
            replacement: None,
        },
        &mut text,
    )
    .unwrap();
    let before = h.semantic_text_input_snapshot(COMPOSER).unwrap();
    let binding = h.focused_text_binding();
    let removed = h.remove(READER).unwrap();
    render(&mut h, &mut text, 500., f, Theme::DARK);
    let after = h.semantic_text_input_snapshot(COMPOSER).unwrap();
    assert_eq!(
        (before.text, before.selection, before.marked),
        (after.text, after.selection, after.marked)
    );
    assert_eq!(binding, h.focused_text_binding());
    assert_eq!(h.focused(), Some(COMPOSER));
    assert!(h.is_composing());
    assert!(h.drain_action_records().is_empty());
    drop(removed);
}
#[test]
fn real_text_icon_keep_mount_theme_override_clear_and_default_display_share_arranged_geometry() {
    let mut h = build().unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let f = Rect::new(0., 0., 500., 700.);
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    let mount = h.semantic_mount(ComponentId(202));
    let frame = h.get(ComponentId(101)).unwrap().frame();
    for theme in [Theme::LIGHT, Theme::DARK] {
        h.set_inherited_surface("panel").unwrap();
        h.get_mut(LIST)
            .unwrap()
            .set_surface(Some("canvas"))
            .unwrap();
        render(&mut h, &mut text, 500., f, theme);
        assert_eq!(h.get(ComponentId(101)).unwrap().frame(), frame);
        h.get_mut(LIST).unwrap().set_surface(None).unwrap();
        render(&mut h, &mut text, 500., f, theme);
        assert_eq!(h.get(ComponentId(101)).unwrap().frame(), frame);
        assert_eq!(h.semantic_mount(ComponentId(202)), mount);
    }
    let s = snapshot(true);
    replace(&mut h, &s, &mut rows(&s)).unwrap();
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    assert_eq!(h.semantic_mount(ComponentId(202)), mount);
    let s = snapshot(false);
    let mut d = rows(&s);
    d[0].display = ListDisplayUpdate::DefaultLabel;
    replace(&mut h, &s, &mut d).unwrap();
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    assert!(h.get(ComponentId(202)).is_none());
    assert_eq!(parts(&h)[0].frame, h.get(ComponentId(101)).unwrap().frame());
}
#[test]
fn shared_platform_adapter_uses_same_visible_positive_list_parts_and_real_focus_press_owner() {
    use tekes_ui_platform_macos::{
        AccessibilityAction as A, AccessibilityRole as R, content_accessibility as bridge,
    };
    let mut h = build().unwrap();
    h.set_action_events(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let f = Rect::new(0., 0., 500., 700.);
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    let nodes = bridge::content_nodes(&h);
    let _snapshot = bridge::ContentAccessibilitySnapshot::from_host(&h)
        .expect("whole accessibility snapshot accepted");
    assert_eq!(
        tekes_ui_platform_macos::accept_accessibility_nodes(nodes.clone()),
        nodes
    );
    let row = nodes
        .iter()
        .find(|n| matches!(n.role, R::ListItem))
        .unwrap()
        .key;
    assert!(bridge::content_action(&mut h, row, A::Focus).consumed);
    assert_eq!(
        h.get(LIST)
            .unwrap()
            .component::<KeyedSelectableListContent>()
            .unwrap()
            .selected_key(),
        None
    );
    assert!(
        bridge::content_nodes(&h)
            .iter()
            .find(|n| n.key == row)
            .unwrap()
            .focused
    );
    assert!(bridge::content_action(&mut h, row, A::Press).consumed);
    assert_eq!(
        h.get(LIST)
            .unwrap()
            .component::<KeyedSelectableListContent>()
            .unwrap()
            .selected_key(),
        Some(ListItemKey(1))
    );
    assert_eq!(h.drain_action_records().len(), 1);
    h.set_window_active(false);
    h.set_window_active(true);
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    let new = bridge::content_nodes(&h)
        .into_iter()
        .find(|n| matches!(n.role, R::ListItem))
        .unwrap()
        .key;
    assert_eq!(new.component, row.component);
    assert!(new.part.unwrap() > row.part.unwrap());
    assert!(!bridge::content_action(&mut h, row, A::Press).consumed);
    assert!(bridge::content_action(&mut h, new, A::Press).consumed);
}
