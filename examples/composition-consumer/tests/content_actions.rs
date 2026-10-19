use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), r.style().line_height)
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
        _: tekes_ui_scene::Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}

use tekes_ui_composition_consumer::content_actions::*;
use tekes_ui_controls::ControlKey;
fn paint(f: &mut ActionFixture) {
    let frame = Rect::new(50., 60., 500., 500.);
    f.host
        .render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
}
fn button(f: &ActionFixture, id: ComponentId) -> &ButtonContent {
    f.host
        .get(id)
        .unwrap()
        .component::<ButtonContent>()
        .unwrap()
}
#[test]
fn pointer_keyboard_and_ax_have_one_concrete_target_and_correct_source() {
    let mut f = ActionFixture::new();
    paint(&mut f);
    let p = f.host.get(FIRST).unwrap().frame().center();
    f.event(&ContentEvent::PointerDown(p));
    assert!(f.delivered.is_empty());
    f.event(&ContentEvent::PointerUp(p));
    assert_eq!(
        f.delivered,
        vec![ContentAction {
            target: FIRST,
            kind: ContentActionKind::Activate,
            source: ContentActionSource::Pointer
        }]
    );
    f.host.focus(Some(SECOND), true);
    for key in [ControlKey::Space, ControlKey::Enter] {
        let n = f.delivered.len();
        f.event(&ContentEvent::Key { key, down: true });
        f.event(&ContentEvent::Key { key, down: false });
        assert_eq!(f.delivered.len(), n + 1);
        assert_eq!(
            f.delivered.last().unwrap(),
            &ContentAction {
                target: SECOND,
                kind: ContentActionKind::Activate,
                source: ContentActionSource::Keyboard
            }
        );
    }
    f.accessibility_action(FIRST, None, AccessibilityAction::Press);
    assert_eq!(
        f.delivered.last().unwrap(),
        &ContentAction {
            target: FIRST,
            kind: ContentActionKind::Activate,
            source: ContentActionSource::Accessibility
        }
    );
    assert_eq!(f.delivered.len(), 4);
    assert!(f.host.drain_actions().is_empty());
}
#[test]
fn disabled_controls_never_emit_and_focus_or_render_never_replays_counters() {
    let mut f = ActionFixture::new();
    paint(&mut f);
    f.accessibility_action(FIRST, None, AccessibilityAction::Press);
    assert_eq!(button(&f, FIRST).activations, 1);
    f.host
        .get_mut(FIRST)
        .unwrap()
        .component_mut::<ButtonContent>()
        .unwrap()
        .button
        .state
        .disabled = true;
    let p = f.host.get(FIRST).unwrap().frame().center();
    f.event(&ContentEvent::PointerDown(p));
    f.event(&ContentEvent::PointerUp(p));
    f.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    f.accessibility_action(FIRST, None, AccessibilityAction::Press);
    paint(&mut f);
    f.host.focus(Some(SECOND), true);
    f.host.focus(Some(FIRST), true);
    assert_eq!(f.delivered.len(), 1);
    assert!(f.host.drain_actions().is_empty());
}
#[test]
fn undrained_actions_are_not_retargeted_to_new_or_retained_remounts() {
    let mut f = ActionFixture::new();
    paint(&mut f);
    f.host
        .accessibility_action(FIRST, None, AccessibilityAction::Press);
    let retained = f.host.remove(FIRST).unwrap();
    f.host.insert(ROOT, retained).unwrap();
    assert!(f.host.drain_actions().is_empty());
    f.host
        .accessibility_action(FIRST, None, AccessibilityAction::Press);
    f.host.remove(FIRST).unwrap();
    f.host
        .insert(ROOT, Content::new(FIRST, ButtonContent::new("new")))
        .unwrap();
    assert!(f.host.drain_actions().is_empty());
    f.host
        .accessibility_action(FIRST, None, AccessibilityAction::Press);
    assert_eq!(f.host.drain_actions().len(), 1);
    assert!(f.host.drain_actions().is_empty());
}
#[test]
fn legacy_hosts_do_not_accumulate_and_opt_in_never_rescans_old_activations() {
    let mut f = ActionFixture::new();
    paint(&mut f);
    f.host.set_action_events(false);
    f.host
        .accessibility_action(FIRST, None, AccessibilityAction::Press);
    assert!(f.host.drain_actions().is_empty());
    f.host.set_action_events(true);
    assert!(f.host.drain_actions().is_empty());
    f.host
        .accessibility_action(FIRST, None, AccessibilityAction::Press);
    f.host.set_action_events(true);
    assert_eq!(f.host.drain_actions().len(), 1);
    f.host
        .accessibility_action(FIRST, None, AccessibilityAction::Press);
    f.host.set_action_events(false);
    f.host.set_action_events(true);
    assert!(f.host.drain_actions().is_empty());
}

