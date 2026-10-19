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

use tekes_ui_controls::{ControlKey, TextSurface};
const EDIT: ComponentId = ComponentId(2);
const OTHER: ComponentId = ComponentId(3);
fn host(multiline: bool) -> ContentHost {
    let mut field = TextFieldContent::new("edit", "");
    field.surface = TextSurface::new("", multiline, false);
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        ListContent::new(
            "fields",
            vec![
                Content::new(EDIT, field),
                Content::new(OTHER, TextFieldContent::new("other", "")),
            ],
        ),
    ))
    .unwrap();
    host.render(
        &mut Scene::new(),
        &mut Text,
        Rect::new(0., 0., 600., 300.),
        Rect::new(0., 0., 600., 300.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(host.focus(Some(EDIT), true));
    host
}
fn field(h: &ContentHost) -> &TextFieldContent {
    h.get(EDIT)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
}
fn field_mut(h: &mut ContentHost) -> &mut TextFieldContent {
    h.get_mut(EDIT)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
}
fn key(key: ControlKey) -> ContentKeyEvent {
    ContentKeyEvent {
        key: Some(key),
        down: true,
        repeat: false,
        modifiers: InputModifiers::default(),
        native_code: 36,
        characters: "\r".into(),
        characters_ignoring_modifiers: "\r".into(),
    }
}
fn start(h: &mut ContentHost, id: u64, key: ContentKeyEvent, composing: bool) {
    h.event(&ContentEvent::TextKeyStart { id, key, composing });
}
fn end(h: &mut ContentHost, id: u64) -> ContentResponse {
    h.event(&ContentEvent::TextKeyEnd {
        id,
        input_context_consumed: false,
    })
}
fn command(h: &mut ContentHost, c: &str) -> ContentResponse {
    h.event(&ContentEvent::TextCommand(c.into()))
}
#[test]
fn ordinary_callbacks_edit_multiline_but_never_invent_submission() {
    for multi in [false, true] {
        let mut h = host(multi);
        field_mut(&mut h).submit_policy = TextSubmitPolicy::Enter;
        assert!(!command(&mut h, "insertNewline:").activated);
        assert!(!command(&mut h, "insertLineBreak:").activated);
        assert_eq!(
            field(&h).surface.input.text,
            if multi { "\n\n" } else { "" }
        );
        if multi {
            assert!(field_mut(&mut h).surface.input.undo());
            assert_eq!(field(&h).surface.input.text, "\n");
        }
    }
}
#[test]
fn one_chain_has_one_newline_even_with_duplicate_selector_and_raw_key() {
    let mut h = host(true);
    let k = key(ControlKey::Enter);
    start(&mut h, 1, k.clone(), false);
    command(&mut h, "insertNewline:");
    command(&mut h, "insertLineBreak:");
    h.event(&ContentEvent::KeyInput(k));
    assert_eq!(field(&h).surface.input.text, "");
    let r = end(&mut h, 1);
    assert_eq!(field(&h).surface.input.text, "\n");
    assert!(!r.activated);
    assert_eq!(
        r.text_key.unwrap().disposition,
        TextKeyDisposition::EditHandled
    );
    assert!(!end(&mut h, 1).activated);
    assert_eq!(field(&h).surface.input.text, "\n");
}
#[test]
fn configured_submit_is_an_intent_only_and_repeat_or_replay_cannot_send_twice() {
    for policy in [TextSubmitPolicy::Enter, TextSubmitPolicy::CommandEnter] {
        let mut h = host(true);
        field_mut(&mut h).submit_policy = policy;
        for id in 1..=2 {
            let mut k = key(ControlKey::Enter);
            k.modifiers.command = policy == TextSubmitPolicy::CommandEnter;
            k.repeat = id == 2;
            start(&mut h, id, k.clone(), false);
            command(&mut h, "insertNewline:");
            h.event(&ContentEvent::KeyInput(k));
            let r = end(&mut h, id);
            assert_eq!(r.activated, id == 1);
            assert_eq!(r.text_key.unwrap().target, Some(EDIT));
            assert_eq!(
                r.text_key.unwrap().disposition,
                if id == 1 {
                    TextKeyDisposition::Submitted
                } else {
                    TextKeyDisposition::EditHandled
                }
            );
            assert!(!end(&mut h, id).activated);
        }
        assert_eq!(field(&h).surface.input.text, "");
    }
}
#[test]
fn commit_unmark_and_control_selectors_never_escape_a_starting_composition() {
    for control in [
        ControlKey::Enter,
        ControlKey::Tab { reverse: false },
        ControlKey::Escape,
    ] {
        for commit_first in [false, true] {
            let mut h = host(true);
            field_mut(&mut h).submit_policy = TextSubmitPolicy::Enter;
            h.event(&ContentEvent::Preedit {
                text: "ni".into(),
                selection: 2..2,
                clauses: std::iter::once(0..2).collect(),
                replacement: None,
            });
            start(&mut h, 1, key(control), true);
            let selector = match control {
                ControlKey::Enter => "insertNewline:",
                ControlKey::Tab { .. } => "insertTab:",
                _ => "cancelOperation:",
            };
            if !commit_first {
                command(&mut h, selector);
            }
            h.event(&ContentEvent::Commit {
                text: "你".into(),
                replacement: None,
            });
            h.event(&ContentEvent::Unmark);
            if commit_first {
                command(&mut h, selector);
            }
            let r = end(&mut h, 1);
            assert!(!r.activated);
            assert_eq!(h.focused(), Some(EDIT));
            assert_eq!(
                r.text_key.unwrap().disposition,
                TextKeyDisposition::InputConsumed
            );
            assert!(!field(&h).surface.input.text.contains('\n'));
        }
    }
}
#[test]
fn tab_insertion_precedes_existing_focus_fallback_and_backtab_policy_is_explicit() {
    let mut h = host(true);
    field_mut(&mut h).tab_policy = TextTabPolicy::Insert;
    for (id, reverse) in [(1, false), (2, true)] {
        start(&mut h, id, key(ControlKey::Tab { reverse }), false);
        command(
            &mut h,
            if reverse {
                "insertBacktab:"
            } else {
                "insertTab:"
            },
        );
        end(&mut h, id);
        assert_eq!(h.focused(), Some(EDIT));
    }
    assert_eq!(field(&h).surface.input.text, "\t");
    field_mut(&mut h).tab_policy = TextTabPolicy::Navigate;
    start(&mut h, 3, key(ControlKey::Tab { reverse: false }), false);
    command(&mut h, "insertTab:");
    let r = end(&mut h, 3);
    assert_eq!(h.focused(), Some(OTHER));
    assert_eq!(
        r.text_key.unwrap().disposition,
        TextKeyDisposition::Navigated
    );
    h.event(&ContentEvent::KeyInput(key(ControlKey::Tab {
        reverse: true,
    })));
    assert_eq!(h.focused(), Some(EDIT));
}
#[test]
fn focus_change_cancels_the_original_chain_and_never_edits_the_new_owner() {
    let mut h = host(true);
    field_mut(&mut h).submit_policy = TextSubmitPolicy::Enter;
    start(&mut h, 1, key(ControlKey::Enter), false);
    assert!(h.focus(Some(OTHER), true));
    h.event(&ContentEvent::Commit {
        text: "late".into(),
        replacement: None,
    });
    command(&mut h, "insertNewline:");
    let r = end(&mut h, 1);
    assert_eq!(
        r.text_key.unwrap().disposition,
        TextKeyDisposition::Cancelled
    );
    assert!(!r.activated);
    assert_eq!(field(&h).surface.input.text, "");
    assert_eq!(h.focused_text().unwrap().input.text, "");
}
#[test]
fn opaque_input_consumption_and_actual_edit_acknowledgement_block_submission() {
    let mut h = host(true);
    field_mut(&mut h).submit_policy = TextSubmitPolicy::Enter;
    start(&mut h, 1, key(ControlKey::Enter), false);
    let r = h.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: true,
    });
    assert!(!r.activated);
    assert_eq!(
        r.text_key.unwrap().disposition,
        TextKeyDisposition::InputConsumed
    );
    start(&mut h, 2, key(ControlKey::Enter), false);
    assert!(command(&mut h, "deleteBackward:").consumed);
    command(&mut h, "insertNewline:");
    let r = end(&mut h, 2);
    assert!(!r.activated);
    assert_eq!(
        r.text_key.unwrap().disposition,
        TextKeyDisposition::EditHandled
    );
}
#[test]
fn modifiers_key_up_shift_return_and_unknown_commands_keep_their_meaning() {
    let mut h = host(true);
    field_mut(&mut h).submit_policy = TextSubmitPolicy::Enter;
    let mut shift = key(ControlKey::Enter);
    shift.modifiers.shift = true;
    start(&mut h, 1, shift, false);
    command(&mut h, "insertLineBreak:");
    let r = end(&mut h, 1);
    assert!(!r.activated);
    assert_eq!(field(&h).surface.input.text, "\n");
    let mut up = key(ControlKey::Enter);
    up.down = false;
    assert!(!h.event(&ContentEvent::KeyInput(up)).activated);
    start(&mut h, 2, key(ControlKey::Enter), false);
    command(&mut h, "unknownSelector:");
    let r = end(&mut h, 2);
    assert!(!r.activated);
    assert_eq!(
        r.text_key.unwrap().disposition,
        TextKeyDisposition::Unhandled
    );
    field_mut(&mut h).surface.input.insert("ab😀", None);
    let mut left = key(ControlKey::Left);
    left.modifiers.shift = true;
    h.event(&ContentEvent::KeyInput(left));
    assert_eq!(field(&h).surface.input.selected_text(), "😀");
}

