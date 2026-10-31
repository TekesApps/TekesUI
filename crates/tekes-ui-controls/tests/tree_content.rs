use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::*,
    extended::{Axis, KeyedTreeNode, Stack, TreeDatasetKey as D, TreeNodeKey as K},
    layout::MeasuredChild,
    layout_view::IconRenderer,
};
use tekes_ui_scene::{ImageId, Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
#[derive(Default)]
struct Icons;
impl IconRenderer for Icons {
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {}
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
#[derive(Default)]
struct Text {
    icons: Icons,
}
impl ButtonTextRenderer for Text {
    fn icon_renderer(&mut self) -> Option<&mut dyn IconRenderer> {
        Some(&mut self.icons)
    }
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), 20.)
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
fn n(key: u64, parent: Option<u64>) -> TreeContentNode {
    TreeContentNode {
        row_id: ComponentId(1000 + key),
        node: KeyedTreeNode {
            key: K(key),
            parent: parent.map(K),
            label: format!("node{key}"),
            disabled: false,
            initially_expanded: true,
        },
        display: TreeDisplayUpdate::Keep,
    }
}
fn tree() -> TreeContent {
    TreeContent::new("Tree", D(1), vec![n(1, None), n(2, Some(1)), n(3, None)]).unwrap()
}
fn host() -> ContentHost {
    let mut h = ContentHost::new(Content::new(ComponentId(10), tree())).unwrap();
    h.set_action_events(true);
    paint(&mut h);
    h
}
fn paint(h: &mut ContentHost) {
    paint_clip(h, Rect::new(0., 0., 320., 400.));
}
fn paint_clip(h: &mut ContentHost, c: Rect) {
    h.render(
        &mut Scene::new(),
        &mut Text::default(),
        Rect::new(0., 0., 320., 400.),
        c,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn parts(h: &ContentHost) -> Vec<SemanticPart> {
    h.semantics()
        .into_iter()
        .find(|n| n.id == ComponentId(10))
        .unwrap()
        .parts
}
fn t(h: &ContentHost) -> &TreeContent {
    h.get(ComponentId(10))
        .unwrap()
        .component::<TreeContent>()
        .unwrap()
}
fn point(h: &ContentHost, i: usize) -> Point {
    parts(h)[i].frame.center()
}
fn press(h: &mut ContentHost, i: usize) {
    let p = parts(h)[i].index;
    assert!(
        h.accessibility_action(ComponentId(10), Some(p), AccessibilityAction::Press)
            .activated
    );
}
fn intent(h: &mut ContentHost) -> TreeIntent {
    let r = h.drain_action_records();
    assert_eq!(r.len(), 1);
    match r[0].payload.clone().unwrap() {
        ContentActionPayload::TreeIntent(i) => i,
        _ => panic!("tree intent"),
    }
}
#[test]
fn host_roles_parent_and_readonly_queries_share_arranged_geometry() {
    let h = host();
    let p = parts(&h);
    assert_eq!(h.semantics()[0].semantics.role, ContentRole::Tree);
    assert_eq!(p[0].semantics.role, ContentRole::TreeItem { level: 0 });
    assert_eq!(p[1].semantics.role, ContentRole::TreeItem { level: 1 });
    assert_eq!(
        h.semantic_part_parent(ComponentId(10), p[1].index),
        Some(p[0].index)
    );
    for _ in 0..5 {
        assert_eq!(parts(&h), p);
        assert_eq!(t(&h).selected_key(), None);
        assert!(h.semantic_mount(ComponentId(10)).is_some());
    }
}
#[test]
fn reorder_preserves_tokens_and_valid_history_but_rejects_stale_geometry() {
    let mut h = host();
    press(&mut h, 1);
    let original = parts(&h)[1].index;
    h.replace_tree_nodes(
        ComponentId(10),
        D(1),
        vec![n(3, None), n(1, None), n(2, Some(1))],
    )
    .unwrap();
    assert!(parts(&h).is_empty());
    let i = intent(&mut h);
    assert_eq!(i.key, K(2));
    assert!(t(&h).intent_is_current(&i));
    assert!(
        !h.accessibility_action(ComponentId(10), Some(original), AccessibilityAction::Press)
            .consumed
    );
    paint(&mut h);
    assert_eq!(parts(&h)[2].index, original);
    assert_eq!(t(&h).selected_key(), Some(K(2)));
}
#[test]
fn hide_disable_remove_clear_and_dataset_aba_never_resurrect_exposure() {
    for mode in 0..5 {
        let mut h = host();
        press(&mut h, 1);
        let old = intent(&mut h);
        let p = parts(&h);
        press(&mut h, 1);
        match mode {
            0 => {
                assert!(
                    h.accessibility_action(
                        ComponentId(10),
                        Some(p[0].index),
                        AccessibilityAction::SetExpanded(false)
                    )
                    .changed
                );
                paint(&mut h);
                assert!(
                    h.accessibility_action(
                        ComponentId(10),
                        Some(parts(&h)[0].index),
                        AccessibilityAction::SetExpanded(true)
                    )
                    .changed
                );
            }
            1 => {
                let mut b = n(2, Some(1));
                b.node.disabled = true;
                h.replace_tree_nodes(ComponentId(10), D(1), vec![n(1, None), b, n(3, None)])
                    .unwrap();
            }
            2 => {
                h.replace_tree_nodes(ComponentId(10), D(1), vec![n(1, None), n(3, None)])
                    .unwrap();
            }
            3 => {
                h.replace_tree_nodes(ComponentId(10), D(1), vec![]).unwrap();
            }
            _ => {
                h.replace_tree_nodes(
                    ComponentId(10),
                    D(2),
                    vec![n(1, None), n(2, Some(1)), n(3, None)],
                )
                .unwrap();
            }
        }
        h.replace_tree_nodes(
            ComponentId(10),
            D(1),
            vec![n(1, None), n(2, Some(1)), n(3, None)],
        )
        .unwrap();
        paint(&mut h);
        assert_ne!(parts(&h)[1].index, old.exposure, "mode {mode}");
        assert!(!t(&h).intent_is_current(&old));
        assert!(
            !h.accessibility_action(
                ComponentId(10),
                Some(old.exposure),
                AccessibilityAction::Press
            )
            .consumed
        );
        assert!(h.drain_action_records().iter().all(
            |r| !matches!(&r.payload,Some(ContentActionPayload::TreeIntent(i)) if i.key==K(2))
        ));
    }
}
#[test]
fn inactive_epoch_and_cancel_have_different_queue_effects() {
    let mut h = host();
    press(&mut h, 1);
    h.set_window_active(false);
    h.set_window_active(false);
    h.set_window_active(true);
    assert!(h.drain_action_records().is_empty());
    paint(&mut h);
    assert!(parts(&h).iter().all(|p| !p.focused));
    let p = point(&h, 2);
    h.event(&ContentEvent::PointerDown(p));
    h.cancel_pointer_capture();
    let r = intent(&mut h);
    assert_eq!(r.kind, TreeIntentKind::SelectionChanged);
    assert!(t(&h).intent_is_current(&r));
    h.event(&ContentEvent::PointerUp(p));
    assert!(h.drain_action_records().is_empty());
}
#[derive(Debug)]
struct Display {
    mounted: Rc<Cell<u32>>,
    unmounted: Rc<Cell<u32>>,
}
impl Component for Display {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(100., 64.))
    }
    fn lifecycle(&mut self, e: Lifecycle) {
        let c = if e == Lifecycle::Mounted {
            &self.mounted
        } else {
            &self.unmounted
        };
        c.set(c.get() + 1);
    }
}
#[test]
fn keep_moves_original_content_and_failure_preserves_mount_selection_capture_queue() {
    let (mounted, unmounted) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let mut a = n(1, None);
    a.display = TreeDisplayUpdate::Replace(Content::new(
        ComponentId(77),
        Display {
            mounted: mounted.clone(),
            unmounted: unmounted.clone(),
        },
    ));
    let tr = TreeContent::new("T", D(1), vec![a, n(2, Some(1)), n(3, None)]).unwrap();
    let mut h = ContentHost::new(Content::new(ComponentId(10), tr)).unwrap();
    h.set_action_events(true);
    paint(&mut h);
    assert_eq!(mounted.get(), 1);
    let mount = h.semantic_mount(ComponentId(77));
    h.replace_tree_nodes(
        ComponentId(10),
        D(1),
        vec![n(3, None), n(1, None), n(2, Some(1))],
    )
    .unwrap();
    paint(&mut h);
    assert_eq!(h.semantic_mount(ComponentId(77)), mount);
    assert_eq!((mounted.get(), unmounted.get()), (1, 0));
    let p = point(&h, 1);
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    let before = parts(&h);
    let mut bad = n(1, None);
    bad.display = TreeDisplayUpdate::Replace(Content::new(ComponentId(80), Button::new("illegal")));
    assert!(matches!(
        h.replace_tree_nodes(ComponentId(10), D(1), vec![n(3, None), bad, n(2, Some(1))]),
        Err(TreeContentError::InteractiveDisplay { .. })
    ));
    assert_eq!(parts(&h), before);
    assert_eq!(h.semantic_mount(ComponentId(77)), mount);
    assert_eq!((mounted.get(), unmounted.get()), (1, 0));
    h.event(&ContentEvent::PointerUp(p));
    assert_eq!(h.drain_action_records().len(), 2);
}
#[test]
fn whole_host_duplicate_and_direct_mounted_id_changes_are_atomic() {
    let root = Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(ComponentId(10), tree()),
                Content::new(ComponentId(90), TextContent::new("neighbor")),
            ],
        },
    );
    let mut h = ContentHost::new(root).unwrap();
    paint(&mut h);
    let before = parts(&h);
    let mut a = n(1, None);
    a.display =
        TreeDisplayUpdate::Replace(Content::new(ComponentId(90), TextContent::new("duplicate")));
    assert!(matches!(
        h.replace_tree_nodes(ComponentId(10), D(1), vec![a]),
        Err(TreeContentError::Composition(
            CompositionError::DuplicateId(ComponentId(90))
        ))
    ));
    assert_eq!(parts(&h), before);
    let mut a = n(1, None);
    a.display =
        TreeDisplayUpdate::Replace(Content::new(ComponentId(91), TextContent::new("new ID")));
    assert_eq!(
        h.get_mut(ComponentId(10))
            .unwrap()
            .component_mut::<TreeContent>()
            .unwrap()
            .replace_nodes(D(1), vec![a]),
        Err(TreeContentError::HostTransactionRequired)
    );
    assert_eq!(parts(&h), before);
}
#[test]
fn zero_area_structure_keeps_parent_closure_and_no_actions() {
    let mut h = host();
    paint_clip(&mut h, Rect::new(0., 50., 320., 20.));
    let p = parts(&h);
    assert_eq!(p.len(), 3);
    assert!(p[0].frame.is_empty());
    assert_eq!(
        h.semantic_part_parent(ComponentId(10), p[1].index),
        Some(p[0].index)
    );
    for action in [
        AccessibilityAction::Focus,
        AccessibilityAction::Press,
        AccessibilityAction::SetExpanded(false),
    ] {
        assert!(
            !h.accessibility_action(ComponentId(10), Some(p[0].index), action)
                .consumed
        );
    }
    assert!(h.drain_action_records().is_empty());
    let tokens = p.iter().map(|p| p.index).collect::<Vec<_>>();
    paint_clip(&mut h, Rect::new(500., 0., 50., 100.));
    let offscreen = parts(&h);
    assert_eq!(
        offscreen.iter().map(|p| p.index).collect::<Vec<_>>(),
        tokens
    );
    assert!(offscreen.iter().all(|p| p.frame.is_empty() && !p.focused));
    assert_eq!(
        h.semantic_part_parent(ComponentId(10), offscreen[1].index),
        Some(offscreen[0].index)
    );
}
#[test]
fn stale_pointer_up_cannot_activate_another_entity() {
    let mut h = host();
    let p = point(&h, 1);
    h.event(&ContentEvent::PointerDown(p));
    h.replace_tree_nodes(
        ComponentId(10),
        D(1),
        vec![n(1, None), n(4, Some(1)), n(3, None)],
    )
    .unwrap();
    paint(&mut h);
    h.event(&ContentEvent::PointerUp(p));
    assert!(h.drain_action_records().is_empty());
}
#[cfg(target_os = "macos")]
#[path = "../../../examples/composition-consumer/tests/support/coretext.rs"]
mod coretext;
#[cfg(target_os = "macos")]
#[test]
fn neighbor_real_preedit_blocks_all_tree_ax_transfer_and_background_update_preserves_binding() {
    let root = Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(ComponentId(10), tree()),
                Content::new(
                    ComponentId(20),
                    TextFieldContent::new("Composer", "draft😀"),
                ),
            ],
        },
    );
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut icons = Icons;
    let render = |h: &mut ContentHost, text: &mut coretext::Text, icons: &mut Icons| {
        let f = Rect::new(0., 0., 320., 400.);
        h.render(
            &mut Scene::new(),
            &mut WithIconRenderer::new(text, icons),
            f,
            f,
            Theme::LIGHT.into(),
        )
        .unwrap();
    };
    let mut h = ContentHost::new(root).unwrap();
    render(&mut h, &mut text, &mut icons);
    assert!(h.focus(Some(ComponentId(20)), true));
    h.event_with_text(
        &ContentEvent::Preedit {
            text: "拼音".into(),
            selection: 0..2,
            clauses: vec![],
            replacement: None,
        },
        &mut text,
    )
    .unwrap();
    assert!(h.is_composing());
    let old = h.semantic_text_input_snapshot(ComponentId(20)).unwrap();
    let token = parts(&h)[0].index;
    for action in [
        AccessibilityAction::Focus,
        AccessibilityAction::Press,
        AccessibilityAction::SetExpanded(false),
    ] {
        assert!(
            !h.accessibility_action(ComponentId(10), Some(token), action)
                .consumed
        );
        assert_eq!(h.focused(), Some(ComponentId(20)));
    }
    h.replace_tree_nodes(
        ComponentId(10),
        D(1),
        vec![n(3, None), n(1, None), n(2, Some(1))],
    )
    .unwrap();
    h.set_inherited_surface("canvas").unwrap();
    render(&mut h, &mut text, &mut icons);
    assert_eq!(
        h.semantic_text_input_snapshot(ComponentId(20)).unwrap(),
        old
    );
    assert!(h.is_composing());
    assert_eq!(h.focused(), Some(ComponentId(20)));
}
#[derive(Debug)]
struct Activity(Rc<std::cell::RefCell<Vec<bool>>>);
impl Component for Activity {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(20., 20.))
    }
    fn window_active_changed(&mut self, b: bool) {
        self.0.borrow_mut().push(b);
    }
}
#[test]
fn original_host_observation_reaches_new_inactive_subtrees_and_ignores_repeat() {
    let log = Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut h = ContentHost::new(Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![],
        },
    ))
    .unwrap();
    h.set_window_active(false);
    h.insert(
        ComponentId(1),
        Content::new(ComponentId(2), Activity(log.clone())),
    )
    .unwrap();
    assert_eq!(*log.borrow(), vec![false]);
    h.set_window_active(false);
    assert_eq!(*log.borrow(), vec![false]);
    h.set_window_active(true);
    assert_eq!(*log.borrow(), vec![false, true]);
}

