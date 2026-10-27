use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
const ROOT: ComponentId = ComponentId(18100);
const MENU: ComponentId = ComponentId(18101);
struct Text;
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new(
            (s.chars().count() as f32 * 8.).min(w),
            r.style().line_height,
        )
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
        _: Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}
fn entries() -> Vec<MenuEntry> {
    vec![
        MenuEntry::submenu(
            "sort",
            "Sort by",
            vec![
                MenuEntry::Item {
                    value: "recent".into(),
                    label: "Recent".into(),
                    checked: Some(true),
                    disabled: false,
                },
                MenuEntry::Item {
                    value: "title-asc".into(),
                    label: "Title disabled".into(),
                    checked: Some(false),
                    disabled: true,
                },
                MenuEntry::Separator,
                MenuEntry::Item {
                    value: "title-desc".into(),
                    label: "Title Z–A".into(),
                    checked: Some(false),
                    disabled: false,
                },
            ],
        ),
        MenuEntry::submenu(
            "group",
            "Group by",
            vec![
                MenuEntry::item("none", "None"),
                MenuEntry::Item {
                    value: "folder".into(),
                    label: "Primary Folder".into(),
                    checked: Some(true),
                    disabled: false,
                },
            ],
        ),
        MenuEntry::Separator,
        MenuEntry::Item {
            value: "pin-up".into(),
            label: "Move Pin Up".into(),
            checked: None,
            disabled: true,
        },
        MenuEntry::item("pin-down", "Move Pin Down"),
    ]
}
fn menu(h: &mut ContentHost) -> &mut MenuContent {
    h.get_mut(MENU)
        .unwrap()
        .component_mut::<MenuContent>()
        .unwrap()
}
fn host() -> ContentHost {
    let mut m = MenuContent::new("Session list options", Vec::new());
    m.set_entries(entries()).unwrap();
    let mut h = ContentHost::new(Content::new(
        ROOT,
        ListContent::new("Sidebar", vec![Content::new(MENU, m)]),
    ))
    .unwrap();
    h.set_action_events(true);
    paint(&mut h);
    h
}
fn paint(h: &mut ContentHost) {
    let f = Rect::new(0., 0., 640., 500.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
}
fn parts(h: &ContentHost) -> Vec<SemanticPart> {
    h.semantics()
        .into_iter()
        .find(|n| n.id == MENU)
        .unwrap()
        .parts
}
fn part(h: &ContentHost, label: &str) -> SemanticPart {
    parts(h)
        .into_iter()
        .find(|p| p.semantics.name == label)
        .unwrap()
}
fn press(h: &mut ContentHost, label: &str) -> ContentResponse {
    h.accessibility_action(MENU, Some(part(h, label).index), AccessibilityAction::Press)
}
fn open(h: &mut ContentHost) {
    h.accessibility_action(MENU, None, AccessibilityAction::Press);
    paint(h);
}
fn key(h: &mut ContentHost, k: ControlKey) {
    h.event(&ContentEvent::Key { key: k, down: true });
    h.event(&ContentEvent::Key {
        key: k,
        down: false,
    });
    paint(h);
}
fn selection(record: &ContentActionRecord) -> &MenuSelection {
    let ContentActionPayload::MenuSelection(s) = record.payload.as_ref().unwrap() else { panic!("expected original MenuSelection payload"); };
    s
}
#[test]
fn two_level_keyboard_and_ax_selection_capture_stable_paths_once() {
    let mut h = host();
    open(&mut h);
    assert!(h.drain_actions().is_empty());
    assert!(press(&mut h, "Sort by").consumed);
    paint(&mut h);
    assert!(h.drain_actions().is_empty());
    assert!(press(&mut h, "Recent").activated);
    open(&mut h);
    key(&mut h, ControlKey::Right);
    assert_eq!(part(&h, "Sort by").semantics.expanded, Some(true));
    key(&mut h, ControlKey::Down);
    assert!(part(&h, "Title Z–A").focused);
    key(&mut h, ControlKey::Enter);
    menu(&mut h)
        .set_entries(vec![MenuEntry::item("new", "New model")])
        .unwrap();
    let records = h.drain_action_records();
    assert_eq!(records.len(), 2);
    assert_eq!(
        selection(&records[0]).value_path().unwrap(),
        ["sort", "recent"]
    );
    assert_eq!(
        selection(&records[1]).value_path().unwrap(),
        ["sort", "title-desc"]
    );
    assert_eq!(records[0].action.source, ContentActionSource::Accessibility);
    assert_eq!(records[1].action.source, ContentActionSource::Keyboard);
    assert!(h.drain_actions().is_empty());
}
#[test]
fn submenu_pointer_hover_and_click_use_original_layout_and_action_owner() {
    let mut h = host();
    open(&mut h);
    let parent = part(&h, "Group by").frame.center();
    h.event(&ContentEvent::PointerMove(parent));
    paint(&mut h);
    let p = part(&h, "Primary Folder");
    h.event(&ContentEvent::PointerDown(p.frame.center()));
    h.event(&ContentEvent::PointerUp(p.frame.center()));
    let r = h.drain_action_records();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].action.target, MENU);
    assert_eq!(r[0].action.source, ContentActionSource::Pointer);
    assert_eq!(selection(&r[0]).value_path().unwrap(), ["group", "folder"]);
}
#[test]
fn semantic_hierarchy_checks_disabled_and_focus_are_distinct() {
    let mut h = host();
    open(&mut h);
    let parent = part(&h, "Sort by");
    press(&mut h, "Sort by");
    paint(&mut h);
    let recent = part(&h, "Recent");
    let group = h.semantic_part_parent(MENU, recent.index).unwrap();
    assert_eq!(h.semantic_part_parent(MENU, group), Some(parent.index));
    assert_eq!(
        parts(&h)
            .iter()
            .find(|p| p.index == group)
            .unwrap()
            .semantics
            .role,
        ContentRole::Menu
    );
    assert!(recent.semantics.selected);
    assert_eq!(recent.semantics.value.as_deref(), Some("on"));
    let disabled = part(&h, "Title disabled");
    assert!(disabled.semantics.disabled);
    assert!(!press(&mut h, "Title disabled").activated);
    assert_eq!(parts(&h).iter().filter(|p| p.focused).count(), 1);
    key(&mut h, ControlKey::Down);
    assert!(part(&h, "Recent").semantics.selected);
    assert!(!part(&h, "Recent").focused);
    assert!(part(&h, "Title Z–A").focused);
    assert!(h.drain_actions().is_empty());
}
#[test]
fn closed_reopened_replaced_and_unmounted_popup_tokens_cannot_activate_new_model() {
    let mut h = host();
    open(&mut h);
    press(&mut h, "Sort by");
    paint(&mut h);
    let old = part(&h, "Recent").index;
    key(&mut h, ControlKey::Escape);
    open(&mut h);
    press(&mut h, "Sort by");
    paint(&mut h);
    assert_ne!(part(&h, "Recent").index, old);
    assert!(
        !h.accessibility_action(MENU, Some(old), AccessibilityAction::Press)
            .activated
    );
    let current = part(&h, "Recent").index;
    menu(&mut h)
        .set_entries(vec![MenuEntry::item("different", "Different")])
        .unwrap();
    assert!(
        !h.accessibility_action(MENU, Some(current), AccessibilityAction::Press)
            .activated
    );
    open(&mut h);
    let current = part(&h, "Different").index;
    let node = h.remove(MENU).unwrap();
    h.insert(ROOT, node).unwrap();
    paint(&mut h);
    assert!(
        !h.accessibility_action(MENU, Some(current), AccessibilityAction::Press)
            .activated
    );
    assert!(h.drain_actions().is_empty());
}
#[test]
fn left_returns_to_parent_and_outside_dismissal_has_no_selection() {
    let mut h = host();
    open(&mut h);
    key(&mut h, ControlKey::Right);
    assert!(parts(&h).iter().any(|p| p.semantics.name == "Recent"));
    key(&mut h, ControlKey::Left);
    assert!(!parts(&h).iter().any(|p| p.semantics.name == "Recent"));
    assert!(part(&h, "Sort by").focused);
    h.event(&ContentEvent::PointerDown(Point::new(620., 480.)));
    h.event(&ContentEvent::PointerUp(Point::new(620., 480.)));
    assert!(!menu(&mut h).menu.is_open());
    assert!(parts(&h).is_empty());
    assert!(h.drain_actions().is_empty());
}
#[test]
fn invalid_declaration_rejects_atomically_and_equal_tree_is_noop() {
    let mut h = host();
    open(&mut h);
    let rev = menu(&mut h).model_revision();
    menu(&mut h).set_entries(entries()).unwrap();
    assert_eq!(menu(&mut h).model_revision(), rev);
    assert!(menu(&mut h).menu.is_open());
    for invalid in [
        vec![MenuEntry::item("dup", "A"), MenuEntry::item("dup", "B")],
        vec![MenuEntry::item(" ", "A")],
        vec![MenuEntry::item("a", " ")],
    ] {
        assert!(menu(&mut h).set_entries(invalid).is_err());
        assert_eq!(menu(&mut h).model_revision(), rev);
        assert!(menu(&mut h).menu.is_open());
    }
    assert!(h.drain_actions().is_empty());
}