#[test]
fn returning_focus_to_the_same_id_does_not_resurrect_a_late_native_key() {
    let mut h = host(true);
    field_mut(&mut h).submit_policy = TextSubmitPolicy::Enter;
    start(&mut h, 1, key(ControlKey::Enter), false);
    h.focus(Some(OTHER), true);
    h.focus(Some(EDIT), true);
    h.event(&ContentEvent::Commit {
        text: "late".into(),
        replacement: None,
    });
    command(&mut h, "insertNewline:");
    let r = end(&mut h, 1);
    assert!(!r.activated);
    assert_eq!(
        r.text_key.unwrap().disposition,
        TextKeyDisposition::Cancelled
    );
    assert_eq!(field(&h).surface.input.text, "");
}
#[test]
fn nested_spans_keep_their_own_edit_acknowledgement_and_reject_out_of_order_end_or_replay() {
    let mut h = host(true);
    field_mut(&mut h).submit_policy = TextSubmitPolicy::Enter;
    start(&mut h, 1, key(ControlKey::Enter), false);
    let mut letter = key(ControlKey::Space);
    letter.key = None;
    letter.native_code = 0;
    letter.characters = "a".into();
    start(&mut h, 2, letter, false);
    h.event(&ContentEvent::Commit {
        text: "a".into(),
        replacement: None,
    });
    assert!(end(&mut h, 1).text_key.is_none());
    assert_eq!(
        end(&mut h, 2).text_key.unwrap().disposition,
        TextKeyDisposition::EditHandled
    );
    assert_eq!(
        end(&mut h, 1).text_key.unwrap().disposition,
        TextKeyDisposition::Unhandled
    );
    start(&mut h, 1, key(ControlKey::Enter), false);
    assert!(end(&mut h, 1).text_key.is_none());
    assert_eq!(field(&h).surface.input.text, "a");
}

