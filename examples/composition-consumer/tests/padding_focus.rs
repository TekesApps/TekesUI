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

use tekes_ui_composition_consumer::padding_focus::*;
use tekes_ui_controls::ControlKey;
use tekes_ui_scene::Point;
fn paint(h: &mut ContentHost) {
    let f = Rect::new(20., 30., 600., 260.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
}
fn setup() -> ContentHost {
    let mut h = build().unwrap();
    paint(&mut h);
    h
}
fn padding(h: &ContentHost) -> Point {
    let f = h.get(BAND).unwrap().frame();
    let p = Point::new(f.min_x() + 1., f.center().y);
    assert!(!h.get(EDITOR).unwrap().frame().contains(p));
    p
}
fn click(h: &mut ContentHost, p: Point) {
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::PointerUp(p));
}
fn input(h: &mut ContentHost) -> &mut TextFieldContent {
    h.get_mut(EDITOR).unwrap().component_mut().unwrap()
}
#[test]
fn padding_focuses_original_editor_without_capture_or_synthetic_caret_mapping() {
    let mut h = setup();
    let before = input(&mut h).surface.input.external_baseline();
    let p = padding(&h);
    assert!(h.event(&ContentEvent::PointerDown(p)).consumed);
    assert_eq!(h.focused(), Some(EDITOR));
    h.event(&ContentEvent::PointerMove(Point::new(500., 200.)));
    h.event(&ContentEvent::PointerUp(Point::new(500., 200.)));
    assert_eq!(h.focused_text().unwrap().input.external_baseline(), before);
    assert!(h.drain_actions().is_empty());
}
#[test]
fn already_focused_padding_preserves_preedit_selection_and_undo() {
    for composing in [false, true] {
        let mut h = setup();
        h.focus(Some(EDITOR), true);
        h.event(&ContentEvent::Commit {
            text: "retained".into(),
            replacement: None,
        });
        input(&mut h).surface.input.select(2, false);
        input(&mut h).surface.input.select(1, true);
        if composing {
            h.event(&ContentEvent::Preedit {
                text: "ni".into(),
                selection: 0..2,
                clauses: vec![],
                replacement: None,
            });
        }
        let before = h.focused_text().unwrap().input.external_baseline();
        let p = padding(&h);
        click(&mut h, p);
        assert_eq!(h.focused_text().unwrap().input.external_baseline(), before);
        if !composing {
            h.event(&ContentEvent::TextCommand("undo:".into()));
            assert_eq!(h.focused_text().unwrap().input.text, "Draft A😀B");
        }
    }
}
#[test]
fn actual_button_wins_including_disabled_unhandled_bounds() {
    let mut h = setup();
    let p = h.get(ACTION).unwrap().frame().center();
    click(&mut h, p);
    assert_eq!(h.focused(), Some(ACTION));
    let a = h.drain_actions();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].target, ACTION);
    h.focus(None, false);
    h.get_mut(ACTION)
        .unwrap()
        .component_mut::<ButtonContent>()
        .unwrap()
        .button
        .state
        .disabled = true;
    click(&mut h, p);
    assert_eq!(h.focused(), None);
    assert!(h.drain_actions().is_empty());
}
#[test]
fn readonly_disabled_hidden_and_parent_disabled_reject_redirect() {
    for state in 0..4 {
        let mut h = setup();
        let p = padding(&h);
        match state {
            0 => input(&mut h).surface.input.read_only = true,
            1 => input(&mut h).surface.input.disabled = true,
            2 => {
                h.get_mut(BAND)
                    .unwrap()
                    .component_mut::<InputBand>()
                    .unwrap()
                    .shown = false
            }
            _ => {
                h.get_mut(BAND)
                    .unwrap()
                    .component_mut::<InputBand>()
                    .unwrap()
                    .disabled = true
            }
        }
        click(&mut h, p);
        assert_eq!(h.focused(), None, "state {state}");
    }
}
#[test]
fn same_numeric_id_replacement_and_remount_need_explicit_new_declaration() {
    for replacement in [false, true] {
        let mut h = setup();
        let p = padding(&h);
        let old = h.remove(EDITOR).unwrap();
        let child = if replacement {
            Content::new(EDITOR, TextFieldContent::new("Other document", "other"))
        } else {
            old
        };
        h.insert(BAND, child).unwrap();
        paint(&mut h);
        click(&mut h, p);
        assert_eq!(h.focused(), None);
        h.set_padding_focus(BAND, Some(EDITOR)).unwrap();
        click(&mut h, p);
        assert_eq!(h.focused(), Some(EDITOR));
    }
}
#[test]
fn declaration_rejects_foreign_self_and_non_editor_and_can_be_cleared() {
    let mut h = setup();
    for id in [BAND, OTHER, ACTION, ComponentId(999999)] {
        assert!(matches!(
            h.set_padding_focus(BAND, Some(id)),
            Err(CompositionError::InvalidPaddingFocus { .. })
        ));
    }
    h.set_padding_focus(BAND, None).unwrap();
    let p = padding(&h);
    click(&mut h, p);
    assert_eq!(h.focused(), None);
}
#[test]
fn tab_reverse_tab_and_ax_retain_original_nodes_only() {
    let mut h = setup();
    let targets = h.focus_targets();
    assert_eq!(
        targets.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec![EDITOR, ACTION, OTHER]
    );
    let nodes = h.semantics();
    assert_eq!(nodes.iter().filter(|n| n.id == EDITOR).count(), 1);
    h.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    assert_eq!(h.focused(), Some(EDITOR));
    h.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    assert_eq!(h.focused(), Some(ACTION));
    h.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: true },
        down: true,
    });
    assert_eq!(h.focused(), Some(EDITOR));
    h.focus(None, false);
    assert!(
        h.accessibility_action(EDITOR, None, AccessibilityAction::Focus)
            .consumed
    );
    assert_eq!(h.focused(), Some(EDITOR));
}
#[test]
fn existing_editor_drag_capture_is_not_stolen_by_padding() {
    let mut h = setup();
    let p = h.text_surface(EDITOR).unwrap().outer.center();
    h.event(&ContentEvent::PointerDown(p));
    let pad = padding(&h);
    h.event(&ContentEvent::PointerMove(pad));
    assert_eq!(h.focused(), Some(EDITOR));
    h.event(&ContentEvent::PointerUp(pad));
    assert!(h.drain_actions().is_empty());
}
#[test]
fn parent_remove_reinsert_invalidates_descendant_binding() {
    let mut h = setup();
    let p = padding(&h);
    let band = h.remove(BAND).unwrap();
    h.insert(ROOT, band).unwrap();
    paint(&mut h);
    let _ = p;
    let p = padding(&h);
    click(&mut h, p);
    assert_eq!(h.focused(), None);
}
#[test]
fn clipped_padding_and_secondary_pointer_do_not_change_focus() {
    let mut h = setup();
    let p = padding(&h);
    h.event(&ContentEvent::Pointer(PointerEvent {
        position: p,
        phase: PointerPhase::Down,
        button: Some(PointerButton::Secondary),
        modifiers: Default::default(),
        click_count: 1,
    }));
    assert_eq!(h.focused(), None);
    let f = Rect::new(20., 30., 600., 260.);
    h.render(
        &mut Scene::new(),
        &mut Text,
        f,
        Rect::new(40., 30., 580., 260.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    click(&mut h, p);
    assert_eq!(h.focused(), None);
}
#[test]
fn padding_focus_is_atomic_for_original_key_chain_and_repeated_click_preserves_it() {
    let mut h = setup();
    h.focus(Some(EDITOR), true);
    h.event(&ContentEvent::TextKeyStart {
        id: 1,
        composing: true,
        key: ContentKeyEvent {
            key: None,
            down: true,
            repeat: false,
            modifiers: Default::default(),
            native_code: 45,
            characters: "n".into(),
            characters_ignoring_modifiers: "n".into(),
        },
    });
    let p = padding(&h);
    click(&mut h, p);
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: vec![],
        replacement: None,
    });
    let result = h.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: false,
    });
    let key = result.text_key.unwrap();
    assert_eq!(key.target, Some(EDITOR));
    assert_ne!(key.disposition, TextKeyDisposition::Cancelled);
    assert!(h.focused_text().unwrap().input.marked.is_some());
}
#[test]
fn old_editor_key_chain_cannot_leak_across_atomic_padding_focus_switch() {
    let mut h = setup();
    h.focus(Some(OTHER), true);
    h.event(&ContentEvent::TextKeyStart {
        id: 1,
        composing: false,
        key: ContentKeyEvent {
            key: None,
            down: true,
            repeat: false,
            modifiers: Default::default(),
            native_code: 45,
            characters: "n".into(),
            characters_ignoring_modifiers: "n".into(),
        },
    });
    let before = input(&mut h).surface.input.external_baseline();
    let p = padding(&h);
    click(&mut h, p);
    assert_eq!(h.focused(), Some(EDITOR));
    h.event(&ContentEvent::Commit {
        text: "late".into(),
        replacement: None,
    });
    let result = h.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: false,
    });
    assert_eq!(
        result.text_key.unwrap().disposition,
        TextKeyDisposition::Cancelled
    );
    assert_eq!(h.focused_text().unwrap().input.external_baseline(), before);
}
#[test]
fn visible_container_with_fully_clipped_text_does_not_focus_invisible_editor() {
    let mut h = setup();
    let outer = h.text_surface(EDITOR).unwrap().outer;
    let f = Rect::new(20., 30., 600., 260.);
    let clip = Rect::new(
        f.min_x(),
        outer.max_y() + 1.,
        f.size.width,
        f.max_y() - outer.max_y() - 1.,
    );
    h.render(&mut Scene::new(), &mut Text, f, clip, Theme::LIGHT.into())
        .unwrap();
    let p = padding(&h);
    assert!(clip.contains(p));
    assert!(h.get(EDITOR).unwrap().visible().is_some());
    assert!(
        h.text_surface(EDITOR)
            .unwrap()
            .outer
            .intersection(&clip)
            .is_none()
    );
    assert_eq!(h.padding_focus_target(&ContentEvent::PointerDown(p)), None);
    click(&mut h, p);
    assert_eq!(h.focused(), None);
}