#[derive(Debug)]
struct Multiple;
impl Component for Multiple {
    fn interactive(&self) -> bool {
        true
    }
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(a.width, 30.))
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "multiple")
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::PointerMove(_)) {
            ContentResponse::consumed(false)
        } else {
            ContentResponse::default()
        }
    }
    fn action_kinds(&self, result: &ContentResponse) -> Vec<ContentActionKind> {
        if result.consumed {
            vec![ContentActionKind::Submit, ContentActionKind::Activate]
        } else {
            vec![]
        }
    }
}
#[test]
fn nested_multi_action_delivery_is_ordered_and_independent_of_changed_or_activation() {
    use tekes_ui_controls::extended::{Axis, Stack};
    let mut h = ContentHost::new(Content::new(
        ROOT,
        ListContent::new(
            "nested",
            vec![
                Content::new(FIRST, Multiple),
                Content::new(
                    SECOND,
                    StackContent {
                        model: Stack {
                            axis: Axis::Vertical,
                            gap: 0.,
                        },
                        flexible_child: None,
                        children: vec![Content::new(EDITOR, Multiple)],
                    },
                ),
            ],
        ),
    ))
    .unwrap();
    h.set_action_events(true);
    let r = h.event(&ContentEvent::PointerMove(tekes_ui_scene::Point::new(
        0., 0.,
    )));
    assert!(!r.activated);
    let actions = h.drain_actions();
    assert_eq!(
        actions
            .iter()
            .map(|a| (a.target, a.kind))
            .collect::<Vec<_>>(),
        vec![
            (FIRST, ContentActionKind::Submit),
            (FIRST, ContentActionKind::Activate),
            (EDITOR, ContentActionKind::Submit),
            (EDITOR, ContentActionKind::Activate)
        ]
    );
    assert!(
        actions
            .iter()
            .all(|a| a.source == ContentActionSource::Pointer)
    );
    assert!(h.drain_actions().is_empty());
    for a in actions {
        if a.target == FIRST {
            let old = h.remove(FIRST).unwrap();
            h.insert(ROOT, old).unwrap();
        }
        h.render(
            &mut Scene::new(),
            &mut Text,
            Rect::new(0., 0., 300., 300.),
            Rect::new(0., 0., 300., 300.),
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(h.drain_actions().is_empty());
    }
}
fn enter() -> ContentKeyEvent {
    ContentKeyEvent {
        key: Some(ControlKey::Enter),
        down: true,
        repeat: false,
        modifiers: InputModifiers::default(),
        native_code: 36,
        characters: "\r".into(),
        characters_ignoring_modifiers: "\r".into(),
    }
}
#[test]
fn native_key_chain_emits_once_at_actual_dispatch_not_for_each_callback_or_replay() {
    let mut f = ActionFixture::new();
    paint(&mut f);
    f.host.focus(Some(FIRST), true);
    let key = enter();
    f.event(&ContentEvent::TextKeyStart {
        id: 1,
        key: key.clone(),
        composing: false,
    });
    f.event(&ContentEvent::TextCommand("insertNewline:".into()));
    f.event(&ContentEvent::KeyInput(key));
    assert!(f.delivered.is_empty());
    f.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: false,
    });
    f.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: false,
    });
    assert_eq!(
        f.delivered,
        vec![ContentAction {
            target: FIRST,
            kind: ContentActionKind::Activate,
            source: ContentActionSource::Keyboard
        }]
    );
}
#[test]
fn editor_submit_is_typed_and_composition_barriers_do_not_emit_actions() {
    let mut f = ActionFixture::new();
    paint(&mut f);
    f.host.focus(Some(EDITOR), true);
    let mut key = enter();
    key.modifiers.command = true;
    f.event(&ContentEvent::TextKeyStart {
        id: 1,
        key: key.clone(),
        composing: false,
    });
    f.event(&ContentEvent::KeyInput(key.clone()));
    let result = f.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: false,
    });
    assert_eq!(
        result.text_key.unwrap().disposition,
        TextKeyDisposition::Submitted
    );
    assert_eq!(
        f.delivered,
        vec![ContentAction {
            target: EDITOR,
            kind: ContentActionKind::Submit,
            source: ContentActionSource::Keyboard
        }]
    );
    f.event(&ContentEvent::TextKeyStart {
        id: 2,
        key: key.clone(),
        composing: true,
    });
    f.event(&ContentEvent::KeyInput(key));
    f.event(&ContentEvent::TextKeyEnd {
        id: 2,
        input_context_consumed: false,
    });
    assert_eq!(f.delivered.len(), 1);
    f.host
        .get_mut(FIRST)
        .unwrap()
        .component_mut::<ButtonContent>()
        .unwrap()
        .activations = 100;
    f.host.focus(Some(FIRST), true);
    paint(&mut f);
    assert!(f.host.drain_actions().is_empty());
}

