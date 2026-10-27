use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ScrollArea, TextInput,
    composition::*,
    extended::{Axis, Grid, Stack},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
#[derive(Default)]
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, max: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(max), 24.)
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
fn id(n: u64) -> ComponentId {
    ComponentId(n)
}
fn button(n: u64) -> Content {
    Content::new(id(n), Button::new(format!("Action {n}")))
}
fn stack(children: Vec<Content>) -> Content {
    Content::new(
        id(1),
        StackContent {
            flexible_child: None,
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            children,
        },
    )
}
fn render(h: &mut ContentHost) {
    h.render(
        &mut Scene::new(),
        &mut Text,
        Rect::new(0., 0., 300., 120.),
        Rect::new(0., 0., 300., 120.),
        Theme::LIGHT.into(),
    )
    .unwrap();
}
#[derive(Debug)]
struct Editor {
    input: TextInput,
    unmounts: Rc<Cell<usize>>,
}
impl Component for Editor {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(180., 40.))
    }
    fn focusable(&self) -> bool {
        true
    }
    fn composing(&self) -> bool {
        self.input.marked.is_some()
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        match e {
            ContentEvent::Commit { text, replacement } => {
                ContentResponse::consumed(self.input.insert(text, replacement.clone()))
            }
            ContentEvent::Preedit {
                text,
                selection,
                replacement,
                ..
            } => {
                self.input
                    .preedit(text, selection.clone(), replacement.clone());
                ContentResponse::consumed(true)
            }
            _ => ContentResponse::default(),
        }
    }
    fn lifecycle(&mut self, e: Lifecycle) {
        if e == Lifecycle::Unmounted {
            self.unmounts.set(self.unmounts.get() + 1)
        }
    }
}
#[test]
fn focused_ime_owns_raw_control_keys_before_any_overlay_router() {
    for key in [
        tekes_ui_controls::ControlKey::Enter,
        tekes_ui_controls::ControlKey::Space,
        tekes_ui_controls::ControlKey::Up,
        tekes_ui_controls::ControlKey::Down,
        tekes_ui_controls::ControlKey::Left,
        tekes_ui_controls::ControlKey::Right,
        tekes_ui_controls::ControlKey::Home,
        tekes_ui_controls::ControlKey::End,
        tekes_ui_controls::ControlKey::Escape,
        tekes_ui_controls::ControlKey::Tab { reverse: false },
        tekes_ui_controls::ControlKey::Tab { reverse: true },
    ] {
        let mut h = ContentHost::new(stack(vec![
            Content::new(
                id(2),
                MenuContent::new("Options", vec!["One".into(), "Two".into()]),
            ),
            Content::new(
                id(3),
                Editor {
                    input: TextInput::new("draft", false),
                    unmounts: Rc::new(Cell::new(0)),
                },
            ),
        ]))
        .unwrap();
        render(&mut h);
        h.focus(Some(id(3)), true);
        h.event(&ContentEvent::Preedit {
            text: "候補".into(),
            selection: 1..1,
            replacement: Some(0..0),
            clauses: std::iter::once(0..2).collect(),
        });
        h.get_mut(id(2))
            .unwrap()
            .component_mut::<MenuContent>()
            .unwrap()
            .menu
            .show(0);
        render(&mut h);
        let before = h
            .get(id(3))
            .unwrap()
            .component::<Editor>()
            .unwrap()
            .input
            .text
            .clone();
        for down in [true, false] {
            let event = h.event(&ContentEvent::Key { key, down });
            assert!(
                !event.consumed && !event.activated,
                "IME raw key must remain available to the native input method: {key:?}"
            );
            let menu = h.get(id(2)).unwrap().component::<MenuContent>().unwrap();
            assert!(menu.menu.is_open());
            assert_eq!(menu.selected, None);
            assert_eq!(
                h.get(id(3))
                    .unwrap()
                    .component::<Editor>()
                    .unwrap()
                    .input
                    .text,
                before
            );
            assert!(
                h.get(id(3))
                    .unwrap()
                    .component::<Editor>()
                    .unwrap()
                    .input
                    .marked
                    .is_some()
            );
            assert_eq!(h.focused(), Some(id(3)));
        }
    }
}