#[test]
fn original_compound_trigger_is_one_exact_action_with_passive_body_and_name() {
    use tekes_ui_composition_consumer::declared_menu as f;
    let mut h = f::build().unwrap();
    paint(&mut h);
    let nodes = h.semantics();
    let menu = nodes.iter().find(|n| n.id == f::OPTIONS).unwrap();
    assert_eq!(menu.semantics.name, "Session list options");
    assert_eq!(menu.semantics.role, ContentRole::MenuButton);
    assert!(!nodes.iter().any(|n| n.id == f::GLYPH));
    assert!(menu.frame.size.width < 60.);
    let original = h
        .get(f::OPTIONS)
        .unwrap()
        .component::<MenuContent>()
        .unwrap()
        .trigger
        .activations;
    let p = menu.frame.center();
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::PointerUp(p));
    paint(&mut h);
    assert!(h.drain_actions().is_empty());
    assert_eq!(
        h.get(f::OPTIONS)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .trigger
            .activations,
        original + 1
    );
    let parts = h
        .semantics()
        .into_iter()
        .find(|n| n.id == f::OPTIONS)
        .unwrap()
        .parts;
    assert!(parts.iter().any(|p| p.semantics.name == "Sort by"));
}
#[test]
fn interactive_trigger_body_uses_existing_action_owner_validation() {
    let m = MenuContent::new("Options", Vec::new())
        .with_trigger_content(
            "Options",
            Content::new(ComponentId(18290), Button::new("Nested action")),
        )
        .unwrap();
    assert!(ContentHost::new(Content::new(MENU, m)).is_err());
    assert!(
        MenuContent::new("Options", Vec::new())
            .with_trigger_content(" ", Content::new(ComponentId(18291), TextContent::new("⋮")))
            .is_err()
    );
}
#[test]
fn compact_body_instance_survives_item_model_replacement() {
    use tekes_ui_composition_consumer::declared_menu as f;
    let mut h = f::build().unwrap();
    paint(&mut h);
    assert!(h.get(f::GLYPH).is_some());
    let id = h.get(f::GLYPH).unwrap().id();
    h.get_mut(f::OPTIONS)
        .unwrap()
        .component_mut::<MenuContent>()
        .unwrap()
        .set_entries(vec![MenuEntry::item("new", "New")])
        .unwrap();
    paint(&mut h);
    assert_eq!(h.get(f::GLYPH).unwrap().id(), id);
    assert_eq!(
        h.get(f::OPTIONS)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .trigger
            .label,
        "Session list options"
    );
}

