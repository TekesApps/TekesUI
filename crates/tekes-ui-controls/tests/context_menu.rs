use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, TextInput,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
#[derive(Default)]
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, text: &str, _: proposal::TextRole, width: f32) -> Size {
        Size::new((text.len() as f32 * 8.).min(width), 24.)
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
const ROOT: ComponentId = ComponentId(1);
const A: ComponentId = ComponentId(2);
const B: ComponentId = ComponentId(3);
const MENU_A: ComponentId = ComponentId(4);
const MENU_B: ComponentId = ComponentId(5);
#[derive(Debug)]
struct Draft {
    input: TextInput,
}
impl Default for Draft {
    fn default() -> Self {
        Self {
            input: TextInput::new("", true),
        }
    }
}
impl Component for Draft {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(240., 40.))
    }
    fn focusable(&self) -> bool {
        true
    }
    fn composing(&self) -> bool {
        self.input.marked.is_some()
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::TextInput, "Draft A")
    }
}
fn entries() -> Vec<MenuEntry> {
    vec![
        MenuEntry::item("rename", "Rename"),
        MenuEntry::submenu("more", "More", vec![MenuEntry::item("archive", "Archive")]),
        MenuEntry::Item {
            value: "blocked".into(),
            label: "Unavailable".into(),
            checked: Some(true),
            disabled: true,
        },
    ]
}
fn menu(id: ComponentId) -> Content {
    Content::new(
        id,
        MenuContent::context_menu("Row actions", entries()).unwrap(),
    )
}
fn host() -> (ContentHost, ContextMenuBinding, ContextMenuBinding) {
    let mut host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![
                Content::new(A, Draft::default()),
                Content::new(B, Button::new("Row B")),
                menu(MENU_A),
                menu(MENU_B),
            ],
        },
    ))
    .unwrap();
    host.set_action_events(true);
    let a = host.bind_context_menu(A, MENU_A).unwrap();
    let b = host.bind_context_menu(B, MENU_B).unwrap();
    render(&mut host);
    assert!(host.focus(Some(A), true));
    (host, a, b)
}
fn render(host: &mut ContentHost) {
    host.render(
        &mut Scene::new(),
        &mut Text,
        Rect::new(0., 0., 300., 240.),
        Rect::new(0., 0., 300., 240.),
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn secondary(point: Point) -> ContentEvent {
    ContentEvent::Pointer(PointerEvent {
        position: point,
        phase: PointerPhase::Down,
        button: Some(PointerButton::Secondary),
        modifiers: InputModifiers::default(),
        click_count: 1,
    })
}
fn open(host: &mut ContentHost, owner: ComponentId) {
    let frame = host.get(owner).unwrap().frame();
    assert!(
        host.event(&secondary(Point::new(
            frame.min_x() + 8.,
            frame.min_y() + 8.
        )))
        .consumed
    );
    render(host);
}
fn activate(host: &mut ContentHost) {
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
}
fn selections(host: &mut ContentHost) -> Vec<MenuSelection> {
    host.drain_action_records()
        .into_iter()
        .map(|record| match record.payload.unwrap() {
            ContentActionPayload::MenuSelection(selection) => selection,
            other => panic!("unexpected {other:?}"),
        })
        .collect()
}
#[test]
fn context_menu_keeps_focus_draft_and_original_a_b_delayed_values() {
    let (mut host, _, _) = host();
    host.get_mut(A)
        .unwrap()
        .component_mut::<Draft>()
        .unwrap()
        .input
        .insert("A draft", None);
    open(&mut host, A);
    assert!(host.drain_actions().is_empty());
    activate(&mut host);
    open(&mut host, B);
    assert_eq!(host.focused(), Some(A));
    activate(&mut host);
    let values = selections(&mut host);
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].context_request().unwrap().owner(), A);
    assert_eq!(values[1].context_request().unwrap().owner(), B);
    assert_eq!(
        values[1].context_request().unwrap().source(),
        ContentActionSource::Pointer
    );
    assert_ne!(
        values[0].context_request().unwrap().anchor_identity(),
        values[1].context_request().unwrap().anchor_identity()
    );
    assert!(
        values
            .iter()
            .all(|value| value.value_path().unwrap() == ["rename"])
    );
    assert_eq!(
        host.get(A)
            .unwrap()
            .component::<Draft>()
            .unwrap()
            .input
            .text,
        "A draft"
    );
    assert!(host.drain_actions().is_empty());
}
#[test]
fn context_menu_marked_text_and_capture_policy_is_explicit() {
    let (mut host, _, b) = host();
    let draft = host.get_mut(A).unwrap().component_mut::<Draft>().unwrap();
    draft.input.preedit("中文", 0..1, None);
    let before = (
        draft.input.text.clone(),
        draft.input.marked.clone(),
        draft.input.selection(),
    );
    assert_eq!(
        host.request_context_menu(b, ContentActionSource::Keyboard),
        Err(ContextMenuError::Composing)
    );
    open(&mut host, B);
    assert!(
        !host
            .get(MENU_B)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    assert_eq!(host.focused(), Some(A));
    let after = &host.get(A).unwrap().component::<Draft>().unwrap().input;
    assert_eq!(after.text, before.0);
    assert_eq!(after.marked, before.1);
    assert_eq!(after.selection(), before.2);
}
#[test]
fn context_menu_rejects_deleted_remounted_owner_and_replaced_model_actions() {
    let (mut host, a, b) = host();
    open(&mut host, B);
    activate(&mut host);
    let row = host.remove(B).unwrap();
    host.insert(ROOT, row).unwrap();
    render(&mut host);
    assert!(selections(&mut host).is_empty());
    assert_eq!(
        host.request_context_menu(b, ContentActionSource::Keyboard),
        Err(ContextMenuError::StaleBinding)
    );
    let b2 = host.bind_context_menu(B, MENU_B).unwrap();
    host.request_context_menu(b2, ContentActionSource::Keyboard)
        .unwrap();
    render(&mut host);
    activate(&mut host);
    host.get_mut(MENU_B)
        .unwrap()
        .component_mut::<MenuContent>()
        .unwrap()
        .set_entries(vec![MenuEntry::item("delete", "Delete")])
        .unwrap();
    assert!(selections(&mut host).is_empty());
    host.request_context_menu(a, ContentActionSource::Keyboard)
        .unwrap();
    render(&mut host);
    activate(&mut host);
    assert_eq!(
        selections(&mut host)[0].context_request().unwrap().owner(),
        A
    );
}
#[test]
fn context_menu_keyboard_owner_rect_clamps_and_escape_outside_do_not_activate() {
    let (mut host, _, b) = host();
    host.focus(Some(B), true);
    let key = ContentKeyEvent {
        key: None,
        down: true,
        repeat: false,
        modifiers: InputModifiers {
            shift: true,
            ..Default::default()
        },
        native_code: 109,
        characters: "\u{f70d}".into(),
        characters_ignoring_modifiers: "\u{f70d}".into(),
    };
    assert!(host.event(&ContentEvent::KeyInput(key)).consumed);
    render(&mut host);
    let popup = host
        .get(MENU_B)
        .unwrap()
        .component::<MenuContent>()
        .unwrap()
        .popup
        .as_ref()
        .unwrap()
        .popup;
    assert!(popup.min_x() >= 0. && popup.max_x() <= 300. && popup.max_y() <= 240.);
    assert_eq!(host.focused(), Some(B));
    host.event(&ContentEvent::Key {
        key: ControlKey::Escape,
        down: true,
    });
    assert!(host.drain_actions().is_empty());
    host.request_context_menu(b, ContentActionSource::Accessibility)
        .unwrap();
    render(&mut host);
    host.event(&ContentEvent::PointerDown(Point::new(299., 239.)));
    assert!(
        !host
            .get(MENU_B)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    assert!(host.drain_actions().is_empty());
}
#[test]
fn context_menu_anchor_movement_clipping_and_deactivation_retire() {
    let (mut host, _, b) = host();
    host.request_context_menu(b, ContentActionSource::Keyboard)
        .unwrap();
    render(&mut host);
    host.get_mut(ROOT)
        .unwrap()
        .component_mut::<StackContent>()
        .unwrap()
        .model
        .gap = 10.;
    render(&mut host);
    assert!(
        !host
            .get(MENU_B)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    host.request_context_menu(b, ContentActionSource::Keyboard)
        .unwrap();
    render(&mut host);
    host.set_window_active(false);
    assert!(
        !host
            .get(MENU_B)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    host.set_window_active(true);
    render(&mut host);
    host.request_context_menu(b, ContentActionSource::Keyboard)
        .unwrap();
    host.render(
        &mut Scene::new(),
        &mut Text,
        Rect::new(0., 0., 300., 240.),
        Rect::new(0., 0., 300., 10.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(
        !host
            .get(MENU_B)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
}
#[test]
fn context_menu_declared_submenus_and_disabled_semantics_reuse_runtime() {
    let (mut host, _, b) = host();
    host.request_context_menu(b, ContentActionSource::Keyboard)
        .unwrap();
    render(&mut host);
    assert!(
        host.semantics()
            .iter()
            .flat_map(|node| &node.parts)
            .any(|part| part.semantics.name == "Unavailable" && part.semantics.disabled)
    );
    host.event(&ContentEvent::Key {
        key: ControlKey::Down,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Right,
        down: true,
    });
    render(&mut host);
    activate(&mut host);
    let values = selections(&mut host);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].value_path().unwrap(), ["more", "archive"]);
}

#[test]
fn context_menu_does_not_steal_an_existing_primary_capture_or_ordinary_overlay() {
    let (mut host, _, b) = host();
    let frame = host.get(B).unwrap().frame();
    let down = host.event(&ContentEvent::PointerDown(Point::new(
        frame.min_x() + 4.,
        frame.min_y() + 4.,
    )));
    assert!(down.capture_pointer);
    assert_eq!(
        host.request_context_menu(b, ContentActionSource::Keyboard),
        Err(ContextMenuError::InputCaptured)
    );
    host.event(&ContentEvent::Cancel);
    render(&mut host);
    host.insert(
        ROOT,
        Content::new(
            ComponentId(6),
            MenuContent::new("Options", vec!["Other".into()]),
        ),
    )
    .unwrap();
    render(&mut host);
    host.accessibility_action(ComponentId(6), None, AccessibilityAction::Press);
    assert_eq!(
        host.request_context_menu(b, ContentActionSource::Keyboard),
        Err(ContextMenuError::OverlayActive)
    );
}

#[test]
fn context_menu_semantic_root_is_the_real_popup_without_a_trigger_node() {
    let (mut host, _, b) = host();
    assert!(!host.focus_targets().iter().any(|(id, _)| *id == MENU_B));
    assert!(!host.semantics().iter().any(|node| node.id == MENU_B));
    host.request_context_menu(b, ContentActionSource::Keyboard)
        .unwrap();
    render(&mut host);
    let frame = host
        .get(MENU_B)
        .unwrap()
        .component::<MenuContent>()
        .unwrap()
        .popup
        .as_ref()
        .unwrap()
        .popup;
    let semantics = host.semantics();
    let menu = semantics.iter().find(|node| node.id == MENU_B).unwrap();
    assert_eq!(menu.semantics.role, ContentRole::Menu);
    assert_eq!(menu.frame, frame);
    assert!(
        menu.parts
            .iter()
            .all(|part| frame.intersection(&part.frame).is_some())
    );
    host.event(&ContentEvent::Key {
        key: ControlKey::Escape,
        down: true,
    });
    assert!(!host.semantics().iter().any(|node| node.id == MENU_B));
}

#[test]
fn context_menu_native_key_chain_cannot_be_interrupted_by_another_request() {
    let (mut host, _, b) = host();
    let key = ContentKeyEvent {
        key: None,
        down: true,
        repeat: false,
        modifiers: InputModifiers::default(),
        native_code: 0,
        characters: String::new(),
        characters_ignoring_modifiers: String::new(),
    };
    host.event(&ContentEvent::TextKeyStart {
        id: 4401,
        key,
        composing: true,
    });
    assert_eq!(
        host.request_context_menu(b, ContentActionSource::Keyboard),
        Err(ContextMenuError::InputCaptured)
    );
    host.event(&ContentEvent::TextKeyEnd {
        id: 4401,
        input_context_consumed: true,
    });
    host.request_context_menu(b, ContentActionSource::Keyboard)
        .unwrap();
    assert_eq!(host.focused(), Some(A));
}

#[test]
fn context_menu_shift_f10_opens_only_after_original_native_key_chain_finishes() {
    let (mut host, _, _) = host();
    host.focus(Some(B), true);
    let key = ContentKeyEvent {
        key: None,
        down: true,
        repeat: false,
        modifiers: InputModifiers {
            shift: true,
            ..Default::default()
        },
        native_code: 109,
        characters: "\u{f70d}".into(),
        characters_ignoring_modifiers: "\u{f70d}".into(),
    };
    host.event(&ContentEvent::TextKeyStart {
        id: 4402,
        key: key.clone(),
        composing: false,
    });
    host.event(&ContentEvent::KeyInput(key));
    assert!(
        !host
            .get(MENU_B)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    host.event(&ContentEvent::TextKeyEnd {
        id: 4402,
        input_context_consumed: false,
    });
    render(&mut host);
    assert!(
        host.get(MENU_B)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    assert_eq!(host.focused(), Some(B));
    assert!(host.drain_actions().is_empty());
    activate(&mut host);
    let selection = selections(&mut host).remove(0);
    assert_eq!(selection.context_request().unwrap().owner(), B);
    assert_eq!(
        selection.context_request().unwrap().source(),
        ContentActionSource::Keyboard
    );
}

#[test]
fn context_menu_bound_owner_cannot_open_through_front_interactive_sibling() {
    #[derive(Debug)]
    struct Overlap(Vec<Content>);
    impl Component for Overlap {
        fn children(&self) -> &[Content] {
            &self.0
        }
        fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
            Some(&mut self.0)
        }
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
            MeasuredChild::new(Size::new(200., 80.))
        }
        fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, _: Rect, clip: Rect) {
            for node in &mut self.0 {
                let frame = if node.id() == MENU_B {
                    Rect::default()
                } else {
                    Rect::new(10., 10., 160., 40.)
                };
                node.arrange(text, frame, clip);
            }
        }
    }
    let front = ComponentId(6);
    let mut host = ContentHost::new(Content::new(
        ROOT,
        Overlap(vec![
            Content::new(B, Button::new("B")),
            Content::new(front, Button::new("Front")),
            menu(MENU_B),
        ]),
    ))
    .unwrap();
    host.bind_context_menu(B, MENU_B).unwrap();
    render(&mut host);
    host.event(&secondary(Point::new(20., 20.)));
    assert!(
        !host
            .get(MENU_B)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    host.remove(front).unwrap();
    render(&mut host);
    host.event(&secondary(Point::new(20., 20.)));
    assert!(
        host.get(MENU_B)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
}

#[test]
fn context_menu_original_owner_ax_show_menu_is_distinct_from_press() {
    let (mut host, _, _) = host();
    assert!(host.can_show_context_menu(B));
    assert!(!host.can_show_context_menu(ROOT));
    let opened = host.accessibility_action(B, None, AccessibilityAction::ShowMenu);
    assert!(opened.consumed && !opened.activated);
    assert_eq!(host.focused(), Some(A));
    assert!(host.drain_actions().is_empty());
    render(&mut host);
    activate(&mut host);
    let selection = selections(&mut host).remove(0);
    assert_eq!(selection.context_request().unwrap().owner(), B);
    assert_eq!(
        selection.context_request().unwrap().source(),
        ContentActionSource::Accessibility
    );
    assert!(
        host.accessibility_action(B, None, AccessibilityAction::Press)
            .activated
    );
    assert_eq!(host.drain_actions()[0].target, B);
}
#[test]
fn context_menu_ax_discovery_and_route_reject_marked_disabled_part_and_remount() {
    let (mut host, _, _) = host();
    assert!(
        !host
            .accessibility_action(B, Some(0), AccessibilityAction::ShowMenu)
            .consumed
    );
    host.get_mut(A)
        .unwrap()
        .component_mut::<Draft>()
        .unwrap()
        .input
        .preedit("中文", 1..1, None);
    assert!(!host.can_show_context_menu(B));
    assert!(
        !host
            .accessibility_action(B, None, AccessibilityAction::ShowMenu)
            .consumed
    );
    assert_eq!(host.focused(), Some(A));
    host.get_mut(A)
        .unwrap()
        .component_mut::<Draft>()
        .unwrap()
        .input
        .unmark();
    host.get_mut(B)
        .unwrap()
        .component_mut::<Button>()
        .unwrap()
        .button
        .state
        .disabled = true;
    assert!(!host.can_show_context_menu(B));
    assert!(
        !host
            .accessibility_action(B, None, AccessibilityAction::ShowMenu)
            .consumed
    );
    host.get_mut(B)
        .unwrap()
        .component_mut::<Button>()
        .unwrap()
        .button
        .state
        .disabled = false;
    let row = host.remove(B).unwrap();
    host.insert(ROOT, row).unwrap();
    render(&mut host);
    assert!(!host.can_show_context_menu(B));
    assert!(
        !host
            .accessibility_action(B, None, AccessibilityAction::ShowMenu)
            .consumed
    );
}

#[test]
fn context_menu_window_margin_primary_click_closes_without_retargeting() {
    let (mut host, _, b) = host();
    let outside = Point::new(-8., -8.);
    for event in [
        ContentEvent::PointerDown(outside),
        ContentEvent::Pointer(PointerEvent {
            position: outside,
            phase: PointerPhase::Down,
            button: Some(PointerButton::Primary),
            modifiers: InputModifiers::default(),
            click_count: 1,
        }),
    ] {
        host.request_context_menu(b, ContentActionSource::Accessibility)
            .unwrap();
        render(&mut host);
        assert!(host.event(&event).consumed);
        assert!(
            !host
                .get(MENU_B)
                .unwrap()
                .component::<MenuContent>()
                .unwrap()
                .menu
                .is_open()
        );
        assert_eq!(host.focused(), Some(A));
        assert!(host.drain_actions().is_empty());
    }
}

#[test]
fn context_menu_control_primary_retargets_original_owner_instead_of_dismissing() {
    let (mut host, _, b) = host();
    host.request_context_menu(b, ContentActionSource::Accessibility)
        .unwrap();
    render(&mut host);
    let point = host.get(A).unwrap().frame().center();
    assert!(
        host.event(&ContentEvent::Pointer(PointerEvent {
            position: point,
            phase: PointerPhase::Down,
            button: Some(PointerButton::Primary),
            modifiers: InputModifiers {
                control: true,
                ..Default::default()
            },
            click_count: 1,
        }))
        .consumed
    );
    render(&mut host);
    assert!(
        host.get(MENU_A)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    assert!(
        !host
            .get(MENU_B)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    assert_eq!(host.focused(), Some(A));
    assert!(host.drain_actions().is_empty());
}