#[test]
fn heterogeneous_children_reorder_preserves_editor_selection_ime_and_focus() {
    let count = Rc::new(Cell::new(0));
    let editor = Content::new(
        id(3),
        Editor {
            input: TextInput::new("draft", false),
            unmounts: count.clone(),
        },
    );
    let mut h = ContentHost::new(stack(vec![
        button(2),
        editor,
        Content::new(id(4), TextContent::new("Caption")),
    ]))
    .unwrap();
    render(&mut h);
    h.focus(Some(id(3)), true);
    h.event(&ContentEvent::Preedit {
        text: "候補".into(),
        selection: 1..1,
        replacement: Some(0..0),
        clauses: std::iter::once(0..2).collect(),
    });
    let before = h
        .get(id(3))
        .unwrap()
        .component::<Editor>()
        .unwrap()
        .input
        .text
        .clone();
    assert!(
        !h.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Tab { reverse: false },
            down: true
        })
        .consumed
    );
    h.reorder(id(1), &[id(4), id(3), id(2)]).unwrap();
    render(&mut h);
    let e = h.get(id(3)).unwrap().component::<Editor>().unwrap();
    assert_eq!(e.input.text, before);
    assert!(e.input.marked.is_some());
    assert_eq!(h.focused(), Some(id(3)));
    assert_eq!(count.get(), 0);
    assert_eq!(
        h.reorder(id(1), &[id(3), id(3), id(2)]),
        Err(CompositionError::InvalidOrder)
    );
    let removed = h.remove(id(3)).unwrap();
    assert_eq!(h.focused(), None);
    assert_eq!(count.get(), 1);
    h.insert(id(1), removed).unwrap();
    render(&mut h);
    assert!(
        h.get(id(3))
            .unwrap()
            .component::<Editor>()
            .unwrap()
            .input
            .marked
            .is_some()
    );
}
#[test]
fn duplicate_keys_fail_before_mount_or_mutation() {
    assert!(matches!(
        ContentHost::new(stack(vec![button(2), button(2)])),
        Err(CompositionError::DuplicateId(ComponentId(2)))
    ));
    let mut h = ContentHost::new(stack(vec![button(2)])).unwrap();
    assert_eq!(
        h.insert(id(1), button(2)),
        Err(CompositionError::DuplicateId(id(2)))
    );
    assert!(h.get(id(2)).is_some());
}
#[test]
fn nested_scroll_grid_uses_real_button_and_same_clip_for_hits() {
    let grid = Content::new(
        id(3),
        GridContent {
            model: Grid {
                columns: 1,
                gap_x: 0.,
                gap_y: 8.,
            },
            children: (4..14).map(button).collect(),
        },
    );
    let mut h = ContentHost::new(Content::new(
        id(1),
        ScrollContent::new(ScrollArea::default(), grid),
    ))
    .unwrap();
    render(&mut h);
    let first = h.get(id(4)).unwrap().frame();
    let focus = tekes_ui_controls::extended::ControlFrame::new(first, None).focus_bounds(1.0);
    assert!(
        focus.min_x() >= 0.0 && focus.min_y() >= 0.0,
        "scroll document reserves the shared focus gutter"
    );
    let p = h.get(id(4)).unwrap().frame().center();
    let down = h.event(&ContentEvent::PointerDown(p));
    assert!(down.consumed && down.capture_pointer);
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(
        h.get(id(4))
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        1
    );
    assert!(
        !h.event(&ContentEvent::PointerDown(Point::new(320., 20.)))
            .consumed
    );
    let wheel = h.event(&ContentEvent::Scroll {
        position: p,
        delta: Point::new(0., 40.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 1.,
    });
    assert!(
        wheel.consumed && wheel.changed,
        "wheel bubbles past the button to its scroll ancestor"
    );
    assert!(
        h.get(id(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y
            > 0.
    );
    h.focus(Some(id(13)), true);
    render(&mut h);
    assert!(
        h.get(id(13)).unwrap().visible().is_some(),
        "focus reveals offscreen mounted content"
    );
    let node = h.semantics().into_iter().find(|n| n.id == id(13)).unwrap();
    assert_eq!(node.parent, Some(id(3)));
    assert!(node.focused);
    assert_eq!(node.semantics.role, ContentRole::Button);
}
#[test]
fn clipping_or_unmounting_capture_cancels_activation_and_focus() {
    let mut h = ContentHost::new(stack(vec![button(2)])).unwrap();
    render(&mut h);
    let p = h.get(id(2)).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(p));
    h.render(
        &mut Scene::new(),
        &mut Text,
        Rect::new(0., 0., 300., 120.),
        Rect::default(),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    render(&mut h);
    h.event(&ContentEvent::PointerDown(p));
    let _removed = h.remove(id(2)).unwrap();
    assert_eq!(h.focused(), None);
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
}
#[test]
fn theme_and_relayout_preserve_real_button_keyboard_arm() {
    let mut h = ContentHost::new(stack(vec![button(2)])).unwrap();
    render(&mut h);
    h.focus(Some(id(2)), true);
    h.event(&ContentEvent::Key {
        key: tekes_ui_controls::ControlKey::Space,
        down: true,
    });
    for theme in Theme::ALL {
        h.render(
            &mut Scene::new(),
            &mut Text,
            Rect::new(0., 0., 200., 120.),
            Rect::new(0., 0., 200., 120.),
            Palette::from(theme),
        )
        .unwrap();
    }
    assert!(
        h.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Space,
            down: false
        })
        .activated
    );
}

#[test]
fn compound_action_has_one_focus_one_name_and_rejects_nested_controls() {
    let content = Content::new(
        id(2),
        StackContent {
            flexible_child: None,
            model: Stack {
                axis: Axis::Vertical,
                gap: 4.,
            },
            children: vec![
                Content::new(id(3), TextContent::new("Title")),
                Content::new(id(4), TextContent::new("Description")),
            ],
        },
    );
    let action = Button::new(content).with_accessible_name("Open workspace");
    let mut h = ContentHost::new(Content::new(id(1), action)).unwrap();
    render(&mut h);
    assert_eq!(h.focus_targets().len(), 1);
    assert_eq!(h.semantics().len(), 1);
    assert_eq!(h.semantics()[0].semantics.name, "Open workspace");
    let p = h.get(id(3)).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(p));
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(
        h.get(id(1))
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        1
    );
    assert!(
        h.get_mut(id(3))
            .unwrap()
            .component_mut::<TextContent>()
            .is_some(),
        "fixed action slot still uses the common content traversal"
    );
    let bad = Button::new(button(7)).with_accessible_name("Bad nesting");
    assert!(matches!(
        ContentHost::new(Content::new(id(6), bad)),
        Err(CompositionError::NestedInteractive {
            action: ComponentId(6),
            child: ComponentId(7)
        })
    ));
    assert!(matches!(
        ContentHost::new(Content::new(
            id(6),
            Button::new(button(7)).with_accessible_name("  ")
        )),
        Err(CompositionError::EmptyAccessibleName)
    ));
}
#[derive(Debug)]
struct Drag {
    events: Rc<Cell<usize>>,
}
impl Component for Drag {
    fn interactive(&self) -> bool {
        true
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(80., 40.))
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        match e {
            ContentEvent::PointerDown(_) => ContentResponse {
                consumed: true,
                capture_pointer: true,
                ..Default::default()
            },
            ContentEvent::PointerUp(_) => {
                self.events.set(self.events.get() + 1);
                ContentResponse::consumed(true)
            }
            _ => ContentResponse::default(),
        }
    }
}
#[test]
fn pointer_capture_is_independent_of_keyboard_focus() {
    let count = Rc::new(Cell::new(0));
    let mut h = ContentHost::new(stack(vec![Content::new(
        id(2),
        Drag {
            events: count.clone(),
        },
    )]))
    .unwrap();
    render(&mut h);
    let p = h.get(id(2)).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(p));
    render(&mut h);
    h.event(&ContentEvent::PointerUp(p));
    assert_eq!(count.get(), 1);
    assert_eq!(h.focused(), None);
}