#[test]
fn padding_intent_is_read_only_and_dispatch_resolves_the_same_original_editor() {
    let mut h = setup();
    let e = ContentEvent::PointerDown(padding(&h));
    assert_eq!(h.padding_focus_target(&e), Some(EDITOR));
    assert_eq!(h.focused(), None);
    assert!(h.drain_actions().is_empty());
    h.event(&e);
    assert_eq!(h.focused(), Some(EDITOR));
    h.event(&ContentEvent::Commit {
        text: "retained".into(),
        replacement: None,
    });
    input(&mut h).surface.input.select(2, false);
    input(&mut h).surface.input.select(1, true);
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: vec![],
        replacement: None,
    });
    let baseline = h.focused_text().unwrap().input.external_baseline();
    let focus = h.capture_focus();
    let semantics = format!("{:?}", h.semantics());
    for _ in 0..3 {
        assert_eq!(h.padding_focus_target(&e), Some(EDITOR));
    }
    assert_eq!(h.capture_focus(), focus);
    assert_eq!(
        h.focused_text().unwrap().input.external_baseline(),
        baseline
    );
    assert_eq!(format!("{:?}", h.semantics()), semantics);
    assert!(h.drain_actions().is_empty());
    assert!(!h.event(&e).changed);
    assert_eq!(
        h.focused_text().unwrap().input.external_baseline(),
        baseline
    );
}