#[test]
fn caller_wrapper_identity_is_stable_for_all_display_variants_and_collision_is_recoverable() {
    for update in 0..3 {
        let mut h = host();
        let before = parts(&h);
        let mut changed = n(1, None);
        changed.row_id = ComponentId(500);
        changed.display = match update {
            0 => TreeDisplayUpdate::Keep,
            1 => TreeDisplayUpdate::DefaultLabel,
            _ => TreeDisplayUpdate::Replace(Content::new(
                ComponentId(501),
                TextContent::new("label"),
            )),
        };
        assert!(matches!(
            h.replace_tree_nodes(ComponentId(10), D(1), vec![changed]),
            Err(TreeContentError::RowIdentityChanged { .. })
        ));
        assert_eq!(parts(&h), before);
    }
    let mut bad = n(4, None);
    bad.row_id = ComponentId(10);
    let mut h = host();
    let before = parts(&h);
    assert!(matches!(
        h.replace_tree_nodes(ComponentId(10), D(1), vec![bad]),
        Err(TreeContentError::Composition(
            CompositionError::DuplicateId(ComponentId(10))
        ))
    ));
    assert_eq!(parts(&h), before);
    h.replace_tree_nodes(ComponentId(10), D(1), vec![n(4, None)])
        .unwrap();
    paint(&mut h);
    assert_eq!(parts(&h).len(), 1);
    let old = parts(&h)[0].index;
    h.replace_tree_nodes(ComponentId(10), D(2), vec![n(4, None)])
        .unwrap();
    h.replace_tree_nodes(ComponentId(10), D(1), vec![n(4, None)])
        .unwrap();
    paint(&mut h);
    assert_ne!(parts(&h)[0].index, old);
}

