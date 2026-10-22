use tekes_ui_composition_consumer::toolbar_groups::*;
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text(f32);
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new(
            (s.chars().count() as f32 * self.0 * 0.5).min(width),
            role.style().line_height * self.0 / proposal::TOOLBAR_REFERENCE_TITLE_SIZE,
        )
    }
    fn resolved_font_size(&mut self, _: proposal::TextRole) -> f32 {
        self.0
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
fn paint(h: &mut ContentHost, width: f32, size: f32, x: f32) {
    let f = Rect::new(x, 20., width, 42.);
    h.render(
        &mut Scene::new(),
        &mut Text(size),
        f,
        Rect::new(x, 20., width, 200.),
        Theme::LIGHT.into(),
    )
    .unwrap();
}
#[test]
fn original_text_and_actions_resolve_inside_the_receiving_toolbar() {
    let mut h = build().unwrap();
    paint(&mut h, 800., 13., 200.);
    let title = h.get(TITLE).unwrap().frame();
    let status = h.get(STATUS).unwrap().frame();
    let save = h.get(SAVE).unwrap().frame();
    let publish = h.get(OTHER).unwrap().frame();
    assert!(title.min_x() >= 200. && title.max_x() < status.min_x());
    assert!((status.center().x - 600.).abs() < 0.01);
    assert!(status.max_x() < save.min_x() && save.max_x() < publish.min_x());
    assert_eq!(
        h.focus_targets()
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        vec![SAVE, OTHER]
    );
    let n = h.semantics().into_iter().find(|n| n.id == TITLE).unwrap();
    assert_eq!(n.semantics.role, ContentRole::Text);
    assert!(!n.focused);
    h.event(&ContentEvent::PointerDown(save.center()));
    h.event(&ContentEvent::PointerUp(save.center()));
    let actions = h.drain_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].target, SAVE);
}
#[test]
fn group_reorder_and_placement_update_retain_original_focus_and_mounts() {
    let mut h = build().unwrap();
    paint(&mut h, 800., 13., 0.);
    h.focus(Some(SAVE), true);
    let mount = h.semantic_mount(SAVE);
    h.reorder(BAR, &[NAV, ACTION_GROUP, STATUS_GROUP]).unwrap();
    h.get_mut(NAV)
        .unwrap()
        .component_mut::<ToolbarGroup>()
        .unwrap()
        .placement = ToolbarPlacement::Principal;
    paint(&mut h, 800., 13., 0.);
    assert_eq!(h.semantic_mount(SAVE), mount);
    assert_eq!(h.focused(), Some(SAVE));
    assert!(h.drain_actions().is_empty());
    let removed = h.remove(ACTION_GROUP).unwrap();
    h.insert(BAR, removed).unwrap();
    paint(&mut h, 800., 13., 0.);
    assert_ne!(h.semantic_mount(SAVE), mount);
}
#[test]
fn title_scale_changes_toolbar_geometry_without_a_native_input() {
    let mut h = build().unwrap();
    paint(&mut h, 1000., 13., 0.);
    let a = h.get(BAR).unwrap().frame().size.height;
    paint(&mut h, 1000., 20., 0.);
    let b = h.get(BAR).unwrap().frame().size.height;
    assert!((b / a - 20. / 13.).abs() < 0.0001);
    assert_eq!(
        h.semantics()
            .iter()
            .filter(|n| n.semantics.role == ContentRole::Button)
            .count(),
        2
    );
}
#[test]
fn narrow_regions_do_not_overlap_and_never_make_passive_text_focusable() {
    for width in [0., 1., 16., 32., 80., 160., 320., 600.] {
        let mut h = build().unwrap();
        paint(&mut h, width, 20., 50.);
        let groups = [NAV, STATUS_GROUP, ACTION_GROUP].map(|id| h.get(id).unwrap());
        for (i, a) in groups.iter().enumerate() {
            for b in &groups[i + 1..] {
                if let (Some(a), Some(b)) = (a.visible(), b.visible()) {
                    assert!(
                        a.intersection(&b).is_none_or(|r| r.is_empty()),
                        "width={width}: {a:?}, {b:?}"
                    );
                }
            }
        }
        assert!(h.focus_targets().iter().all(|(id, _)| *id != TITLE));
        assert!(h.get(BAR).unwrap().frame().size.height.is_finite());
    }
}

#[test]
fn secondary_action_and_status_keep_distinct_original_control_contracts() {
    let mut h = build().unwrap();
    paint(&mut h, 800., 13., 0.);
    h.focus(Some(SAVE), true);
    let mount = h.semantic_mount(SAVE);
    h.get_mut(ACTION_GROUP)
        .unwrap()
        .component_mut::<ToolbarGroup>()
        .unwrap()
        .placement = ToolbarPlacement::SecondaryAction;
    paint(&mut h, 800., 13., 0.);
    assert_eq!(h.semantic_mount(SAVE), mount);
    assert_eq!(h.focused(), Some(SAVE));
    assert_eq!(
        h.semantics()
            .into_iter()
            .find(|n| n.id == SAVE)
            .unwrap()
            .semantics
            .role,
        ContentRole::Button
    );
    assert_eq!(
        h.semantics()
            .into_iter()
            .find(|n| n.id == STATUS)
            .unwrap()
            .semantics
            .role,
        ContentRole::Text
    );
    assert!(h.drain_actions().is_empty());
}

#[derive(Debug)]
struct ConsumerDocumentState {
    revision: u64,
    paints: std::rc::Rc<std::cell::Cell<usize>>,
}
impl Component for ConsumerDocumentState {
    fn measure(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(text.measure(
            &format!("Revision {}", self.revision),
            proposal::TextRole::UiBody,
            available.width,
        ))
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Text, format!("Revision {}", self.revision))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        self.paints.set(self.paints.get() + 1);
        text.draw(
            scene,
            &format!("Revision {}", self.revision),
            proposal::TextRole::UiBody,
            frame,
            ButtonTextAlign::Leading,
            theme.palette.color("primary-text").expect("DS text role"),
        );
    }
}
#[test]
fn unknown_consumer_content_keeps_owned_state_and_original_mount_during_group_reorder() {
    let mut h = build().unwrap();
    let paints = std::rc::Rc::new(std::cell::Cell::new(0));
    let custom = ComponentId(19508);
    h.insert(
        STATUS_GROUP,
        Content::new(
            custom,
            ConsumerDocumentState {
                revision: 7,
                paints: paints.clone(),
            },
        ),
    )
    .unwrap();
    paint(&mut h, 1200., 13., 200.);
    let mount = h.semantic_mount(custom);
    assert_eq!(paints.get(), 1);
    h.reorder(BAR, &[NAV, ACTION_GROUP, STATUS_GROUP]).unwrap();
    paint(&mut h, 1200., 20., 200.);
    assert_eq!(h.semantic_mount(custom), mount);
    assert_eq!(
        h.get(custom)
            .unwrap()
            .component::<ConsumerDocumentState>()
            .unwrap()
            .revision,
        7
    );
    assert_eq!(paints.get(), 2);
    assert!(h.focus_targets().iter().all(|(id, _)| *id != custom));
    assert!(h.drain_actions().is_empty());
}