#[test]
fn padding_intent_rejects_real_actions_and_non_down_pointer_phases() {
    let mut h = setup();
    let p = padding(&h);
    for phase in [
        PointerPhase::Up,
        PointerPhase::Move,
        PointerPhase::Drag,
        PointerPhase::Enter,
        PointerPhase::Exit,
    ] {
        let event = ContentEvent::Pointer(PointerEvent {
            position: p,
            phase,
            button: Some(PointerButton::Primary),
            modifiers: Default::default(),
            click_count: 1,
        });
        assert_eq!(h.padding_focus_target(&event), None);
    }
    for button in [
        None,
        Some(PointerButton::Secondary),
        Some(PointerButton::Middle),
        Some(PointerButton::Other(4)),
    ] {
        let event = ContentEvent::Pointer(PointerEvent {
            position: p,
            phase: PointerPhase::Down,
            button,
            modifiers: Default::default(),
            click_count: 1,
        });
        assert_eq!(h.padding_focus_target(&event), None);
    }
    assert_eq!(h.padding_focus_target(&ContentEvent::PointerUp(p)), None);
    assert_eq!(
        h.padding_focus_target(&ContentEvent::PointerDown(Point::new(-1., -1.))),
        None
    );
    for state in 0..3 {
        let b = &mut h
            .get_mut(ACTION)
            .unwrap()
            .component_mut::<ButtonContent>()
            .unwrap()
            .button;
        b.state.disabled = state == 1;
        b.state.loading = state == 2;
        let event = ContentEvent::PointerDown(h.get(ACTION).unwrap().frame().center());
        assert_eq!(
            h.padding_focus_target(&event),
            None,
            "real action state {state}"
        );
    }
    let event = ContentEvent::Pointer(PointerEvent {
        position: p,
        phase: PointerPhase::Down,
        button: Some(PointerButton::Primary),
        modifiers: Default::default(),
        click_count: 1,
    });
    assert_eq!(h.padding_focus_target(&event), Some(EDITOR));
    assert!(h.event(&event).consumed);
    assert_eq!(h.focused(), Some(EDITOR));
}