#[derive(Debug)]
struct HidesOnAction(bool);
impl Component for HidesOnAction {
    fn active(&self) -> bool {
        self.0
    }
    fn interactive(&self) -> bool {
        true
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "dismiss after activation")
    }
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(a.width, 30.))
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::PointerMove(_)) {
            self.0 = false;
            ContentResponse::consumed(true)
        } else {
            ContentResponse::default()
        }
    }
    fn action_kinds(&self, result: &ContentResponse) -> Vec<ContentActionKind> {
        if result.consumed {
            vec![ContentActionKind::Activate]
        } else {
            vec![]
        }
    }
}
#[test]
fn a_successful_action_survives_hiding_but_not_a_same_id_public_child_replacement() {
    let mut h = ContentHost::new(Content::new(FIRST, HidesOnAction(true))).unwrap();
    h.set_action_events(true);
    h.event(&ContentEvent::PointerMove(tekes_ui_scene::Point::new(
        0., 0.,
    )));
    assert_eq!(
        h.drain_actions(),
        vec![ContentAction {
            target: FIRST,
            kind: ContentActionKind::Activate,
            source: ContentActionSource::Pointer
        }]
    );
    assert!(h.drain_actions().is_empty());
    let mut f = ActionFixture::new();
    paint(&mut f);
    f.host
        .accessibility_action(FIRST, None, AccessibilityAction::Press);
    let children = &mut f
        .host
        .get_mut(ROOT)
        .unwrap()
        .component_mut::<StackContent>()
        .unwrap()
        .children;
    let index = children.iter().position(|c| c.id() == FIRST).unwrap();
    children[index] = Content::new(FIRST, ButtonContent::new("replacement"));
    assert!(f.host.drain_actions().is_empty());
}
#[test]
fn focused_target_changes_never_retarget_or_repeat_already_collected_actions() {
    let mut f = ActionFixture::new();
    paint(&mut f);
    f.host
        .accessibility_action(FIRST, None, AccessibilityAction::Press);
    f.host.focus(Some(SECOND), true);
    f.host
        .accessibility_action(SECOND, None, AccessibilityAction::Press);
    assert_eq!(
        f.host
            .drain_actions()
            .into_iter()
            .map(|a| a.target)
            .collect::<Vec<_>>(),
        vec![FIRST, SECOND]
    );
    f.host.focus(Some(FIRST), true);
    paint(&mut f);
    assert!(f.host.drain_actions().is_empty());
}