#[test]
fn reopening_original_view_without_a_paint_revokes_previous_semantic_generation() {
    let mut h = host();
    open(&mut h);
    let old = part(&h, "Sort by").index;
    menu(&mut h).menu.deactivate();
    menu(&mut h).menu.show(0);
    assert!(
        !h.accessibility_action(MENU, Some(old), AccessibilityAction::Press)
            .consumed
    );
    assert!(parts(&h).is_empty());
    paint(&mut h);
    assert_ne!(part(&h, "Sort by").index, old);
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn switching_hovered_parent_retires_old_child_and_scroll_revokes_new_child() {
    let mut h = host();
    open(&mut h);
    h.event(&ContentEvent::PointerMove(
        part(&h, "Sort by").frame.center(),
    ));
    paint(&mut h);
    let old = part(&h, "Recent").index;
    h.event(&ContentEvent::PointerMove(
        part(&h, "Group by").frame.center(),
    ));
    paint(&mut h);
    assert!(!parts(&h).iter().any(|p| p.semantics.name == "Recent"));
    assert!(
        parts(&h)
            .iter()
            .any(|p| p.semantics.name == "Primary Folder")
    );
    assert!(
        !h.accessibility_action(MENU, Some(old), AccessibilityAction::Press)
            .activated
    );
    h.event(&ContentEvent::Scroll {
        position: part(&h, "Group by").frame.center(),
        delta: Point::new(0., 10.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 0.,
    });
    paint(&mut h);
    assert!(
        !parts(&h)
            .iter()
            .any(|p| p.semantics.name == "Primary Folder")
    );
    assert!(h.drain_actions().is_empty());
}
#[test]
fn nested_levels_keep_full_value_path_and_cancel_revokes_all_panels() {
    let mut h = host();
    menu(&mut h)
        .set_entries(vec![MenuEntry::submenu(
            "one",
            "One",
            vec![MenuEntry::submenu(
                "two",
                "Two",
                vec![MenuEntry::item("three", "Three")],
            )],
        )])
        .unwrap();
    open(&mut h);
    press(&mut h, "One");
    paint(&mut h);
    press(&mut h, "Two");
    paint(&mut h);
    assert!(press(&mut h, "Three").activated);
    let a = h.drain_action_records();
    assert_eq!(
        selection(&a[0]).value_path().unwrap(),
        ["one", "two", "three"]
    );
    open(&mut h);
    press(&mut h, "One");
    paint(&mut h);
    let old = part(&h, "Two").index;
    h.event(&ContentEvent::Cancel);
    assert!(parts(&h).is_empty());
    assert!(
        !h.accessibility_action(MENU, Some(old), AccessibilityAction::Press)
            .activated
    );
    assert!(h.drain_actions().is_empty());
}
#[test]
fn semantic_menu_surfaces_stay_inside_real_host_clip_at_edges_and_narrow_width() {
    for viewport in [
        Rect::new(0., 0., 640., 500.),
        Rect::new(20., 30., 340., 210.),
    ] {
        let mut h = host();
        let frame = Rect::new(viewport.max_x() - 80., viewport.max_y() - 60., 80., 50.);
        h.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            viewport,
            Theme::LIGHT.into(),
        )
        .unwrap();
        h.accessibility_action(MENU, None, AccessibilityAction::Press);
        h.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            viewport,
            Theme::LIGHT.into(),
        )
        .unwrap();
        press(&mut h, "Sort by");
        h.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            viewport,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let groups = parts(&h)
            .into_iter()
            .filter(|p| p.semantics.role == ContentRole::Menu)
            .collect::<Vec<_>>();
        assert_eq!(groups.len(), 2);
        for group in groups {
            assert_eq!(group.frame.intersection(&viewport), Some(group.frame));
        }
    }
}
#[test]
fn semantic_submenu_selection_does_not_edit_or_unmark_focused_original_composer() {
    use tekes_ui_composition_consumer::declared_menu as f;
    let mut h = f::build().unwrap();
    paint(&mut h);
    h.focus(Some(f::MESSAGE), false);
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: vec![],
        replacement: None,
    });
    paint(&mut h);
    let before = h.focused_text().unwrap().input.external_baseline();
    h.get_mut(f::OPTIONS)
        .unwrap()
        .component_mut::<MenuContent>()
        .unwrap()
        .menu
        .show(0);
    paint(&mut h);
    let find = |h: &ContentHost, name: &str| {
        h.semantics()
            .into_iter()
            .find(|n| n.id == f::OPTIONS)
            .unwrap()
            .parts
            .into_iter()
            .find(|p| p.semantics.name == name)
            .unwrap()
            .index
    };
    let sort = find(&h, "Sort by");
    h.accessibility_action(f::OPTIONS, Some(sort), AccessibilityAction::Press);
    paint(&mut h);
    let recent = find(&h, "Recent activity");
    assert!(
        h.accessibility_action(f::OPTIONS, Some(recent), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(h.focused(), Some(f::MESSAGE));
    assert_eq!(h.focused_text().unwrap().input.external_baseline(), before);
    assert_eq!(h.drain_action_records().len(), 1);
}

#[test]
fn ax_opening_second_submenu_updates_original_active_parent_before_left_return() {
    let mut h = host();
    open(&mut h);
    press(&mut h, "Group by");
    assert_eq!(
        menu(&mut h).menu.menu().items.selected,
        1,
        "AX navigation must select/reveal the real requested parent, not keep index zero"
    );
    paint(&mut h);
    key(&mut h, ControlKey::Left);
    assert!(part(&h, "Group by").focused);
    assert!(!part(&h, "Sort by").focused);
    assert!(h.drain_actions().is_empty());
}

#[derive(Debug)]
struct DelegatedMenu(MenuContent);
impl Component for DelegatedMenu {
    fn measure(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.0.measure(t, a)
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.0.paint(s, t, f, c, theme)
    }
    fn paint_overlay(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.0.paint_overlay(s, t, f, c, theme)
    }
    fn overlay_open(&self) -> bool {
        self.0.overlay_open()
    }
    fn overlay_presented(&self) -> bool {
        self.0.overlay_presented()
    }
    fn overlay_event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.0.overlay_event(e)
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.0.event(e)
    }
    fn focusable(&self) -> bool {
        self.0.focusable()
    }
    fn set_focus(&mut self, f: bool, v: bool) {
        self.0.set_focus(f, v)
    }
    fn lifecycle(&mut self, e: Lifecycle) {
        self.0.lifecycle(e)
    }
    fn semantics(&self) -> ContentSemantics {
        self.0.semantics()
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        self.0.semantic_parts()
    }
    fn semantic_part_parent(&self, p: usize) -> Option<usize> {
        self.0.semantic_part_parent(p)
    }
    fn accessibility_action(
        &mut self,
        p: Option<usize>,
        a: &AccessibilityAction,
    ) -> ContentResponse {
        self.0.accessibility_action(p, a)
    }
    fn action_payload(&self, k: ContentActionKind) -> Option<ContentActionPayload> {
        self.0.action_payload(k)
    }
}
#[test]
fn independent_wrapper_forwards_original_parts_and_owned_action_without_private_sdk_access() {
    let mut m = MenuContent::new("Options", Vec::new());
    m.set_entries(entries()).unwrap();
    let mut h = ContentHost::new(Content::new(
        ROOT,
        ListContent::new("Sidebar", vec![Content::new(MENU, DelegatedMenu(m))]),
    ))
    .unwrap();
    h.set_action_events(true);
    paint(&mut h);
    open(&mut h);
    let sort = part(&h, "Sort by").index;
    press(&mut h, "Sort by");
    paint(&mut h);
    let recent = part(&h, "Recent").index;
    let group = h.semantic_part_parent(MENU, recent).unwrap();
    assert_eq!(h.semantic_part_parent(MENU, group), Some(sort));
    assert!(press(&mut h, "Recent").activated);
    let a = h.drain_action_records();
    assert_eq!(a.len(), 1);
    assert_eq!(selection(&a[0]).value_path().unwrap(), ["sort", "recent"]);
}