#[derive(Debug)]
struct Row {
    children: Vec<Content>,
    activations: Rc<Cell<usize>>,
}
impl Component for Row {
    fn interactive(&self) -> bool {
        true
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(300., 80.))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.children[0].arrange(t, Rect::new(f.min_x(), f.min_y(), 90., 40.), c)
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        if matches!(e, ContentEvent::PointerDown(_)) {
            self.activations.set(self.activations.get() + 1);
            ContentResponse::consumed(true)
        } else {
            ContentResponse::default()
        }
    }
}
#[test]
fn consumed_child_input_does_not_activate_the_containing_row() {
    let count = Rc::new(Cell::new(0));
    let mut h = ContentHost::new(Content::new(
        id(1),
        Row {
            children: vec![button(2)],
            activations: count.clone(),
        },
    ))
    .unwrap();
    render(&mut h);
    let p = h.get(id(2)).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(p));
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(count.get(), 0);
    h.event(&ContentEvent::PointerDown(Point::new(200., 60.)));
    assert_eq!(count.get(), 1);
}
#[test]
fn fixed_action_slots_fail_closed_after_invalid_direct_structural_mutation() {
    let action =
        Button::new(Content::new(id(2), TextContent::new("Text"))).with_accessible_name("Action");
    let mut h = ContentHost::new(Content::new(id(1), action)).unwrap();
    render(&mut h);
    h.get_mut(id(1))
        .unwrap()
        .component_mut::<Button>()
        .unwrap()
        .children_mut()
        .unwrap()
        .clear();
    assert_eq!(
        h.render(
            &mut Scene::new(),
            &mut Text,
            Rect::new(0., 0., 300., 100.),
            Rect::new(0., 0., 300., 100.),
            Theme::LIGHT.into()
        ),
        Err(CompositionError::InvalidSlotCount)
    );
}

