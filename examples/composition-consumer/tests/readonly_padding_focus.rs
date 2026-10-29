//! Additive readonly consumer, with original CoreText/TextField selection and host intent.
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::padding_focus::*;
use tekes_ui_controls::{ControlKey, composition::*};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::Theme;

fn paint(host: &mut ContentHost) {
    let frame = Rect::new(20., 30., 600., 260.);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn setup() -> ContentHost {
    let mut host = build_read_only().unwrap();
    paint(&mut host);
    host
}
fn field(host: &mut ContentHost) -> &mut TextFieldContent {
    host.get_mut(EDITOR)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
}
fn edges(host: &ContentHost) -> [Point; 4] {
    let band = host.get(BAND).unwrap().frame();
    [
        Point::new(band.min_x() + 1., band.center().y),
        Point::new(band.max_x() - 1., band.center().y),
        Point::new(band.center().x, band.min_y() + 1.),
        Point::new(band.center().x, band.max_y() - 1.),
    ]
}
fn click(host: &mut ContentHost, point: Point) -> ContentResponse {
    let response = host.event(&ContentEvent::PointerDown(point));
    host.event(&ContentEvent::PointerUp(point));
    response
}
#[test]
fn legacy_api_rejects_readonly_and_new_opt_in_registers_the_same_retained_owner() {
    let mut host = setup();
    assert_eq!(
        host.set_padding_focus(BAND, Some(EDITOR)),
        Err(CompositionError::InvalidPaddingFocus {
            owner: BAND,
            target: EDITOR
        })
    );
    // Failed old-API registration cannot overwrite the existing explicit opt-in.
    assert_eq!(
        host.padding_focus_target(&ContentEvent::PointerDown(edges(&host)[0])),
        Some(EDITOR)
    );
    host.set_padding_text_focus(BAND, None).unwrap();
    assert_eq!(
        host.padding_focus_target(&ContentEvent::PointerDown(edges(&host)[0])),
        None
    );
    host.set_padding_text_focus(BAND, Some(EDITOR)).unwrap();
    for invalid in [BAND, ACTION, OTHER, ComponentId(999_999)] {
        assert_eq!(
            host.set_padding_text_focus(BAND, Some(invalid)),
            Err(CompositionError::InvalidPaddingFocus {
                owner: BAND,
                target: invalid
            })
        );
    }
    assert_eq!(
        host.padding_focus_target(&ContentEvent::PointerDown(edges(&host)[0])),
        Some(EDITOR)
    );
    // The additive API can also serve existing editable text, without changing the old API.
    let mut editable = build().unwrap();
    editable.set_padding_text_focus(BAND, Some(EDITOR)).unwrap();
}
#[test]
fn all_four_edges_only_focus_and_preserve_original_selection_and_readonly_state() {
    for edge in 0..4 {
        let mut host = setup();
        let mount = host.semantic_mount(EDITOR).unwrap();
        field(&mut host).surface.input.select_range(1..3);
        let before = field(&mut host).surface.input.external_baseline();
        let point = edges(&host)[edge];
        assert!(!host.get(EDITOR).unwrap().frame().contains(point));
        let event = ContentEvent::PointerDown(point);
        assert_eq!(host.padding_focus_target(&event), Some(EDITOR));
        assert_eq!(
            field(&mut host).surface.input.external_baseline(),
            before,
            "query is pure"
        );
        let response = click(&mut host, point);
        assert!(response.consumed && !response.capture_pointer);
        assert_eq!(host.focused(), Some(EDITOR));
        assert_eq!(host.semantic_mount(EDITOR).unwrap(), mount);
        assert_eq!(field(&mut host).surface.input.external_baseline(), before);
        assert!(field(&mut host).surface.input.read_only);
        for point in edges(&host) {
            assert!(!click(&mut host, point).capture_pointer);
            assert_eq!(field(&mut host).surface.input.external_baseline(), before);
        }
        assert!(host.drain_actions().is_empty());
    }
}
#[test]
fn direct_readonly_pointer_selection_stays_original_and_cannot_edit_or_mark() {
    let mut host = setup();
    let inner = host.text_surface(EDITOR).unwrap().inner;
    let from = Point::new(inner.max_x() - 2., inner.center().y);
    let to = Point::new(inner.min_x() + 2., inner.center().y);
    assert_eq!(
        host.padding_focus_target(&ContentEvent::PointerDown(from)),
        None
    );
    assert!(host.event(&ContentEvent::PointerDown(from)).capture_pointer);
    host.event(&ContentEvent::PointerMove(to));
    host.event(&ContentEvent::PointerUp(to));
    assert_eq!(host.focused(), Some(EDITOR));
    assert!(
        !host
            .text_surface(EDITOR)
            .unwrap()
            .input
            .selection()
            .is_empty()
    );
    assert_eq!(
        host.text_surface(EDITOR).unwrap().input.text,
        READ_ONLY_TEXT
    );
    let before = host.text_surface(EDITOR).unwrap().input.external_baseline();
    for event in [
        ContentEvent::Commit {
            text: "forbidden".into(),
            replacement: None,
        },
        ContentEvent::Preedit {
            text: "ni".into(),
            selection: 0..2,
            clauses: vec![],
            replacement: None,
        },
        ContentEvent::TextCommand("deleteBackward:".into()),
    ] {
        host.event(&event);
        assert_eq!(
            host.text_surface(EDITOR).unwrap().input.external_baseline(),
            before
        );
    }
    assert!(
        !host
            .accessibility_action(
                EDITOR,
                None,
                AccessibilityAction::ReplaceSelectedText("forbidden".into())
            )
            .changed
    );
    assert_eq!(
        host.text_surface(EDITOR).unwrap().input.external_baseline(),
        before
    );
    assert!(host.text_surface(EDITOR).unwrap().input.marked.is_none());
}
#[test]
fn original_sibling_action_wins_even_when_disabled_and_unconsumed() {
    let mut host = setup();
    let point = host.get(ACTION).unwrap().frame().center();
    assert_eq!(
        host.padding_focus_target(&ContentEvent::PointerDown(point)),
        None
    );
    click(&mut host, point);
    let records = host.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.target, ACTION);
    host.focus(None, false);
    host.get_mut(ACTION)
        .unwrap()
        .component_mut::<Button>()
        .unwrap()
        .button
        .state
        .disabled = true;
    click(&mut host, point);
    assert_eq!(host.focused(), None);
    assert!(host.drain_actions().is_empty());
}
#[test]
fn unavailable_or_stale_mounts_and_foreign_subtrees_refuse_the_original_declaration() {
    for mutation in 0..8 {
        let mut host = setup();
        let point = edges(&host)[0];
        match mutation {
            0 => field(&mut host).surface.input.disabled = true,
            1 => {
                host.get_mut(BAND)
                    .unwrap()
                    .component_mut::<InputBand>()
                    .unwrap()
                    .shown = false
            }
            2 => {
                host.get_mut(BAND)
                    .unwrap()
                    .component_mut::<InputBand>()
                    .unwrap()
                    .disabled = true
            }
            3 | 4 => {
                let previous = host.remove(EDITOR).unwrap();
                let target = if mutation == 3 {
                    Content::new(
                        EDITOR,
                        TextFieldContent::read_only_content(
                            "New document",
                            "new document",
                            tekes_ui_tokens::proposal::TextRole::UiBody,
                        ),
                    )
                } else {
                    previous
                };
                host.insert(BAND, target).unwrap();
            }
            5 => {
                let band = host.remove(BAND).unwrap();
                host.insert(ROOT, band).unwrap();
            }
            6 => {
                let target = host.remove(EDITOR).unwrap();
                host.insert(ROOT, target).unwrap();
            }
            _ => {
                let frame = Rect::new(20., 30., 600., 260.);
                let mut text = coretext::Text::new(1.);
                text.services = true;
                host.render(
                    &mut Scene::new(),
                    &mut text,
                    frame,
                    Rect::new(20., 30., 10., 260.),
                    Theme::LIGHT.into(),
                )
                .unwrap();
            }
        }
        assert_eq!(
            host.padding_focus_target(&ContentEvent::PointerDown(point)),
            None,
            "mutation={mutation}"
        );
        click(&mut host, point);
        assert_eq!(host.focused(), None, "mutation={mutation}");
        assert!(host.drain_actions().is_empty());
        if matches!(mutation, 3..=5) {
            host.set_padding_text_focus(BAND, Some(EDITOR)).unwrap();
            paint(&mut host);
            let point = edges(&host)[0];
            click(&mut host, point);
            assert_eq!(host.focused(), Some(EDITOR));
        } else if mutation < 7 {
            assert!(host.set_padding_text_focus(BAND, Some(EDITOR)).is_err());
        }
    }
}
#[test]
fn original_tab_ax_and_capture_do_not_create_another_text_or_input_owner() {
    let mut host = setup();
    assert_eq!(
        host.focus_targets()
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        vec![EDITOR, ACTION, OTHER]
    );
    assert_eq!(
        host.semantics()
            .iter()
            .filter(|node| node.id == EDITOR)
            .count(),
        1
    );
    host.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    assert_eq!(host.focused(), Some(EDITOR));
    host.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    assert_eq!(host.focused(), Some(ACTION));
    host.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: true },
        down: true,
    });
    assert_eq!(host.focused(), Some(EDITOR));
    host.focus(None, false);
    assert!(
        host.accessibility_action(EDITOR, None, AccessibilityAction::Focus)
            .consumed
    );
    assert_eq!(host.focused(), Some(EDITOR));
    let point = host.text_surface(EDITOR).unwrap().inner.center();
    assert!(
        host.event(&ContentEvent::PointerDown(point))
            .capture_pointer
    );
    assert_eq!(
        host.padding_focus_target(&ContentEvent::PointerDown(edges(&host)[0])),
        None
    );
    host.event(&ContentEvent::PointerUp(point));
    assert_eq!(
        host.padding_focus_target(&ContentEvent::PointerDown(edges(&host)[0])),
        Some(EDITOR)
    );
}
#[test]
fn same_turn_public_intent_keeps_a_composing_other_owner_behind_the_host_barrier() {
    let mut host = setup();
    host.focus(Some(OTHER), true);
    host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: vec![],
        replacement: None,
    });
    let original = host.focused_text().unwrap().input.external_baseline();
    let event = ContentEvent::PointerDown(edges(&host)[0]);
    assert_eq!(host.padding_focus_target(&event), Some(EDITOR));
    assert_eq!(padding_ime_admission(&host, &event), Some(false));
    assert_eq!(host.focused(), Some(OTHER));
    assert_eq!(
        host.focused_text().unwrap().input.external_baseline(),
        original
    );
    // The host intentionally withholds this new-owner down; it neither blurs nor refocuses.
    host.event(&ContentEvent::Unmark);
    assert_eq!(padding_ime_admission(&host, &event), Some(true));
    let point = edges(&host)[0];
    click(&mut host, point);
    assert_eq!(host.focused(), Some(EDITOR));
}