#[test]
fn old_declared_token_cannot_alias_a_legacy_position_after_model_mode_switch() {
    let mut h = host();
    open(&mut h);
    let old = part(&h, "Sort by").index;
    let count = old.checked_add(1).filter(|&n| n < 10_000).unwrap_or(8);
    menu(&mut h).labels = (0..count).map(|n| format!("Legacy {n}")).collect();
    menu(&mut h)
        .set_values((0..count).map(|n| format!("legacy-{n}")).collect())
        .unwrap();
    open(&mut h);
    assert!(
        !h.accessibility_action(MENU, Some(old), AccessibilityAction::Press)
            .activated,
        "a delayed declared popup token cannot become a positional command in a new legacy model"
    );
    assert!(h.drain_actions().is_empty());
}

fn scroll(h: &mut ContentHost, p: Point, y: f32, begin: bool, end: bool, time: f64) {
    assert!(
        h.event(&ContentEvent::Scroll {
            position: p,
            delta: Point::new(0., y),
            precise: true,
            begin,
            end,
            momentum: false,
            time
        })
        .consumed
    );
}
#[test]
fn precise_child_scroll_keeps_original_capture_across_parent_boundary() {
    let mut h = host();
    menu(&mut h)
        .set_entries(vec![MenuEntry::submenu(
            "long",
            "Long",
            (0..30)
                .map(|i| MenuEntry::item(format!("v{i}"), format!("Row {i}")))
                .collect(),
        )])
        .unwrap();
    open(&mut h);
    press(&mut h, "Long");
    paint(&mut h);
    let child = part(&h, "Row 2").frame.center();
    let parent = part(&h, "Long").frame.center();
    scroll(&mut h, child, 30., true, false, 1.);
    paint(&mut h);
    let before = part(&h, "Row 2").frame.origin.y;
    scroll(&mut h, parent, 30., false, false, 1.02);
    paint(&mut h);
    assert_eq!(
        part(&h, "Long").semantics.expanded,
        Some(true),
        "captured child must stay mounted when pointer crosses its parent"
    );
    assert!(
        part(&h, "Row 2").frame.origin.y < before,
        "tail stays with original child ScrollArea"
    );
    scroll(&mut h, parent, 0., false, true, 1.04);
    assert!(h.drain_actions().is_empty());
}
struct ClockText {
    now: std::time::Duration,
    requests: usize,
    reduced: bool,
}
impl ButtonTextRenderer for ClockText {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Text.measure(s, r, w)
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
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        Some(tekes_ui_controls::ControlMotionContext {
            now: self.now,
            reduced_motion: self.reduced,
        })
    }
    fn request_animation_frame(&mut self) {
        self.requests += 1;
    }
}
#[test]
fn content_adapter_advances_original_menu_rebound_from_host_clock() {
    for mode in 0..3 {
        for reduced in [false, true] {
            let mut h = host();
            let entries = (0..30)
                .map(|i| MenuEntry::item(format!("v{i}"), format!("Row {i}")))
                .collect::<Vec<_>>();
            if mode == 0 {
                let m = menu(&mut h);
                m.labels = (0..30).map(|i| format!("Row {i}")).collect();
            } else if mode == 1 {
                menu(&mut h).set_entries(entries).unwrap();
            } else {
                menu(&mut h)
                    .set_entries(vec![MenuEntry::submenu("long", "Long", entries)])
                    .unwrap();
            }
            let mut text = ClockText {
                now: std::time::Duration::ZERO,
                requests: 0,
                reduced: false,
            };
            let f = Rect::new(0., 0., 360., 240.);
            let render = |h: &mut ContentHost, t: &mut ClockText| {
                h.render(&mut Scene::new(), t, f, f, Theme::LIGHT.into())
                    .unwrap()
            };
            h.accessibility_action(MENU, None, AccessibilityAction::Press);
            render(&mut h, &mut text);
            if mode == 2 {
                press(&mut h, "Long");
                render(&mut h, &mut text);
            }
            text.now = std::time::Duration::from_millis(200);
            render(&mut h, &mut text);
            let p = part(&h, "Row 1").frame.center();
            let baseline = part(&h, "Row 1").frame.origin.y;
            scroll(&mut h, p, -400., true, false, 1.);
            scroll(&mut h, p, 0., false, true, 1.02);
            render(&mut h, &mut text);
            assert!(
                (part(&h, "Row 1").frame.origin.y - baseline).abs() > 0.1,
                "original menu visibly bounces in mode {mode}"
            );
            text.reduced = reduced;
            for step in 1..=120 {
                text.now = std::time::Duration::from_millis(200 + step * 16);
                render(&mut h, &mut text);
            }
            assert!(
                (part(&h, "Row 1").frame.origin.y - baseline).abs() < 0.01,
                "ContentHost must settle original return: mode={mode}, reduced={reduced}"
            );
            let requests = text.requests;
            text.now += std::time::Duration::from_millis(16);
            render(&mut h, &mut text);
            assert_eq!(text.requests, requests, "settled menus stop scheduling");
            assert!(h.drain_actions().is_empty());
        }
    }
}