#[test]
fn focus_is_distinct_from_selection_and_leaf_disabled_disclosure_is_rejected() {
    let mut h = host();
    let p = parts(&h);
    assert!(
        h.accessibility_action(
            ComponentId(10),
            Some(p[1].index),
            AccessibilityAction::Focus
        )
        .consumed
    );
    assert_eq!(t(&h).selected_key(), None);
    assert!(parts(&h)[1].focused);
    assert!(h.drain_action_records().is_empty());
    assert!(
        !h.accessibility_action(
            ComponentId(10),
            Some(p[1].index),
            AccessibilityAction::SetExpanded(false)
        )
        .consumed
    );
    let mut root = n(1, None);
    root.node.disabled = true;
    h.replace_tree_nodes(ComponentId(10), D(1), vec![root, n(2, Some(1)), n(3, None)])
        .unwrap();
    paint(&mut h);
    let p = parts(&h);
    assert!(
        !h.accessibility_action(
            ComponentId(10),
            Some(p[0].index),
            AccessibilityAction::SetExpanded(false)
        )
        .consumed
    );
    assert!(
        h.accessibility_action(
            ComponentId(10),
            Some(p[1].index),
            AccessibilityAction::Press
        )
        .activated
    );
    assert_eq!(t(&h).selected_key(), Some(K(2)));
}
#[test]
fn all_target_ids_and_nested_capabilities_validate_before_any_mutation() {
    let mut h = host();
    press(&mut h, 1);
    let before = parts(&h);
    let mut a = n(1, None);
    a.display = TreeDisplayUpdate::Replace(Content::new(
        ComponentId(1010),
        StackContent {
            model: Stack {
                axis: Axis::Horizontal,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![Content::new(ComponentId(1011), Button::new("nested"))],
        },
    ));
    assert!(matches!(
        h.replace_tree_nodes(ComponentId(10), D(1), vec![a]),
        Err(TreeContentError::InteractiveDisplay {
            key: K(1),
            child: ComponentId(1011)
        })
    ));
    assert_eq!(parts(&h), before);
    let mut a = n(1, None);
    a.display = TreeDisplayUpdate::Replace(Content::new(
        ComponentId(1001),
        TextContent::new("wrapper collision"),
    ));
    assert!(matches!(
        h.replace_tree_nodes(ComponentId(10), D(1), vec![a]),
        Err(TreeContentError::Composition(
            CompositionError::DuplicateId(ComponentId(1001))
        ))
    ));
    assert_eq!(parts(&h), before);
    let mut a = n(1, None);
    a.node.parent = Some(K(99));
    assert!(matches!(
        h.replace_tree_nodes(ComponentId(10), D(1), vec![a]),
        Err(TreeContentError::Model(_))
    ));
    assert_eq!(parts(&h), before);
    assert_eq!(h.drain_action_records().len(), 1);
    let mut nested = n(1, None);
    nested.display = TreeDisplayUpdate::Replace(Content::new(ComponentId(900), tree()));
    assert!(matches!(
        h.replace_tree_nodes(ComponentId(10), D(1), vec![nested]),
        Err(TreeContentError::InteractiveDisplay {
            key: K(1),
            child: ComponentId(900)
        })
    ));
}
#[test]
fn replacing_display_keeps_wrapper_mount_and_not_old_display_mount() {
    let mut h = host();
    let wrapper = h.semantic_mount(ComponentId(1001));
    let mut a = n(1, None);
    a.display =
        TreeDisplayUpdate::Replace(Content::new(ComponentId(600), TextContent::new("custom")));
    h.replace_tree_nodes(ComponentId(10), D(1), vec![a, n(2, Some(1)), n(3, None)])
        .unwrap();
    assert_eq!(h.semantic_mount(ComponentId(1001)), wrapper);
    let display = h.semantic_mount(ComponentId(600));
    assert!(display.is_some());
    let mut a = n(1, None);
    a.display = TreeDisplayUpdate::DefaultLabel;
    h.replace_tree_nodes(ComponentId(10), D(1), vec![a, n(2, Some(1)), n(3, None)])
        .unwrap();
    assert_eq!(h.semantic_mount(ComponentId(1001)), wrapper);
    assert_eq!(h.semantic_mount(ComponentId(600)), None);
}
#[test]
fn original_enter_and_space_do_not_double_activate() {
    use tekes_ui_controls::ControlKey;
    let mut h = host();
    let p = parts(&h)[1].index;
    h.accessibility_action(ComponentId(10), Some(p), AccessibilityAction::Focus);
    h.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    let i = intent(&mut h);
    assert_eq!(i.kind, TreeIntentKind::Activated);
    assert_eq!(i.key, K(2));
    h.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    h.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: true,
    });
    h.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: false,
    });
    assert!(h.drain_action_records().is_empty());
}

