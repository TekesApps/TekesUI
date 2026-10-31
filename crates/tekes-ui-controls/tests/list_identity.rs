use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey,
    collections_view::{CollectionEvent as E, ListView},
    extended::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
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
fn budget() -> ListBudget {
    ListBudget {
        max_items: 20,
        max_display_items: 20,
        max_utf8_bytes: 200,
        max_staging_units: 200,
    }
}
fn snapshot() -> KeyedListSnapshot {
    KeyedListSnapshot {
        dataset: ListDatasetKey(1),
        items: (1..=5)
            .map(|i| KeyedListItem {
                key: ListItemKey(i),
                name: "same".into(),
            })
            .collect(),
        display_items: vec![ListItemKey(5), ListItemKey(1), ListItemKey(3)],
    }
}
fn model() -> KeyedList {
    KeyedList::new(snapshot(), budget()).unwrap()
}
fn render(v: &mut ListView, m: &KeyedList) {
    v.render_keyed(
        &mut Scene::new(),
        &mut Text,
        m,
        Rect::new(0., 0., 200., 40.),
        None,
        Theme::LIGHT.into(),
    );
}
fn p() -> Point {
    Point::new(5., 5.)
}
#[test]
fn r_v_source_index_and_ordinal_are_distinct_navigation_ignores_clip() {
    let mut m = model();
    let mut v = ListView::new();
    render(&mut v, &m);
    assert_eq!(m.original().count, 5);
    assert_eq!(m.display_ordinal(ListItemKey(5)), Some(0));
    assert_eq!(m.item_index(ListItemKey(5)), Some(4));
    let rows = &v.layout_keyed(&m).unwrap().items;
    assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![4, 0, 2]);
    assert!(rows[2].visible.is_none());
    assert!(v.focus_keyed(true, &m));
    for (key, index) in [
        (ControlKey::Down, 0),
        (ControlKey::End, 2),
        (ControlKey::Up, 0),
        (ControlKey::Home, 4),
    ] {
        assert_eq!(v.key_keyed(key, true, &mut m), E::Changed);
        assert_eq!(m.original().selected, Some(index));
    }
    assert_eq!(
        v.key_keyed(ControlKey::Enter, true, &mut m),
        E::Activated(4)
    );
    assert_eq!(v.key_keyed(ControlKey::Space, true, &mut m), E::Handled);
    assert_eq!(v.key_keyed(ControlKey::Down, false, &mut m), E::Ignored);
}
#[test]
fn same_name_different_keys_reorder_rename_keep_entity_and_original_selection() {
    let mut m = model();
    let mut v = ListView::new();
    render(&mut v, &m);
    m.select_key(ListItemKey(5));
    v.focus_keyed(true, &m);
    let mut s = snapshot();
    s.items.reverse();
    s.items[0].name = "renamed".into();
    s.display_items.reverse();
    let r = m.replace_keyed(s).unwrap();
    assert_eq!(
        r.old_items_to_new,
        vec![Some(4), Some(3), Some(2), Some(1), Some(0)]
    );
    assert_eq!(m.selected_key(), Some(ListItemKey(5)));
    assert_eq!(m.original().selected, Some(0));
    assert!(v.layout_keyed(&m).is_none());
    assert_eq!(v.focused_keyed(&m), None);
    render(&mut v, &m);
    assert_eq!(v.focused_keyed(&m), Some(ListItemKey(5)));
    assert!(m.clear_selection());
    assert!(!m.clear_selection());
    assert!(v.layout_keyed(&m).is_some());
}
#[test]
fn replacement_revokes_capture_and_old_geometry_without_manual_reconcile() {
    let mut m = model();
    let mut v = ListView::new();
    render(&mut v, &m);
    assert_eq!(v.pointer_down_keyed(p(), &mut m), E::Changed);
    let mut s = snapshot();
    s.items.reverse();
    s.display_items.reverse();
    m.replace_keyed(s).unwrap();
    // Pure queries must not silently repair/reconcile any private state.
    let before = format!("{v:?} {m:?}");
    for _ in 0..3 {
        assert!(v.layout_keyed(&m).is_none());
        assert_eq!(v.focused_keyed(&m), None);
    }
    assert_eq!(format!("{v:?} {m:?}"), before);
    assert_eq!(v.pointer_up_keyed(p(), &mut m), E::Ignored);
    assert_eq!(v.pointer_down_keyed(p(), &mut m), E::Ignored);
    assert_eq!(v.key_keyed(ControlKey::Enter, true, &mut m), E::Ignored);
    assert!(!v.focus_keyed(true, &m));
    render(&mut v, &m);
    assert_eq!(v.pointer_up_keyed(p(), &mut m), E::Ignored);
    assert_eq!(m.selected_key(), Some(ListItemKey(5)));
}
#[test]
fn hidden_removed_and_dataset_aba_between_view_calls_cannot_restore_focus_or_selection() {
    for mode in 0..3 {
        let mut m = model();
        let mut v = ListView::new();
        render(&mut v, &m);
        v.pointer_down_keyed(p(), &mut m);
        let mut s = snapshot();
        match mode {
            0 => s.display_items.retain(|k| *k != ListItemKey(5)),
            1 => {
                s.items.pop();
                s.display_items.remove(0);
            }
            _ => s.dataset = ListDatasetKey(2),
        }
        m.replace_keyed(s).unwrap();
        m.replace_keyed(snapshot()).unwrap();
        assert_eq!(m.selected_key(), None);
        render(&mut v, &m);
        assert_eq!(v.focused_keyed(&m), None);
        assert_eq!(v.pointer_up_keyed(p(), &mut m), E::Ignored);
    }
}
#[test]
fn continuous_multi_replace_retains_focus_but_clone_and_entire_new_wrapper_reject() {
    let mut m = model();
    let mut v = ListView::new();
    render(&mut v, &m);
    v.focus_keyed(true, &m);
    for _ in 0..3 {
        let mut s = m.snapshot().clone();
        s.items.reverse();
        m.replace_keyed(s).unwrap();
    }
    render(&mut v, &m);
    assert_eq!(v.focused_keyed(&m), Some(ListItemKey(5)));
    let mut clone = m.clone();
    assert!(v.layout_keyed(&clone).is_none());
    assert_eq!(v.key_keyed(ControlKey::Enter, true, &mut clone), E::Ignored);
    render(&mut v, &clone);
    assert_eq!(v.focused_keyed(&clone), None);
    v.focus_keyed(true, &clone);
    let mut replacement = model();
    assert_eq!(v.pointer_down_keyed(p(), &mut replacement), E::Ignored);
    render(&mut v, &replacement);
    assert_eq!(v.focused_keyed(&replacement), None);
}
#[test]
fn v_empty_r_nonempty_has_no_fake_target_and_restore_does_not_refocus() {
    let mut m = model();
    let mut v = ListView::new();
    render(&mut v, &m);
    v.pointer_down_keyed(p(), &mut m);
    let mut s = snapshot();
    s.display_items.clear();
    m.replace_keyed(s).unwrap();
    render(&mut v, &m);
    assert_eq!(m.original().count, 5);
    assert_eq!(m.selected_key(), None);
    assert!(v.layout_keyed(&m).unwrap().items.is_empty());
    assert!(!v.focus_keyed(true, &m));
    assert_eq!(v.key_keyed(ControlKey::Home, true, &mut m), E::Ignored);
    m.replace_keyed(snapshot()).unwrap();
    render(&mut v, &m);
    assert_eq!(v.focused_keyed(&m), None);
}
#[test]
fn legacy_entrypoints_cannot_authorize_keyed_input_and_legacy_render_revokes() {
    let mut m = model();
    let mut v = ListView::new();
    render(&mut v, &m);
    let mut legacy = List::new(5);
    v.focus(4, true);
    assert_eq!(v.focused_keyed(&m), None);
    assert!(v.layout().items.is_empty());
    assert!(!v.pointer_move(p()));
    assert_eq!(v.pointer_down(p(), &mut legacy), E::Ignored);
    assert_eq!(v.pointer_up(p(), &mut legacy), E::Ignored);
    assert_eq!(v.key(ControlKey::Enter, true, &mut legacy), E::Ignored);
    v.render(
        &mut Scene::new(),
        &mut Text,
        &legacy,
        &vec!["old".into(); 5],
        Rect::new(0., 0., 200., 200.),
        None,
        Theme::LIGHT.into(),
    );
    assert!(v.layout_keyed(&m).is_none());
    assert_eq!(v.pointer_down_keyed(p(), &mut m), E::Ignored);
    render(&mut v, &m);
    assert_eq!(v.focused_keyed(&m), None);
}
#[test]
fn disabled_inactive_cancel_and_pointer_up_preserve_established_selection() {
    let mut m = model();
    let mut v = ListView::new();
    render(&mut v, &m);
    assert_eq!(v.pointer_down_keyed(p(), &mut m), E::Changed);
    assert_eq!(v.pointer_up_keyed(p(), &mut m), E::Handled);
    v.pointer_down_keyed(p(), &mut m);
    v.cancel();
    assert_eq!(v.pointer_up_keyed(p(), &mut m), E::Ignored);
    v.set_disabled(true);
    assert!(!v.focus_keyed(true, &m));
    assert_eq!(v.focused_keyed(&m), None);
    assert_eq!(m.selected_key(), Some(ListItemKey(5)));
    v.set_disabled(false);
    assert_eq!(v.focused_keyed(&m), None);
    v.focus_keyed(true, &m);
    v.set_window_active(false);
    v.set_window_active(true);
    assert_eq!(v.focused_keyed(&m), None);
    assert_eq!(v.pointer_up_keyed(p(), &mut m), E::Ignored);
}
#[test]
fn invalid_targets_leave_model_and_live_view_capture_exactly_unchanged() {
    let mut bad = Vec::new();
    let mut s = snapshot();
    s.items[0].key = s.items[1].key;
    bad.push(s);
    let mut s = snapshot();
    s.display_items.push(ListItemKey(5));
    bad.push(s);
    let mut s = snapshot();
    s.display_items.push(ListItemKey(999));
    bad.push(s);
    let mut s = snapshot();
    s.items[0].name = "x".repeat(201);
    bad.push(s);
    let mut s = snapshot();
    s.items.extend((6..=21).map(|i| KeyedListItem {
        key: ListItemKey(i),
        name: String::new(),
    }));
    bad.push(s);
    for s in bad {
        let mut m = model();
        let mut v = ListView::new();
        render(&mut v, &m);
        v.pointer_down_keyed(p(), &mut m);
        let before = format!("{m:?} {v:?}");
        assert!(m.replace_keyed(s).is_err());
        assert_eq!(format!("{m:?} {v:?}"), before);
        assert_eq!(v.pointer_up_keyed(p(), &mut m), E::Handled);
    }
    let mut b = budget();
    b.max_staging_units = 20;
    let mut m = KeyedList::new(snapshot(), b).unwrap();
    let before = format!("{m:?}");
    assert_eq!(
        m.replace_keyed(snapshot()),
        Err(ListUpdateError::BudgetExceeded(
            ListBudgetKind::StagingUnits
        ))
    );
    assert_eq!(format!("{m:?}"), before);
    let mut b = budget();
    b.max_display_items = 2;
    assert!(matches!(
        KeyedList::new(snapshot(), b),
        Err(ListUpdateError::BudgetExceeded(
            ListBudgetKind::DisplayItems
        ))
    ));
    let mut b = budget();
    b.max_items = 0;
    assert!(matches!(
        KeyedList::new(snapshot(), b),
        Err(ListUpdateError::InvalidBudget)
    ));
}
#[test]
fn utf8_budget_counts_bytes_and_empty_snapshot_is_valid() {
    let mut s = snapshot();
    s.items[0].name = "汉".repeat(62);
    assert!(matches!(
        KeyedList::new(s, budget()),
        Err(ListUpdateError::BudgetExceeded(ListBudgetKind::Utf8Bytes))
    ));
    let m = KeyedList::new(
        KeyedListSnapshot {
            dataset: ListDatasetKey(1),
            items: vec![],
            display_items: vec![],
        },
        budget(),
    )
    .unwrap();
    let mut v = ListView::new();
    render(&mut v, &m);
    assert!(!v.focus_keyed(true, &m));
}
#[test]
fn default_geometry_and_scene_commands_match_original_for_theme_clip_scale_and_state() {
    for theme in [
        Theme::LIGHT,
        Theme::DARK,
        Theme::LIGHT_INCREASED,
        Theme::DARK_INCREASED,
    ] {
        for scale in [1., 1.5] {
            for width in [200., 1., 0., f32::NAN] {
                for clip in [
                    None,
                    Some(Rect::new(0., 4., 150., 70.)),
                    Some(Rect::new(500., 500., 10., 10.)),
                ] {
                    let mut s = snapshot();
                    s.display_items = s.items.iter().map(|r| r.key).collect();
                    let mut m = KeyedList::new(s, budget()).unwrap();
                    let mut old = List::new(5);
                    let labels = vec!["same".into(); 5];
                    let frame = Rect::new(0., 0., width, 120.);
                    let mut a = ListView::new();
                    let mut b = ListView::new();
                    a.render_scaled(
                        &mut Scene::new(),
                        &mut Text,
                        &old,
                        &labels,
                        frame,
                        clip,
                        theme.into(),
                        scale,
                    );
                    b.render_keyed_scaled(
                        &mut Scene::new(),
                        &mut Text,
                        &m,
                        frame,
                        clip,
                        theme.into(),
                        scale,
                    );
                    a.focus(0, true);
                    b.focus_item_keyed(ListItemKey(1), true, &m);
                    old.select(0);
                    m.select_key(ListItemKey(1));
                    let mut sa = Scene::new();
                    let mut sb = Scene::new();
                    let la = a.render_scaled(
                        &mut sa,
                        &mut Text,
                        &old,
                        &labels,
                        frame,
                        clip,
                        theme.into(),
                        scale,
                    );
                    let lb = b.render_keyed_scaled(
                        &mut sb,
                        &mut Text,
                        &m,
                        frame,
                        clip,
                        theme.into(),
                        scale,
                    );
                    assert_eq!(format!("{la:?}"), format!("{lb:?}"));
                    assert_eq!(format!("{sa:?}"), format!("{sb:?}"));
                }
            }
        }
    }
}

#[test]
fn removed_source_zero_reoccupied_by_new_entity_cannot_receive_old_release() {
    let mut s = snapshot();
    s.display_items = vec![ListItemKey(1), ListItemKey(3)];
    let mut m = KeyedList::new(s.clone(), budget()).unwrap();
    let mut v = ListView::new();
    render(&mut v, &m);
    assert_eq!(v.pointer_down_keyed(p(), &mut m), E::Changed);
    s.items[0] = KeyedListItem {
        key: ListItemKey(99),
        name: "same".into(),
    };
    s.display_items[0] = ListItemKey(99);
    m.replace_keyed(s).unwrap();
    assert_eq!(m.selected_key(), None);
    render(&mut v, &m);
    assert_eq!(v.focused_keyed(&m), None);
    assert_eq!(v.pointer_up_keyed(p(), &mut m), E::Ignored);
    assert_eq!(m.selected_key(), None);
}