#[test]
fn pointer_only_consumer_controls_are_diagnosed_inside_an_action() {
    let drag = Content::new(
        id(2),
        Drag {
            events: Rc::new(Cell::new(0)),
        },
    );
    let action = Button::new(drag).with_accessible_name("Invalid action");
    assert!(matches!(
        ContentHost::new(Content::new(id(1), action)),
        Err(CompositionError::NestedInteractive {
            action: ComponentId(1),
            child: ComponentId(2)
        })
    ));
}

#[test]
fn rejected_removal_preserves_actual_dialog_scope_and_focused_child() {
    let dialog = Content::new(
        id(2),
        DialogContent::new(
            "Settings",
            Content::new(id(3), TextContent::new("Header")),
            Content::new(id(4), TextContent::new("Body")),
            button(5),
        ),
    );
    let mut host = ContentHost::new(stack(vec![dialog])).unwrap();
    host.show_dialog(id(2)).unwrap();
    render(&mut host);
    assert_eq!(host.focused(), Some(id(5)));
    let before: Vec<_> = host.semantics().into_iter().map(|n| n.id).collect();
    assert!(matches!(
        host.remove(id(1)),
        Err(CompositionError::MissingId(_))
    ));
    assert!(
        host.get(id(2))
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_open(),
        "failed removal must not dismiss a modal"
    );
    assert_eq!(host.focused(), Some(id(5)));
    assert_eq!(
        host.semantics()
            .into_iter()
            .map(|n| n.id)
            .collect::<Vec<_>>(),
        before
    );
}