#[test]
fn padding_intent_obeys_live_capture_and_never_acquires_its_own() {
    let mut h = setup();
    let p = padding(&h);
    let event = ContentEvent::PointerDown(p);
    let editor = h.text_surface(EDITOR).unwrap().outer.center();
    assert!(h.event(&ContentEvent::PointerDown(editor)).capture_pointer);
    assert_eq!(h.padding_focus_target(&event), None);
    h.event(&ContentEvent::PointerUp(editor));
    assert_eq!(h.padding_focus_target(&event), Some(EDITOR));
    // Querying repeatedly must not capture the original editor.
    assert_eq!(h.padding_focus_target(&event), Some(EDITOR));
    let action = h.get(ACTION).unwrap().frame().center();
    click(&mut h, action);
    assert_eq!(h.drain_actions().len(), 1);
}

#[test]
fn padding_intent_dispatch_rechecks_mutated_availability_and_mount() {
    for mutation in 0..7 {
        let mut h = setup();
        let event = ContentEvent::PointerDown(padding(&h));
        assert_eq!(h.padding_focus_target(&event), Some(EDITOR));
        match mutation {
            0 => input(&mut h).surface.input.read_only = true,
            1 => input(&mut h).surface.input.disabled = true,
            2 => {
                h.get_mut(BAND)
                    .unwrap()
                    .component_mut::<InputBand>()
                    .unwrap()
                    .shown = false
            }
            3 => {
                h.get_mut(BAND)
                    .unwrap()
                    .component_mut::<InputBand>()
                    .unwrap()
                    .disabled = true
            }
            4 => h.set_padding_focus(BAND, None).unwrap(),
            _ => {
                let old = h.remove(EDITOR).unwrap();
                let node = if mutation == 5 {
                    old
                } else {
                    Content::new(EDITOR, TextFieldContent::new("Replacement", "new"))
                };
                h.insert(BAND, node).unwrap();
                paint(&mut h);
            }
        }
        assert_eq!(h.padding_focus_target(&event), None, "mutation {mutation}");
        h.event(&event);
        assert_eq!(h.focused(), None, "mutation {mutation}");
        assert!(h.drain_actions().is_empty());
    }
}

