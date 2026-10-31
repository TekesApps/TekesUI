use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey,
    collections_view::{CollectionEvent as E, TreeView},
    extended::{KeyedTreeNode, Tree, TreeDatasetKey as D, TreeNodeKey as K, TreeUpdateError},
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, _: &str, _: proposal::TextRole, width: f32) -> Size {
        Size::new(width.min(100.), 20.)
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
fn node(key: u64, parent: Option<u64>) -> KeyedTreeNode {
    KeyedTreeNode {
        key: K(key),
        parent: parent.map(K),
        label: "same label".into(),
        disabled: false,
        initially_expanded: true,
    }
}
fn model() -> Tree {
    let mut m = Tree::default();
    m.replace_keyed(D(1), vec![node(1, None), node(2, Some(1)), node(3, None)])
        .unwrap();
    m
}
fn paint(v: &mut TreeView, m: &mut Tree) {
    v.render_model(
        &mut Scene::new(),
        &mut Text,
        m,
        Rect::new(0., 0., 320., 300.),
        None,
        Theme::LIGHT.into(),
    );
}
fn point(v: &TreeView, m: &Tree, id: usize) -> Point {
    v.layout_for(m)
        .unwrap()
        .items
        .iter()
        .find(|r| r.id == id)
        .unwrap()
        .frame
        .center()
}
#[test]
fn atomic_validation_and_legacy_compatibility() {
    let mut m = model();
    m.select_key(K(2));
    let before = m.clone();
    for (bad, expected) in [
        (
            vec![node(1, None), node(1, None)],
            TreeUpdateError::DuplicateKey(K(1)),
        ),
        (vec![node(1, Some(1))], TreeUpdateError::InvalidParent(K(1))),
        (vec![node(2, Some(9))], TreeUpdateError::InvalidParent(K(2))),
        (
            vec![node(1, Some(2)), node(2, Some(1))],
            TreeUpdateError::InvalidParent(K(1)),
        ),
        (
            vec![node(1, None), node(2, None), node(3, Some(1))],
            TreeUpdateError::NonPreorder(K(3)),
        ),
    ] {
        assert_eq!(m.replace_keyed(D(1), bad), Err(expected));
        assert_eq!(m, before);
    }
    assert_eq!(m.add("mixed", None), None);
    assert_eq!(m, before);
    let mut legacy = Tree::default();
    let r = legacy.add("root", None).unwrap();
    let c = legacy.add("child", Some(r)).unwrap();
    legacy.selected = Some(r);
    assert!(legacy.key(ControlKey::Right));
    assert!(legacy.key(ControlKey::Down));
    assert_eq!(legacy.selected, Some(c));
}
#[test]
fn reorder_requires_layout_but_preserves_entity_selection_expansion_and_focus() {
    let mut m = model();
    let mut v = TreeView::new();
    paint(&mut v, &mut m);
    m.select_key(K(2));
    assert!(v.focus_model(1, true, &mut m));
    m.replace_keyed(D(1), vec![node(3, None), node(1, None), node(2, Some(1))])
        .unwrap();
    assert!(v.layout_for(&m).is_none());
    assert_eq!(v.focused_index(&m), None);
    assert_eq!(v.key(ControlKey::Enter, true, &mut m), E::Ignored);
    paint(&mut v, &mut m);
    assert_eq!(m.selected, Some(2));
    assert_eq!(v.focused_index(&m), Some(2));
    assert!(m.nodes()[1].expanded);
    assert_eq!(v.key(ControlKey::Enter, true, &mut m), E::Activated(2));
}
#[test]
fn late_pointer_cannot_target_replacement_or_reordered_row() {
    for nodes in [
        vec![node(9, None)],
        vec![node(3, None), node(1, None), node(2, Some(1))],
    ] {
        let mut m = model();
        let mut v = TreeView::new();
        paint(&mut v, &mut m);
        let p = point(&v, &m, 0);
        assert_ne!(v.pointer_down(p, &mut m), E::Ignored);
        m.replace_keyed(D(1), nodes).unwrap();
        assert_eq!(v.pointer_up(p, &mut m), E::Ignored);
        paint(&mut v, &mut m);
        assert_eq!(v.pointer_up(p, &mut m), E::Ignored);
    }
}
#[test]
fn aba_without_intermediate_render_revokes_focus_and_capture() {
    for mode in 0..4 {
        let mut m = model();
        let mut v = TreeView::new();
        paint(&mut v, &mut m);
        let p = point(&v, &m, 1);
        v.pointer_down(p, &mut m);
        assert!(v.focus_model(1, true, &mut m));
        match mode {
            0 => {
                m.replace_keyed(D(1), vec![node(1, None), node(3, None)])
                    .unwrap();
                m.replace_keyed(D(1), vec![node(1, None), node(2, Some(1)), node(3, None)])
                    .unwrap();
            }
            1 => {
                m.toggle(0);
                m.toggle(0);
            }
            2 => {
                m.set_disabled(1, true);
                m.set_disabled(1, false);
            }
            _ => {
                m.replace_keyed(D(2), vec![node(1, None), node(2, Some(1)), node(3, None)])
                    .unwrap();
                m.replace_keyed(D(1), vec![node(1, None), node(2, Some(1)), node(3, None)])
                    .unwrap();
            }
        }
        assert_eq!(v.pointer_up(p, &mut m), E::Ignored);
        paint(&mut v, &mut m);
        assert_eq!(v.focused_index(&m), None);
    }
}
#[test]
fn cloned_and_replaced_models_have_no_view_authority() {
    let mut m = model();
    let mut v = TreeView::new();
    paint(&mut v, &mut m);
    let p = point(&v, &m, 0);
    v.pointer_down(p, &mut m);
    let mut clone = m.clone();
    assert!(v.layout_for(&clone).is_none());
    assert_eq!(v.pointer_up(p, &mut clone), E::Ignored);
    paint(&mut v, &mut clone);
    assert_eq!(v.focused_index(&clone), None);
}
#[test]
fn selected_projection_tampering_never_selects_another_entity() {
    let mut m = model();
    m.select_key(K(2));
    let mut v = TreeView::new();
    paint(&mut v, &mut m);
    v.focus_model(1, true, &mut m);
    m.selected = Some(2);
    let before = m.clone();
    let view_before = format!("{v:?}");
    for _ in 0..3 {
        assert!(v.layout_for(&m).is_none());
        assert_eq!(v.focused_index(&m), None);
        assert_eq!(m, before);
        assert_eq!(format!("{v:?}"), view_before);
    }
    assert_eq!(v.key(ControlKey::Enter, true, &mut m), E::Ignored);
    assert_eq!(m.selected_key(), Some(K(2)));
    assert_eq!(m.selected, Some(1));
    paint(&mut v, &mut m);
    assert_eq!(m.selected_key(), Some(K(2)));
    m.selected = Some(0);
    paint(&mut v, &mut m);
    assert_eq!(m.selected, Some(1));
}
#[test]
fn legacy_entries_never_authorize_keyed_focus_or_geometry() {
    let mut m = model();
    let mut v = TreeView::new();
    paint(&mut v, &mut m);
    assert!(v.layout().items.is_empty());
    v.focus(1, true);
    assert_eq!(v.focused_index(&m), None);
    assert!(!v.pointer_move(Point::new(20., 20.)));
}
#[test]
fn collapse_paths_restore_hidden_selection_and_focus_and_enter_activates() {
    for keyed in [false, true] {
        for path in 0..3 {
            let mut m = if keyed {
                model()
            } else {
                let mut m = Tree::default();
                m.add("r", None);
                m.add("c", Some(0));
                m.toggle(0);
                m
            };
            let mut v = TreeView::new();
            paint(&mut v, &mut m);
            m.select(1);
            v.focus_model(1, true, &mut m);
            let p = point(&v, &m, 0);
            match path {
                0 => {
                    assert_eq!(v.set_expanded(0, false, &mut m), E::Toggled(0));
                }
                1 => {
                    v.pointer_down(p, &mut m);
                    m.select(1);
                    v.focus_model(1, true, &mut m);
                    assert_eq!(v.pointer_up(p, &mut m), E::Toggled(0));
                }
                _ => {
                    v.focus_model(0, true, &mut m);
                    assert_eq!(v.key(ControlKey::Left, true, &mut m), E::Toggled(0));
                }
            }
            paint(&mut v, &mut m);
            assert_eq!(m.selected, Some(0));
            assert_eq!(v.focused_index(&m), Some(0));
            assert_eq!(v.key(ControlKey::Enter, true, &mut m), E::Activated(0));
            assert!(!m.nodes()[0].expanded);
        }
    }
}
#[test]
fn disabling_only_one_node_and_whole_view_does_not_revive_focus() {
    let mut m = model();
    let mut v = TreeView::new();
    paint(&mut v, &mut m);
    m.select(1);
    v.focus_model(1, true, &mut m);
    m.set_disabled(0, true);
    paint(&mut v, &mut m);
    assert_eq!(m.selected, Some(1));
    assert_eq!(v.focused_index(&m), Some(1));
    assert_eq!(v.set_expanded(0, false, &mut m), E::Ignored);
    m.set_disabled(1, true);
    paint(&mut v, &mut m);
    assert_eq!(m.selected, None);
    assert_eq!(v.focused_index(&m), None);
    m.set_disabled(1, false);
    paint(&mut v, &mut m);
    assert_eq!(v.focused_index(&m), None);
    v.focus_model(1, true, &mut m);
    v.set_disabled(true);
    v.set_disabled(false);
    assert_eq!(v.focused_index(&m), None);
    v.focus_model(1, true, &mut m);
    v.set_window_active(false);
    v.set_window_active(true);
    assert_eq!(v.focused_index(&m), None);
}
#[test]
fn legacy_and_keyed_share_identical_string_geometry() {
    let mut legacy = Tree::default();
    legacy.add("same label", None);
    legacy.add("same label", Some(0));
    legacy.add("same label", None);
    legacy.toggle(0);
    let mut keyed = model();
    let mut a = TreeView::new();
    let mut b = TreeView::new();
    paint(&mut a, &mut legacy);
    paint(&mut b, &mut keyed);
    assert_eq!(a.layout_for(&legacy), b.layout_for(&keyed));
}

#[test]
fn hidden_and_removed_selection_focus_recover_only_through_old_ancestors() {
    for remove in [false, true] {
        let mut m = model();
        let mut v = TreeView::new();
        paint(&mut v, &mut m);
        m.select_key(K(2));
        v.focus_model(1, true, &mut m);
        if remove {
            m.replace_keyed(D(1), vec![node(1, None), node(3, None)])
                .unwrap();
        } else {
            assert!(m.toggle(0));
        }
        paint(&mut v, &mut m);
        assert_eq!(m.selected_key(), Some(K(1)));
        assert_eq!(v.focused_index(&m), Some(0));
    }
}
#[test]
fn failed_snapshot_preserves_pending_original_pointer_gesture() {
    let mut m = model();
    let mut v = TreeView::new();
    paint(&mut v, &mut m);
    let p = point(&v, &m, 0);
    v.pointer_down(p, &mut m);
    let before = m.clone();
    let layout = v.layout_for(&m).cloned();
    assert!(
        m.replace_keyed(D(1), vec![node(1, None), node(1, None)])
            .is_err()
    );
    assert_eq!(m, before);
    assert_eq!(v.layout_for(&m), layout.as_ref());
    assert_eq!(v.pointer_up(p, &mut m), E::Toggled(0));
}
#[test]
fn stale_query_is_pure_and_readonly_render_cannot_authorize_tampered_projection() {
    let mut m = model();
    let mut v = TreeView::new();
    paint(&mut v, &mut m);
    let p = point(&v, &m, 0);
    v.pointer_down(p, &mut m);
    m.selected = Some(2);
    let before = m.clone();
    assert!(
        v.render(
            &mut Scene::new(),
            &mut Text,
            &m,
            Rect::new(0., 0., 320., 300.),
            None,
            Theme::LIGHT.into()
        )
        .items
        .is_empty()
    );
    assert_eq!(m, before);
    assert_eq!(v.pointer_up(p, &mut m), E::Ignored);
    paint(&mut v, &mut m);
    assert_eq!(m.selected_key(), Some(K(1)));
}

#[test]
fn original_pointer_collapse_must_restore_hidden_selection() {
    let mut m = Tree::default();
    m.add("root", None);
    m.add("child", Some(0));
    m.toggle(0);
    let mut v = TreeView::new();
    v.render(
        &mut Scene::new(),
        &mut Text,
        &m,
        Rect::new(0., 0., 320., 300.),
        None,
        Theme::LIGHT.into(),
    );
    let p = v.layout().items[0].frame.center();
    v.pointer_down(p, &mut m);
    m.select(1);
    v.focus(1, true);
    assert_eq!(v.pointer_up(p, &mut m), E::Toggled(0));
    assert_eq!(
        m.selected,
        Some(0),
        "pointer collapse must restore concealed selected descendant"
    );
}

#[test]
fn stale_model_aware_focus_disclosure_hover_and_icons_fail_closed() {
    struct Icons(usize);
    impl tekes_ui_controls::layout_view::IconRenderer for Icons {
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
            self.0 += 1;
        }
    }
    let mut m = model();
    let mut v = TreeView::new();
    paint(&mut v, &mut m);
    let p = point(&v, &m, 0);
    let mut icons = Icons(0);
    v.render_icons(
        &mut Scene::new(),
        &mut icons,
        None,
        Theme::LIGHT.into(),
        16.,
    );
    assert_eq!(icons.0, 0);
    v.render_icons_model(
        &mut Scene::new(),
        &mut icons,
        &mut m,
        None,
        Theme::LIGHT.into(),
        16.,
    );
    assert_eq!(icons.0, 1);
    m.replace_keyed(D(1), vec![node(3, None), node(1, None), node(2, Some(1))])
        .unwrap();
    assert!(!v.focus_model(0, true, &mut m));
    assert_eq!(v.set_expanded(1, false, &mut m), E::Ignored);
    assert!(!v.pointer_move_model(p, &mut m));
    v.render_icons_model(
        &mut Scene::new(),
        &mut icons,
        &mut m,
        None,
        Theme::LIGHT.into(),
        16.,
    );
    assert_eq!(icons.0, 1);
    paint(&mut v, &mut m);
    assert!(v.focus_model(1, true, &mut m));
    assert!(v.pointer_move_model(point(&v, &m, 1), &mut m));
}
#[test]
fn focused_keyed_and_legacy_paint_commands_and_clipped_geometry_match() {
    let mut legacy = Tree::default();
    legacy.add("same label", None);
    legacy.add("same label", Some(0));
    legacy.add("same label", None);
    legacy.toggle(0);
    let mut keyed = model();
    let mut a = TreeView::new();
    let mut b = TreeView::new();
    paint(&mut a, &mut legacy);
    paint(&mut b, &mut keyed);
    a.focus_model(1, true, &mut legacy);
    b.focus_model(1, true, &mut keyed);
    legacy.select(1);
    keyed.select(1);
    for clip in [
        None,
        Some(Rect::new(4., 4., 150., 50.)),
        Some(Rect::new(0., 1000., 1., 1.)),
    ] {
        let mut x = Scene::new();
        let mut y = Scene::new();
        let frame = Rect::new(0., 0., 90., 300.);
        let l = a.render_model(
            &mut x,
            &mut Text,
            &mut legacy,
            frame,
            clip,
            Theme::LIGHT.into(),
        );
        let r = b.render_model(
            &mut y,
            &mut Text,
            &mut keyed,
            frame,
            clip,
            Theme::LIGHT.into(),
        );
        assert_eq!(l, r);
        assert_eq!(
            format!("{:?}", x.primitives()),
            format!("{:?}", y.primitives())
        );
    }
}