#[test]
fn multiline_field_slot_contains_at_least_one_unclipped_semantic_text_line() {
    let h = host(true);
    let s = &field(&h).surface;
    assert!(
        s.inner.size.height
            >= s.role
                .style()
                .line_height
                .max(s.paragraph.as_ref().unwrap().lines[0].height as f32),
        "multiline inner={} semantic line={}",
        s.inner.size.height,
        s.role.style().line_height
    );
}

#[test]
fn text_edit_reveals_the_new_caret_after_the_actual_paragraph_is_reshaped() {
    let mut h = host(true);
    h.event(&ContentEvent::Commit {
        text: "one\ntwo\nthree".into(),
        replacement: None,
    });
    let frame = Rect::new(0., 0., 600., 300.);
    h.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    let s = &field(&h).surface;
    let caret = s.caret();
    assert!(
        caret.min_y() >= s.inner.min_y() - 0.01 && caret.max_y() <= s.inner.max_y() + 0.01,
        "new caret {caret:?} must fit {:?}",
        s.inner
    );
}

#[test]
fn a_user_scroll_supersedes_caret_reveal_requested_before_reshaping() {
    let mut h = host(true);
    let position = field(&h).surface.inner.center();
    h.event(&ContentEvent::Commit {
        text: "one\ntwo\nthree\nfour".into(),
        replacement: None,
    });
    assert!(
        h.event(&ContentEvent::Scroll {
            position,
            delta: tekes_ui_scene::Point::new(0., 0.),
            precise: true,
            begin: true,
            end: false,
            momentum: false,
            time: 1.
        })
        .consumed
    );
    let frame = Rect::new(0., 0., 600., 300.);
    h.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert_eq!(field(&h).surface.scroll.offset.y, 0.);
}
#[test]
fn multiline_field_measure_arrange_and_paint_share_the_same_public_slot_geometry() {
    let mut h = host(true);
    field_mut(&mut h).field.help = "Helper stays below the editor".into();
    let frame = Rect::new(0., 0., 600., 300.);
    h.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    let f = field(&h);
    assert_eq!(f.view.layout().slot, f.surface.outer);
    assert!(f.view.layout().message.min_y() >= f.surface.outer.max_y());
}