#[derive(Debug)]
struct VisibleContent {
    visible: Rc<Cell<bool>>,
    children: Vec<Content>,
}
impl Component for VisibleContent {
    fn active(&self) -> bool {
        self.visible.get()
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.children[0].measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.children[0].arrange(t, f, c);
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
}
#[test]
fn inactive_retained_content_rejects_keyboard_and_old_pointer_capture() {
    for paint in [false, true] {
        let visible = Rc::new(Cell::new(true));
        let mut host = ContentHost::new(Content::new(
            id(1),
            VisibleContent {
                visible: visible.clone(),
                children: vec![Content::new(
                    id(2),
                    Editor {
                        input: TextInput::new("draft", false),
                        unmounts: Rc::new(Cell::new(0)),
                    },
                )],
            },
        ))
        .unwrap();
        render(&mut host);
        assert!(host.focus(Some(id(2)), true));
        visible.set(false);
        if paint {
            render(&mut host);
            assert_eq!(host.focused(), None);
        }
        let response = host.event(&ContentEvent::Commit {
            text: "hidden".into(),
            replacement: None,
        });
        assert!(
            !response.consumed,
            "inactive subtree must reject keyboard before and after paint"
        );
        assert_eq!(
            host.get(id(2))
                .unwrap()
                .component::<Editor>()
                .unwrap()
                .input
                .text,
            "draft"
        );
        assert_eq!(host.focused(), None);
        let visible = Rc::new(Cell::new(true));
        let mut host = ContentHost::new(Content::new(
            id(1),
            VisibleContent {
                visible: visible.clone(),
                children: vec![button(2)],
            },
        ))
        .unwrap();
        render(&mut host);
        let point = host.get(id(2)).unwrap().frame().center();
        assert!(
            host.event(&ContentEvent::PointerDown(point))
                .capture_pointer
        );
        visible.set(false);
        if paint {
            render(&mut host);
        }
        assert!(
            !host.event(&ContentEvent::PointerUp(point)).activated,
            "inactive subtree must cancel the previous capture"
        );
        visible.set(true);
        render(&mut host);
        assert!(!host.event(&ContentEvent::PointerUp(point)).activated);
    }
}

#[test]
fn inactive_ancestor_dismisses_a_nested_modal_and_its_retained_paint() {
    let visible = Rc::new(Cell::new(true));
    let dialog = DialogContent::new(
        "Panel dialog",
        Content::new(id(3), TextContent::new("Title")),
        button(4),
        button(5),
    );
    let mut host = ContentHost::new(Content::new(
        id(1),
        VisibleContent {
            visible: visible.clone(),
            children: vec![Content::new(id(2), dialog)],
        },
    ))
    .unwrap();
    render(&mut host);
    host.show_dialog(id(2)).unwrap();
    render(&mut host);
    assert!(host.semantics().iter().any(|n| n.id == id(2)));
    visible.set(false);
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        &mut Text,
        Rect::new(0., 0., 300., 120.),
        Rect::new(0., 0., 300., 120.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(scene.is_empty());
    assert!(host.semantics().is_empty());
    assert_eq!(host.focused(), None);
    assert!(!host.close_dialog());
}

#[test]
fn captured_pointer_release_reaches_owner_before_new_overlay() {
    let count = Rc::new(Cell::new(0));
    let mut h = ContentHost::new(stack(vec![
        Content::new(id(2), MenuContent::new("Options", vec!["One".into()])),
        Content::new(
            id(3),
            Drag {
                events: count.clone(),
            },
        ),
    ]))
    .unwrap();
    render(&mut h);
    let point = h.get(id(3)).unwrap().frame().center();
    assert!(h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    h.get_mut(id(2))
        .unwrap()
        .component_mut::<MenuContent>()
        .unwrap()
        .menu
        .show(0);
    render(&mut h);
    let point = h
        .semantics()
        .into_iter()
        .find(|n| n.id == id(2))
        .unwrap()
        .parts[0]
        .frame
        .center();
    assert!(!h.event(&ContentEvent::PointerUp(point)).activated);
    assert_eq!(
        count.get(),
        1,
        "new overlay must not steal captured release"
    );
}

#[derive(Debug)]
struct LinkOwner {
    children: Vec<Content>,
}
impl Component for LinkOwner {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(200., 40.))
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Link, "Documentation")
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
}
#[test]
fn link_semantics_reuse_single_action_ownership_and_fixed_children() {
    let owner = LinkOwner {
        children: vec![Content::new(id(2), TextContent::new("Decorative label"))],
    };
    assert!(
        owner.interactive(),
        "a disabled/non-focusable action remains interactive for nesting validation"
    );
    let mut h = ContentHost::new(Content::new(id(1), owner)).unwrap();
    render(&mut h);
    let nodes = h.semantics();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].semantics.role, ContentRole::Link);
    assert!(matches!(
        h.insert(id(1), Content::new(id(3), TextContent::new("Extra"))),
        Err(CompositionError::NotContainer(_))
    ));
    assert!(h.remove(id(2)).is_err());
    assert!(h.reorder(id(1), &[id(2)]).is_err());
    let nested = LinkOwner {
        children: vec![button(7)],
    };
    assert!(matches!(
        ContentHost::new(Content::new(id(6), nested)),
        Err(CompositionError::NestedInteractive {
            action: ComponentId(6),
            child: ComponentId(7)
        })
    ));
}