#[test]
fn padding_intent_preserves_original_key_epoch_and_undo_chain() {
    let mut h = setup();
    h.focus(Some(EDITOR), true);
    h.event(&ContentEvent::Commit {
        text: "retained".into(),
        replacement: None,
    });
    h.event(&ContentEvent::TextKeyStart {
        id: 42,
        composing: true,
        key: ContentKeyEvent {
            key: None,
            down: true,
            repeat: false,
            modifiers: Default::default(),
            native_code: 45,
            characters: "n".into(),
            characters_ignoring_modifiers: "n".into(),
        },
    });
    let event = ContentEvent::PointerDown(padding(&h));
    assert_eq!(h.padding_focus_target(&event), Some(EDITOR));
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: vec![],
        replacement: None,
    });
    let before = h.focused_text().unwrap().input.external_baseline();
    assert_eq!(h.padding_focus_target(&event), Some(EDITOR));
    let out = h
        .event(&ContentEvent::TextKeyEnd {
            id: 42,
            input_context_consumed: false,
        })
        .text_key
        .unwrap();
    assert_eq!(out.target, Some(EDITOR));
    assert_ne!(out.disposition, TextKeyDisposition::Cancelled);
    assert_eq!(h.focused_text().unwrap().input.external_baseline(), before);
    h.event(&ContentEvent::Commit {
        text: "你".into(),
        replacement: None,
    });
    h.event(&ContentEvent::TextCommand("undo:".into()));
    assert!(h.focused_text().unwrap().input.text.contains("retained"));
}

#[test]
fn padding_intent_yields_to_real_menu_and_modal_scope() {
    let mut h = setup();
    let event = ContentEvent::PointerDown(padding(&h));
    let menu_id = ComponentId(14900);
    h.insert(
        ROOT,
        Content::new(menu_id, MenuContent::new("Options", vec!["First".into()])),
    )
    .unwrap();
    h.get_mut(menu_id)
        .unwrap()
        .component_mut::<MenuContent>()
        .unwrap()
        .menu
        .show(0);
    paint(&mut h);
    assert_eq!(h.padding_focus_target(&event), None);
    h.event(&event);
    assert_ne!(h.focused(), Some(EDITOR));
    h.remove(menu_id).unwrap();
    let modal_id = ComponentId(14910);
    let slot = |id| Content::new(ComponentId(id), ButtonContent::new("Modal slot"));
    h.insert(
        ROOT,
        Content::new(
            modal_id,
            DialogContent::new("Modal", slot(14911), slot(14912), slot(14913)),
        ),
    )
    .unwrap();
    h.show_dialog(modal_id).unwrap();
    paint(&mut h);
    assert_eq!(h.padding_focus_target(&event), None);
    h.event(&event);
    assert_ne!(h.focused(), Some(EDITOR));
    h.close_dialog();
    paint(&mut h);
    assert_eq!(h.padding_focus_target(&event), Some(EDITOR));
}

#[test]
fn unconsumed_overlay_cannot_leak_into_registered_padding() {
    #[derive(Debug)]
    struct OpenOverlay;
    impl Component for OpenOverlay {
        fn overlay_open(&self) -> bool {
            true
        }
        fn measure(
            &mut self,
            _: &mut dyn ButtonTextRenderer,
            _: Size,
        ) -> tekes_ui_controls::layout::MeasuredChild {
            tekes_ui_controls::layout::MeasuredChild::new(Size::new(1., 1.))
        }
        fn paint(
            &mut self,
            _: &mut Scene,
            _: &mut dyn ButtonTextRenderer,
            _: Rect,
            _: Rect,
            _: ContentTheme,
        ) {
        }
    }
    let mut h = setup();
    let e = ContentEvent::PointerDown(padding(&h));
    h.insert(ROOT, Content::new(ComponentId(14990), OpenOverlay))
        .unwrap();
    paint(&mut h);
    assert_eq!(h.padding_focus_target(&e), None);
    h.event(&e);
    assert_eq!(h.focused(), None);
    assert!(h.drain_actions().is_empty());
}

#[test]
fn padding_inside_current_modal_uses_the_original_modal_editor() {
    let mut h = setup();
    let band = h.remove(BAND).unwrap();
    let modal = ComponentId(14920);
    h.insert(
        ROOT,
        Content::new(
            modal,
            DialogContent::new(
                "Edit",
                Content::new(ComponentId(14921), ButtonContent::new("Header")),
                band,
                Content::new(ComponentId(14922), ButtonContent::new("Done")),
            ),
        ),
    )
    .unwrap();
    h.show_dialog(modal).unwrap();
    h.set_padding_focus(BAND, Some(EDITOR)).unwrap();
    paint(&mut h);
    let event = ContentEvent::PointerDown(padding(&h));
    assert_eq!(h.padding_focus_target(&event), Some(EDITOR));
    h.event(&event);
    assert_eq!(h.focused(), Some(EDITOR));
}