#[test]
fn intent_from_different_root_instance_never_authorizes_matching_numbers() {
    let mut old = host();
    press(&mut old, 1);
    let i = intent(&mut old);
    let new = host();
    assert!(!t(&new).intent_is_current(&i));
}

#[test]
fn same_tree_instance_remount_revokes_exposure_and_pending_original_mount() {
    let root = Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![Content::new(ComponentId(10), tree())],
        },
    );
    let mut h = ContentHost::new(root).unwrap();
    h.set_action_events(true);
    paint(&mut h);
    press(&mut h, 1);
    let old = intent(&mut h);
    let old_mount = h.semantic_mount(ComponentId(10));
    press(&mut h, 1);
    let retained = h.remove(ComponentId(10)).unwrap();
    assert!(
        !retained
            .component::<TreeContent>()
            .unwrap()
            .intent_is_current(&old)
    );
    h.insert(ComponentId(1), retained).unwrap();
    assert_ne!(h.semantic_mount(ComponentId(10)), old_mount);
    assert!(!t(&h).intent_is_current(&old));
    assert!(h.drain_action_records().is_empty());
    paint(&mut h);
    assert!(parts(&h)[1].index > old.exposure);
    press(&mut h, 1);
    let new = intent(&mut h);
    assert_ne!(new, old);
    assert_eq!(new, new.clone());
    assert!(t(&h).intent_is_current(&new));
}
#[test]
fn empty_selectable_list_is_an_interaction_owner_but_passive_list_is_allowed() {
    let empty = SelectableListContent::new("Empty", vec![]);
    assert!(!empty.focusable());
    assert_eq!(empty.model.count, 0);
    assert_eq!(empty.model.selected, None);
    let mut node = n(1, None);
    node.display = TreeDisplayUpdate::Replace(Content::new(ComponentId(700), empty));
    assert!(matches!(
        TreeContent::new("Outer", D(1), vec![node]),
        Err(TreeContentError::InteractiveDisplay {
            key: K(1),
            child: ComponentId(700)
        })
    ));
    let mut passive = n(1, None);
    passive.display = TreeDisplayUpdate::Replace(Content::new(
        ComponentId(701),
        ListContent::new(
            "Passive",
            vec![Content::new(ComponentId(702), TextContent::new("text"))],
        ),
    ));
    assert!(TreeContent::new("Outer", D(1), vec![passive]).is_ok());
    let mut list = ContentHost::new(Content::new(
        ComponentId(800),
        SelectableListContent::new("Original", vec!["first".into(), "second".into()]),
    ))
    .unwrap();
    paint(&mut list);
    assert_eq!(list.focus_targets().len(), 1);
    assert!(
        list.accessibility_action(ComponentId(800), Some(1), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(
        list.get(ComponentId(800))
            .unwrap()
            .component::<SelectableListContent>()
            .unwrap()
            .model
            .selected,
        Some(1)
    );
}

fn exercise_malformed_rows(mode: usize) {
    let mut h = host();
    let old = parts(&h);
    let old_focus = h.focused();
    let old_selection = t(&h).selected_key();
    let stolen = if mode == 5 {
        Some(tree().children_mut().unwrap().remove(0))
    } else {
        None
    };
    let rows = h
        .get_mut(ComponentId(10))
        .unwrap()
        .component_mut::<TreeContent>()
        .unwrap()
        .children_mut()
        .unwrap();
    match mode {
        0 => rows.clear(),
        1 => {
            rows.pop();
        }
        2 => rows.swap(0, 1),
        3 => rows.push(Content::new(ComponentId(999), TextContent::new("extra"))),
        4 => rows[0] = Content::new(ComponentId(1001), TextContent::new("same ID, wrong type")),
        _ => rows[0] = stolen.unwrap(),
    }
    let checks = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for _ in 0..3 {
            assert!(t(&h).semantic_parts().is_empty());
            assert_eq!(h.semantics().len(), 1);
            assert_eq!(t(&h).semantic_part_parent(old[1].index), None);
            assert_eq!(h.focused(), old_focus);
            assert_eq!(t(&h).selected_key(), old_selection);
        }
        let tree = h
            .get_mut(ComponentId(10))
            .unwrap()
            .component_mut::<TreeContent>()
            .unwrap();
        assert!(!tree.select_key(K(2)));
        assert!(
            !tree
                .event(&ContentEvent::PointerDown(old[0].frame.center()))
                .consumed
        );
        assert!(
            !tree
                .event(&ContentEvent::Key {
                    key: tekes_ui_controls::ControlKey::Enter,
                    down: true
                })
                .activated
        );
        assert!(
            !h.accessibility_action(
                ComponentId(10),
                Some(old[0].index),
                AccessibilityAction::Press
            )
            .consumed
        );
        assert_eq!(
            h.replace_tree_nodes(
                ComponentId(10),
                D(1),
                vec![n(1, None), n(2, Some(1)), n(3, None)]
            ),
            Err(TreeContentError::InvalidStructure)
        );
        let f = Rect::new(0., 0., 320., 400.);
        assert!(
            h.render(
                &mut Scene::new(),
                &mut Text::default(),
                f,
                f,
                Theme::LIGHT.into()
            )
            .is_err()
        );
        assert!(h.drain_action_records().is_empty());
    }));
    assert!(
        checks.is_ok(),
        "malformed structure mode {mode} must reject without panic/action"
    );
}
#[test]
fn malformed_rows_clear_rejects() {
    exercise_malformed_rows(0);
}
#[test]
fn malformed_rows_pop_rejects() {
    exercise_malformed_rows(1);
}
#[test]
fn malformed_rows_swap_rejects() {
    exercise_malformed_rows(2);
}
#[test]
fn malformed_rows_push_rejects() {
    exercise_malformed_rows(3);
}
#[test]
fn malformed_rows_same_id_wrong_type_rejects() {
    exercise_malformed_rows(4);
}
#[test]
fn malformed_rows_same_id_borrowed_wrapper_rejects() {
    exercise_malformed_rows(5);
}