#[test]
fn the_original_text_surface_reveal_contract_survives_stale_shape_in_both_scroll_axes() {
    for multiline in [false, true] {
        let mut surface = TextSurface::new("", multiline, false);
        let frame = Rect::new(0., 0., 80., surface.minimum_outer_height(1.));
        let role = surface.role;
        let shape =
            |s: &str, w| tekes_ui_controls::TextSurfaceHost::shape_surface(&mut Text, s, role, w);
        surface.layout_with(frame, Some(frame), 1., 1., shape);
        surface.input.insert(
            if multiline {
                "one\ntwo\nthree\nfour"
            } else {
                "a very long single-line editor value with emoji 😀"
            },
            None,
        );
        surface.reveal();
        surface.layout_with(frame, Some(frame), 1., 1., shape);
        let caret = surface.caret();
        assert!(
            caret.min_x() >= surface.inner.min_x() - 0.01
                && caret.max_x() <= surface.inner.max_x() + 0.01,
            "{caret:?} {:?}",
            surface.inner
        );
        assert!(
            caret.min_y() >= surface.inner.min_y() - 0.01
                && caret.max_y() <= surface.inner.max_y() + 0.01,
            "{caret:?} {:?}",
            surface.inner
        );
    }
}

#[cfg(feature = "token-audit")]
#[test]
fn actual_public_editor_policies_keep_shared_surfaces_and_record_bounded_token_reads() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for multiline in [false, true] {
                for policy in ["newline", "tab", "submit"] {
                    let mut f = TextFieldContent::new("Editor", "");
                    f.surface = TextSurface::new("", multiline, false);
                    f.tab_policy = TextTabPolicy::Insert;
                    f.submit_policy = TextSubmitPolicy::CommandEnter;
                    let mut h = ContentHost::new(Content::new(EDIT, f)).unwrap();
                    let frame = Rect::new(0., 0., 400., 160.);
                    h.render(
                        &mut Scene::new(),
                        &mut Text,
                        frame,
                        frame,
                        tekes_ui_tokens::Palette::new(theme, brand).unwrap(),
                    )
                    .unwrap();
                    h.focus(Some(EDIT), true);
                    let mut k = key(if policy == "tab" {
                        ControlKey::Tab { reverse: false }
                    } else {
                        ControlKey::Enter
                    });
                    k.modifiers.command = policy == "submit";
                    start(&mut h, 1, k.clone(), false);
                    if policy == "submit" {
                        h.event(&ContentEvent::KeyInput(k));
                    } else {
                        command(
                            &mut h,
                            if policy == "tab" {
                                "insertTab:"
                            } else {
                                "insertNewline:"
                            },
                        );
                    }
                    let result = end(&mut h, 1);
                    assert_eq!(result.activated, policy == "submit");
                    let expected = if multiline && policy == "newline" {
                        "\n"
                    } else if multiline && policy == "tab" {
                        "\t"
                    } else {
                        ""
                    };
                    assert_eq!(field(&h).surface.input.text, expected);
                    let (scene, lookups) = tekes_ui_tokens::audit::capture(|| {
                        let mut scene = Scene::new();
                        h.render(
                            &mut scene,
                            &mut Text,
                            frame,
                            frame,
                            tekes_ui_tokens::Palette::new(theme, brand).unwrap(),
                        )
                        .unwrap();
                        scene
                    });
                    let f = field(&h);
                    assert!(scene.primitives().iter().any(|p|matches!(p,tekes_ui_scene::Primitive::Quad(q) if q.rect==f.surface.outer)));
                    assert!(
                        lookups
                            .iter()
                            .any(|(family, id)| family == "color" && id == "transcript-input")
                    );
                    assert!(f.surface.caret().max_y() <= f.surface.inner.max_y() + 0.01);
                    cases.push(serde_json::json!({"component":"tui.text-field","mode":if multiline {"multiline"}else{"single-line"},"policy":policy,"theme":format!("{theme:?}"),"brand":brand,"state_asserted":true,"lookups":lookups}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 72);
    if let Some(path) = std::env::var_os("TEKESUI_EDITOR_TOKEN_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"isolated actual public TextFieldContent single/multiline editors and original TextSurface; newline, Tab and submit policies; four themes, three existing brands; CoreText; no-op glyph painter; no new registry component","excluded":["compiled semantic font styles/constants","unexecuted paths","physical IME/VoiceOver","complete dependency extraction"],"cases":cases})).unwrap()).unwrap();
    }
}