#[cfg(feature = "token-audit")]
#[test]
fn typed_delivery_reuses_original_button_input_tokens_for_all_sources_and_themes() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for disabled in [false, true] {
                for source in [
                    ContentActionSource::Pointer,
                    ContentActionSource::Keyboard,
                    ContentActionSource::Accessibility,
                ] {
                    let mut f = ActionFixture::new();
                    f.host
                        .get_mut(FIRST)
                        .unwrap()
                        .component_mut::<ButtonContent>()
                        .unwrap()
                        .button
                        .state
                        .disabled = disabled;
                    let frame = Rect::new(15., 25., 500., 420.);
                    let palette = tekes_ui_tokens::Palette::new(theme, brand).unwrap();
                    f.host
                        .render(&mut Scene::new(), &mut Text, frame, frame, palette)
                        .unwrap();
                    let p = f.host.get(FIRST).unwrap().frame().center();
                    let (scene, lookups) = tekes_ui_tokens::audit::capture(|| {
                        match source {
                            ContentActionSource::Pointer => {
                                f.event(&ContentEvent::PointerDown(p));
                                f.event(&ContentEvent::PointerUp(p));
                            }
                            ContentActionSource::Keyboard => {
                                f.host.focus(Some(FIRST), true);
                                f.event(&ContentEvent::Key {
                                    key: ControlKey::Enter,
                                    down: true,
                                });
                                f.event(&ContentEvent::Key {
                                    key: ControlKey::Enter,
                                    down: false,
                                });
                            }
                            ContentActionSource::Accessibility => {
                                f.accessibility_action(FIRST, None, AccessibilityAction::Press);
                            }
                            _ => unreachable!(),
                        }
                        let mut scene = Scene::new();
                        f.host
                            .render(&mut scene, &mut Text, frame, frame, palette)
                            .unwrap();
                        scene
                    });
                    assert_eq!(f.delivered.len(), usize::from(!disabled));
                    if !disabled {
                        assert_eq!(
                            f.delivered[0],
                            ContentAction {
                                target: FIRST,
                                kind: ContentActionKind::Activate,
                                source
                            }
                        );
                    }
                    assert!(!scene.primitives().is_empty());
                    assert!(lookups.iter().any(|(family, _)| family == "color"));
                    cases.push(serde_json::json!({"component":"tui.button","source":format!("{source:?}"),"disabled":disabled,"theme":format!("{theme:?}"),"brand":brand,"state_asserted":true,"lookups":lookups}));
                }
            }
            let mut f = ActionFixture::new();
            let frame = Rect::new(15., 25., 500., 420.);
            let palette = tekes_ui_tokens::Palette::new(theme, brand).unwrap();
            f.host
                .render(&mut Scene::new(), &mut Text, frame, frame, palette)
                .unwrap();
            f.host.focus(Some(EDITOR), true);
            let (_, lookups) = tekes_ui_tokens::audit::capture(|| {
                let mut key = enter();
                key.modifiers.command = true;
                f.event(&ContentEvent::KeyInput(key));
                f.host
                    .render(&mut Scene::new(), &mut Text, frame, frame, palette)
                    .unwrap();
            });
            assert_eq!(
                f.delivered,
                vec![ContentAction {
                    target: EDITOR,
                    kind: ContentActionKind::Submit,
                    source: ContentActionSource::Keyboard
                }]
            );
            assert!(
                lookups
                    .iter()
                    .any(|(family, id)| family == "color" && id == "transcript-input")
            );
            cases.push(serde_json::json!({"component":"tui.text-field","source":"Keyboard","kind":"Submit","theme":format!("{theme:?}"),"brand":brand,"state_asserted":true,"lookups":lookups}));
        }
    }
    assert_eq!(cases.len(), 84);
    if let Some(path) = std::env::var_os("TEKESUI_ACTION_DELIVERY_TOKEN_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"actual public action collection enabled on original Button/TextField/Stack; keyboard,pointer,logicalAX; enabled/disabled;4themes3brands;CoreText,no-op glyph painter","excluded":["compiled style/font constants","unexecuted paths","physicalIME/VoiceOver","complete dependency extraction"],"cases":cases})).unwrap()).unwrap();
    }
}
