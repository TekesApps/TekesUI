use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, collections_view::ListView, composition::*,
    extended::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new(
            (s.len() as f32 * 8.).min(w),
            if s.is_empty() {
                0.
            } else {
                20. + 20. * s.matches('\n').count() as f32
            },
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
fn snapshot() -> KeyedListSnapshot {
    KeyedListSnapshot {
        dataset: ListDatasetKey(1),
        items: (1..=3)
            .map(|k| KeyedListItem {
                key: ListItemKey(k),
                name: format!("row{k}"),
            })
            .collect(),
        display_items: (1..=3).map(ListItemKey).collect(),
    }
}
fn rows(s: &KeyedListSnapshot) -> Vec<ListRowDisplay> {
    s.items
        .iter()
        .map(|r| ListRowDisplay {
            key: r.key,
            row_id: ComponentId(100 + r.key.0),
            display: ListDisplayUpdate::Keep,
        })
        .collect()
}
fn budget() -> ListBudget {
    ListBudget {
        max_items: 100,
        max_display_items: 100,
        max_utf8_bytes: 1000000,
        max_staging_units: 100000,
    }
}
fn list(s: &KeyedListSnapshot) -> KeyedSelectableListContent {
    SelectableListContent::keyed("L", s, &mut rows(s), budget()).unwrap()
}
fn host(reading: bool) -> ContentHost {
    let s = snapshot();
    let mut h = ContentHost::new(Content::new(
        ComponentId(1),
        ScrollContent::new(
            tekes_ui_controls::ScrollArea::default(),
            Content::new(ComponentId(10), list(&s)),
        ),
    ))
    .unwrap();
    if reading {
        h.set_scroll_reading_document(
            ComponentId(1),
            "rows",
            1,
            s.display_items
                .iter()
                .map(|k| ComponentId(100 + k.0))
                .collect(),
            |_| (),
        )
        .unwrap();
    }
    paint(&mut h);
    h.set_action_events(true);
    h
}
fn paint(h: &mut ContentHost) {
    let f = Rect::new(0., 0., 200., 150.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
}
fn current(h: &ContentHost) -> &KeyedSelectableListContent {
    h.get(ComponentId(10))
        .unwrap()
        .component::<KeyedSelectableListContent>()
        .unwrap()
}
fn parts(h: &ContentHost) -> Vec<SemanticPart> {
    current(h).semantic_parts()
}
fn update(h: &mut ContentHost, s: &KeyedListSnapshot) -> Result<(), ListContentError> {
    let v = h
        .get(ComponentId(1))
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .reading_document_version()
        .cloned();
    let reading = v.map(|expected| ListReadingUpdate {
        owner: ComponentId(1),
        revision: expected.revision() + 1,
        expected,
        items: s
            .display_items
            .iter()
            .map(|k| ComponentId(100 + k.0))
            .collect(),
    });
    h.replace_keyed_list(ComponentId(10), s, &mut rows(s), reading.as_ref())?;
    Ok(())
}
#[test]
fn original_default_frames_scene_and_intrinsic_height_match_public_legacy_view() {
    let mut comparisons = 0;
    for width in [0., 1., 10., 40., 90., 200., 500., 1000.] {
        for count in [0, 1, 3] {
            for theme in [Theme::LIGHT, Theme::DARK] {
                for clip in [Rect::new(0., 0., 300., 400.), Rect::new(0., 10., 300., 20.)] {
                    let mut s = snapshot();
                    s.items.truncate(count);
                    s.display_items.truncate(count);
                    if count > 1 {
                        s.items[1].name = "long\nsecond line".into();
                    }
                    let labels = s.items.iter().map(|i| i.name.clone()).collect::<Vec<_>>();
                    let mut legacy = ListView::new();
                    let mut c = list(&s);
                    c.lifecycle(Lifecycle::Mounted);
                    let f = Rect::new(0., 0., width, 300.);
                    let measured = c.measure(&mut Text, f.size).size;
                    assert_eq!(measured, ListView::measure(&mut Text, &labels, width));
                    let mut a = Scene::new();
                    let l = legacy.render(
                        &mut a,
                        &mut Text,
                        &List::new(count),
                        &labels,
                        f,
                        Some(clip),
                        theme.into(),
                    );
                    c.arrange(&mut Text, f, clip);
                    let before = c.semantic_parts();
                    let mut b = Scene::new();
                    c.paint(
                        &mut b,
                        &mut Text,
                        f,
                        clip,
                        ContentTheme::from(tekes_ui_tokens::Palette::from(theme)),
                    );
                    assert_eq!(
                        format!("{a:?}"),
                        format!("{b:?}"),
                        "width={width},count={count}"
                    );
                    assert_eq!(
                        c.semantic_parts()
                            .iter()
                            .map(|p| p.frame)
                            .collect::<Vec<_>>(),
                        l.items.iter().filter_map(|r| r.visible).collect::<Vec<_>>()
                    );
                    assert_eq!(c.semantic_parts(), before);
                    comparisons += 1;
                }
            }
        }
    }
    assert_eq!(comparisons, 96);
}
#[test]
fn stale_geometry_and_held_up_never_redirect_to_new_source_index() {
    let mut h = host(false);
    h.event(&ContentEvent::PointerDown(Point::new(5., 5.)));
    let mut s = snapshot();
    s.items.remove(0);
    s.display_items.remove(0);
    update(&mut h, &s).unwrap();
    assert_eq!(current(&h).selected_key(), None);
    let before = format!("{h:?}");
    assert!(parts(&h).is_empty());
    assert_eq!(format!("{h:?}"), before);
    h.event(&ContentEvent::PointerUp(Point::new(5., 5.)));
    assert_eq!(current(&h).selected_key(), None);
    paint(&mut h);
    assert_eq!(current(&h).selected_key(), None);
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn display_navigation_empty_display_and_secondary_pointer_keep_original_semantics() {
    let mut s = snapshot();
    s.display_items = vec![ListItemKey(3), ListItemKey(1)];
    let mut c = list(&s);
    c.lifecycle(Lifecycle::Mounted);
    let f = Rect::new(0., 0., 200., 300.);
    c.arrange(&mut Text, f, f);
    c.set_focus(true, true);
    c.event(&ContentEvent::Key {
        key: ControlKey::Down,
        down: true,
    });
    assert_eq!(c.selected_key(), Some(ListItemKey(1)));
    c.event(&ContentEvent::Key {
        key: ControlKey::Home,
        down: true,
    });
    assert_eq!(c.selected_key(), Some(ListItemKey(3)));
    let pointer = PointerEvent {
        position: Point::new(5., 50.),
        phase: PointerPhase::Down,
        button: Some(PointerButton::Secondary),
        modifiers: Default::default(),
        click_count: 1,
    };
    assert!(!c.pointer_event(&pointer).consumed);
    assert_eq!(c.selected_key(), Some(ListItemKey(3)));
    let mut empty = snapshot();
    empty.display_items.clear();
    let mut c = list(&empty);
    c.lifecycle(Lifecycle::Mounted);
    c.arrange(&mut Text, f, f);
    assert!(!c.focusable());
    assert!(c.semantic_parts().is_empty());
    c.set_focus(true, true);
    assert!(
        !c.event(&ContentEvent::Key {
            key: ControlKey::Down,
            down: true
        })
        .consumed
    );
    assert_eq!(c.model().original().count, 3);
}
#[test]
fn dataset_aba_hidden_reappearance_disable_and_remount_reject_old_intents() {
    for case in 0..4 {
        let mut h = host(false);
        let p = parts(&h)[0].index;
        h.accessibility_action(ComponentId(10), Some(p), AccessibilityAction::Press);
        let records = h.drain_action_records();
        let Some(ContentActionPayload::ListIntent(i)) = &records[0].payload else {
            panic!()
        };
        let mut s = snapshot();
        match case {
            0 => {
                s.dataset = ListDatasetKey(2);
                update(&mut h, &s).unwrap();
                s.dataset = ListDatasetKey(1);
                update(&mut h, &s).unwrap();
            }
            1 => {
                s.display_items.remove(0);
                update(&mut h, &s).unwrap();
                update(&mut h, &snapshot()).unwrap();
            }
            2 => {
                h.get_mut(ComponentId(10))
                    .unwrap()
                    .component_mut::<KeyedSelectableListContent>()
                    .unwrap()
                    .set_disabled(true)
                    .unwrap();
                h.get_mut(ComponentId(10))
                    .unwrap()
                    .component_mut::<KeyedSelectableListContent>()
                    .unwrap()
                    .set_disabled(false)
                    .unwrap();
            }
            _ => {
                let c = h
                    .get_mut(ComponentId(10))
                    .unwrap()
                    .component_mut::<KeyedSelectableListContent>()
                    .unwrap();
                c.lifecycle(Lifecycle::Unmounted);
                c.lifecycle(Lifecycle::Mounted);
            }
        }
        paint(&mut h);
        assert!(!current(&h).intent_is_current(i));
        assert!(
            !h.accessibility_action(ComponentId(10), Some(p), AccessibilityAction::Press)
                .consumed
        );
    }
}
#[test]
fn caller_wrapper_reveal_is_original_and_hidden_deleted_anchor_never_redirects() {
    let mut h = host(true);
    let mut s = snapshot();
    for k in 4..30 {
        s.items.push(KeyedListItem {
            key: ListItemKey(k),
            name: format!("r{k}"),
        });
        s.display_items.push(ListItemKey(k));
    }
    update(&mut h, &s).unwrap();
    paint(&mut h);
    let version = h
        .get(ComponentId(1))
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .reading_document_version()
        .unwrap()
        .clone();
    let options = tekes_ui_controls::ScrollRevealOptions {
        animated: false,
        ..Default::default()
    };
    let result = h
        .reveal_scroll_reading_item(
            ComponentId(1),
            &version,
            ComponentId(129),
            options,
            &mut Text,
        )
        .unwrap();
    assert_eq!(result.item, ComponentId(129));
    assert!(
        h.get(ComponentId(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y
            > 0.
    );
    s.display_items.retain(|k| *k != ListItemKey(29));
    update(&mut h, &s).unwrap();
    paint(&mut h);
    let current = h
        .get(ComponentId(1))
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .reading_document_version()
        .unwrap()
        .clone();
    let before = format!("{h:?}");
    assert!(
        h.reveal_scroll_reading_item(
            ComponentId(1),
            &current,
            ComponentId(129),
            options,
            &mut Text
        )
        .is_err()
    );
    assert_eq!(format!("{h:?}"), before);
    assert!(
        h.reveal_scroll_reading_item(
            ComponentId(1),
            &version,
            ComponentId(128),
            options,
            &mut Text
        )
        .is_err()
    );
}

#[derive(Debug)]
struct Gate {
    active: std::rc::Rc<std::cell::Cell<bool>>,
    children: Vec<Content>,
}
impl Component for Gate {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn active(&self) -> bool {
        self.active.get()
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.children[0].measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.children[0].arrange(t, f, c);
    }
}
#[test]
fn ancestor_activity_revokes_exposure_without_replacing_root_mount_or_restoring_focus() {
    let active = std::rc::Rc::new(std::cell::Cell::new(true));
    let mut h = ContentHost::new(Content::new(
        ComponentId(1),
        Gate {
            active: active.clone(),
            children: vec![Content::new(ComponentId(10), list(&snapshot()))],
        },
    ))
    .unwrap();
    paint(&mut h);
    h.set_action_events(true);
    let old = parts(&h)[0].index;
    let mount = h.semantic_mount(ComponentId(10));
    assert!(
        h.accessibility_action(ComponentId(10), Some(old), AccessibilityAction::Press)
            .consumed
    );
    active.set(false);
    paint(&mut h);
    assert!(h.drain_action_records().is_empty());
    active.set(true);
    paint(&mut h);
    paint(&mut h);
    assert_eq!(h.semantic_mount(ComponentId(10)), mount);
    assert!(
        !h.accessibility_action(ComponentId(10), Some(old), AccessibilityAction::Press)
            .consumed
    );
    assert!(parts(&h).iter().all(|p| !p.focused));
    assert!(parts(&h)[0].index > old);
}
#[derive(Debug)]
struct DynamicPassive {
    interactive: std::rc::Rc<std::cell::Cell<bool>>,
}
impl Component for DynamicPassive {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn interactive(&self) -> bool {
        self.interactive.get()
    }
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        _: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(20., 20.))
    }
}
#[test]
fn lawful_custom_passive_dynamic_capability_is_rechecked_without_publishing_illegal_input() {
    let flag = std::rc::Rc::new(std::cell::Cell::new(false));
    let s = snapshot();
    let mut rows = rows(&s);
    rows[0].display = ListDisplayUpdate::Replace(Content::new(
        ComponentId(200),
        DynamicPassive {
            interactive: flag.clone(),
        },
    ));
    let c = KeyedSelectableListContent::new("custom", &s, &mut rows, budget()).unwrap();
    let mut h = ContentHost::new(Content::new(ComponentId(10), c)).unwrap();
    paint(&mut h);
    assert_eq!(parts(&h).len(), 3);
    flag.set(true);
    let before = format!("{h:?}");
    assert!(parts(&h).is_empty());
    assert!(!current(&h).focusable());
    assert_eq!(format!("{h:?}"), before);
    let mut rows = self::rows(&s);
    assert_eq!(
        h.replace_keyed_list(ComponentId(10), &s, &mut rows, None),
        Err(ListContentError::InteractiveDisplay {
            key: ListItemKey(1),
            child: ComponentId(200)
        })
    );
    assert_eq!(format!("{h:?}"), before);
}
#[test]
fn complete_host_other_branch_id_collision_is_refused_before_model_or_caller_changes() {
    let mut h = host(false);
    h.insert(
        ComponentId(1),
        Content::new(ComponentId(900), TextContent::new("other branch")),
    )
    .unwrap();
    let s = snapshot();
    let mut rows = rows(&s);
    rows[0].display = ListDisplayUpdate::Replace(Content::new(
        ComponentId(900),
        TextContent::new("collision"),
    ));
    let before = format!("{h:?}");
    let input = format!("{rows:?}");
    assert_eq!(
        h.replace_keyed_list(ComponentId(10), &s, &mut rows, None),
        Err(ListContentError::Composition(
            CompositionError::DuplicateId(ComponentId(900))
        ))
    );
    assert_eq!(format!("{h:?}"), before);
    assert_eq!(format!("{rows:?}"), input);
}

#[derive(Default)]
struct Recording {
    ink: Vec<(String, Rgba)>,
}
impl ButtonTextRenderer for Recording {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Text.measure(s, r, w)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        s: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        c: Rgba,
    ) {
        self.ink.push((s.into(), c));
    }
}
#[test]
fn passive_display_selected_disabled_ink_uses_original_surface_owner_and_clear() {
    let s = snapshot();
    let mut d = rows(&s);
    d[0].display =
        ListDisplayUpdate::Replace(Content::new(ComponentId(200), TextContent::new("custom")));
    let c = KeyedSelectableListContent::new("L", &s, &mut d, budget()).unwrap();
    let mut h = ContentHost::new(Content::new(ComponentId(10), c)).unwrap();
    paint(&mut h);
    let p = parts(&h)[0].index;
    h.accessibility_action(ComponentId(10), Some(p), AccessibilityAction::Press);
    for surface in [Some("canvas"), Some("panel"), None] {
        h.get_mut(ComponentId(10))
            .unwrap()
            .set_surface(surface)
            .unwrap();
        for disabled in [false, true, false] {
            h.get_mut(ComponentId(10))
                .unwrap()
                .component_mut::<KeyedSelectableListContent>()
                .unwrap()
                .set_disabled(disabled)
                .unwrap();
            let mut text = Recording::default();
            let f = Rect::new(0., 0., 200., 150.);
            h.render(&mut Scene::new(), &mut text, f, f, Theme::DARK.into())
                .unwrap();
            let color = tekes_ui_tokens::surface_palette::resolve_surface(
                Theme::DARK.into(),
                surface.unwrap_or("canvas"),
                "action-quiet",
                if disabled {
                    "selected-disabled"
                } else {
                    "selected"
                },
            )
            .unwrap()
            .text;
            assert!(text.ink.iter().any(|(s, c)| s == "custom" && *c == color));
        }
    }
}
#[test]
fn inactive_window_repaints_original_list_chrome_but_does_not_restore_part_authority() {
    let mut c = list(&snapshot());
    c.lifecycle(Lifecycle::Mounted);
    c.select_key(ListItemKey(2)).unwrap();
    c.window_active_changed(false);
    let f = Rect::new(0., 0., 200., 300.);
    c.arrange(&mut Text, f, f);
    let mut a = Scene::new();
    c.paint(
        &mut a,
        &mut Text,
        f,
        f,
        ContentTheme::from(tekes_ui_tokens::Palette::from(Theme::LIGHT)),
    );
    let mut v = ListView::new();
    v.set_window_active(false);
    let mut m = List::new(3);
    m.select(1);
    let mut b = Scene::new();
    v.render(
        &mut b,
        &mut Text,
        &m,
        &["row1".into(), "row2".into(), "row3".into()],
        f,
        Some(f),
        Theme::LIGHT.into(),
    );
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    assert!(c.semantic_parts().is_empty());
    assert!(
        !c.event(&ContentEvent::PointerDown(Point::new(5., 5.)))
            .consumed
    );
}
