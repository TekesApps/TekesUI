use std::rc::Rc;
use tekes_ui_composition_consumer::datatable_content::*;
use tekes_ui_controls::{composition::*, extended::TableRowKey, layout_view::IconRenderer};
use tekes_ui_scene::{ImageId, Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme};
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
fn render_scene(
    h: &mut ContentHost,
    text: &mut coretext::Text,
    width: f32,
    clip: Rect,
    theme: Theme,
) -> Scene {
    let f = Rect::new(0., 0., width, 700.);
    let mut scene = Scene::new();
    h.render(
        &mut scene,
        &mut WithIconRenderer::new(text, &mut Icons),
        f,
        clip,
        theme.into(),
    )
    .unwrap();
    scene
}
fn render(h: &mut ContentHost, text: &mut coretext::Text, width: f32, clip: Rect, theme: Theme) {
    let _ = render_scene(h, text, width, clip, theme);
}
fn parts(h: &ContentHost) -> Vec<SemanticPart> {
    h.semantics()
        .into_iter()
        .find(|n| n.id == TABLE)
        .unwrap()
        .parts
}
#[test]
fn real_composer_preedit_binding_selection_and_barriers_survive_background_table_updates() {
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
        .find(|p| matches!(p.semantics.role, ContentRole::TableRow { index: 0 }))
        .unwrap();
    for a in [AccessibilityAction::Focus, AccessibilityAction::Press] {
        assert!(!h.accessibility_action(TABLE, Some(row.index), a).consumed);
    }
    h.event(&ContentEvent::Key {
        key: tekes_ui_controls::ControlKey::Tab { reverse: false },
        down: true,
    });
    assert_eq!(h.focused(), Some(COMPOSER));
    reorder(&mut h).unwrap();
    h.set_inherited_surface("canvas").unwrap();
    h.get_mut(TABLE)
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
    h.get_mut(TABLE).unwrap().set_surface(None).unwrap();
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
        .find(|p| matches!(p.semantics.role, ContentRole::TableRow { index: 0 }))
        .unwrap();
    let point = Point::new(row.frame.min_x() + 5., row.frame.min_y() + 5.);
    h.event_with_text(&ContentEvent::PointerDown(point), &mut text)
        .unwrap();
    assert_eq!(h.focused(), Some(TABLE));
    assert!(!h.is_composing());
    assert!(
        h.drain_action_records()
            .iter()
            .all(|r| r.action.kind != ContentActionKind::Submit)
    );
}
#[test]
fn dynamic_passive_slots_hidden_keep_and_header_row_actions_use_original_owners() {
    let mut h = build().unwrap();
    h.set_action_events(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let f = Rect::new(0., 0., 500., 700.);
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    let row = parts(&h)
        .into_iter()
        .find(|p| matches!(p.semantics.role, ContentRole::TableRow { index: 0 }))
        .unwrap();
    assert!(
        h.accessibility_action(TABLE, Some(row.index), AccessibilityAction::Press)
            .consumed
    );
    let i = match h.drain_action_records().remove(0).payload.unwrap() {
        ContentActionPayload::TableIntent(i) => i,
        _ => panic!("table intent"),
    };
    reorder(&mut h).unwrap();
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    assert!(
        h.get(TABLE)
            .unwrap()
            .component::<DataTableContent>()
            .unwrap()
            .intent_is_current(&i)
    );
    assert_eq!(
        h.get(TABLE)
            .unwrap()
            .component::<DataTableContent>()
            .unwrap()
            .selected_key(),
        Some(TableRowKey(1))
    );
    let mount = h.semantic_mount(ComponentId(202));
    let mut s = snapshot(true);
    s.display_rows.retain(|k| *k != TableRowKey(1));
    let mut d = displays(&s);
    replace(&mut h, &s, &mut d).unwrap();
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    assert!(h.semantic_mount(ComponentId(202)).is_none());
    assert!(
        !h.get(TABLE)
            .unwrap()
            .component::<DataTableContent>()
            .unwrap()
            .intent_is_current(&i)
    );
    reorder(&mut h).unwrap();
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    assert_eq!(h.semantic_mount(ComponentId(202)), mount);
    let s = snapshot(true);
    let mut d = displays(&s);
    d.rows[0].cells[0].display = TableDisplayUpdate::DefaultText;
    d.rows[0].cells[1].accessory = TableDisplayUpdate::DefaultText;
    replace(&mut h, &s, &mut d).unwrap();
    render(&mut h, &mut text, 500., f, Theme::LIGHT);
    assert!(h.get(ComponentId(202)).is_none());
    assert!(h.get(ComponentId(203)).is_none());
}

#[test]
fn original_scaled_typography_keeps_row_wrapper_and_semantics_on_one_layout() {
    let mut h = build().unwrap();
    let s = snapshot(false);
    let mut d = displays(&s);
    d.headers[0].display = TableDisplayUpdate::DefaultText;
    d.rows[0].cells[0].display = TableDisplayUpdate::DefaultText;
    d.rows[0].cells[1].accessory = TableDisplayUpdate::DefaultText;
    replace(&mut h, &s, &mut d).unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    for scale in [1., 2., 1.] {
        text.scale = scale;
        render(
            &mut h,
            &mut text,
            500.,
            Rect::new(0., 0., 500., 700.),
            Theme::LIGHT,
        );
        let row = parts(&h)
            .into_iter()
            .find(|p| matches!(p.semantics.role, ContentRole::TableRow { index: 0 }))
            .unwrap();
        assert_eq!(
            row.frame,
            h.get(ComponentId(101)).unwrap().frame(),
            "scale={scale}"
        );
    }
}

#[test]
fn table_header_surface_preserves_default_inherited_explicit_and_clear_contract() {
    use tekes_ui_controls::extended::{SortDirection, TableColumnKey};
    use tekes_ui_tokens::surface_palette::resolve_surface;

    let mut host = build().unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let viewport = Rect::new(0., 0., 500., 700.);
    render_scene(&mut host, &mut text, 500., viewport, Theme::LIGHT);
    let header = parts(&host)
        .into_iter()
        .find(|part| {
            matches!(
                part.semantics.role,
                ContentRole::TableColumnHeader { index: 0, .. }
            )
        })
        .unwrap();
    assert!(
        host.accessibility_action(TABLE, Some(header.index), AccessibilityAction::Press)
            .consumed
    );
    render_scene(&mut host, &mut text, 500., viewport, Theme::LIGHT);
    let row = parts(&host)
        .into_iter()
        .find(|part| matches!(part.semantics.role, ContentRole::TableRow { index: 0 }))
        .unwrap();
    assert!(
        host.accessibility_action(TABLE, Some(row.index), AccessibilityAction::Press)
            .consumed
    );
    render_scene(&mut host, &mut text, 500., viewport, Theme::LIGHT);

    let baseline = parts(&host);
    let table = host
        .get(TABLE)
        .unwrap()
        .component::<DataTableContent>()
        .unwrap();
    let sort = table.sort_key_state();
    let selected = table.selected_key();
    assert_eq!(sort, Some((TableColumnKey(1), SortDirection::Ascending)));
    assert_eq!(selected, Some(TableRowKey(1)));
    assert_eq!(
        baseline
            .iter()
            .filter(|part| {
                matches!(part.semantics.role, ContentRole::TableCell { .. })
                    && part.semantics.value.is_some()
            })
            .count(),
        6
    );

    let palette = Palette::from(Theme::LIGHT);
    let assert_state = |host: &ContentHost, scene: &Scene, expected: Rgba| {
        let header = parts(host)
            .into_iter()
            .find(|part| {
                matches!(
                    part.semantics.role,
                    ContentRole::TableColumnHeader { index: 0, .. }
                )
            })
            .unwrap();
        let fill = scene
            .primitives()
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Quad(quad) if quad.rect == header.frame && quad.fill.a > 0. => {
                    Some(quad.fill)
                }
                _ => None,
            })
            .expect("filled quad at the actual first-header frame");
        assert_eq!(fill, expected);
        assert_eq!(parts(host), baseline);
        let table = host
            .get(TABLE)
            .unwrap()
            .component::<DataTableContent>()
            .unwrap();
        assert_eq!(table.sort_key_state(), sort);
        assert_eq!(table.selected_key(), selected);
    };

    let scene = render_scene(&mut host, &mut text, 500., viewport, Theme::LIGHT);
    assert_state(&host, &scene, palette.color("surface-2").unwrap());

    host.get_mut(TABLE)
        .unwrap()
        .set_surface(Some("sidebar"))
        .unwrap();
    let scene = render_scene(&mut host, &mut text, 500., viewport, Theme::LIGHT);
    assert_state(
        &host,
        &scene,
        resolve_surface(palette, "sidebar", "action-quiet", "rest")
            .unwrap()
            .background,
    );
    host.get_mut(TABLE).unwrap().set_surface(None).unwrap();
    let scene = render_scene(&mut host, &mut text, 500., viewport, Theme::LIGHT);
    assert_state(&host, &scene, palette.color("surface-2").unwrap());

    host.set_inherited_surface("panel").unwrap();
    let scene = render_scene(&mut host, &mut text, 500., viewport, Theme::LIGHT);
    assert_state(
        &host,
        &scene,
        resolve_surface(palette, "panel", "action-quiet", "rest")
            .unwrap()
            .background,
    );
    host.get_mut(TABLE)
        .unwrap()
        .set_surface(Some("sidebar"))
        .unwrap();
    let scene = render_scene(&mut host, &mut text, 500., viewport, Theme::LIGHT);
    assert_state(
        &host,
        &scene,
        resolve_surface(palette, "sidebar", "action-quiet", "rest")
            .unwrap()
            .background,
    );
    host.get_mut(TABLE).unwrap().set_surface(None).unwrap();
    let scene = render_scene(&mut host, &mut text, 500., viewport, Theme::LIGHT);
    assert_state(
        &host,
        &scene,
        resolve_surface(palette, "panel", "action-quiet", "rest")
            .unwrap()
            .background,
    );
}
