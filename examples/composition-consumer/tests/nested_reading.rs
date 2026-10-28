//! Issue53: original bounded readonly child, original parent reading transaction.
//! No wheel replay, alternate controller, copied hit testing or pixel compensation.
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::nested_reading::*;
use tekes_ui_controls::{ScrollArea, ScrollPolicy, composition::*};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Theme, proposal};
fn fixture() -> (
    ContentHost,
    coretext::Text,
    tekes_ui_controls::ReadingDocumentVersion,
) {
    let (mut host, state) = build().unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    paint(&mut host, &mut text);
    (host, text, state.version)
}
fn paint(host: &mut ContentHost, text: &mut coretext::Text) {
    text.atlas.begin_frame();
    let frame = Rect::new(0., 0., 420., 340.);
    host.render(&mut Scene::new(), text, frame, frame, Theme::LIGHT.into())
        .unwrap();
}
fn parent(host: &ContentHost) -> &ScrollContent {
    host.get(PARENT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
}
fn inner(host: &ContentHost) -> &TextFieldContent {
    host.get(INNER)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
}
fn wheel(host: &mut ContentHost, delta: f32, time: f64, momentum: bool) -> ContentResponse {
    let point = inner(host).surface.inner.center();
    host.event(&ContentEvent::Scroll {
        position: point,
        delta: Point::new(0., delta),
        precise: true,
        begin: !momentum,
        end: false,
        momentum,
        time,
    })
}
#[test]
fn inner_history_scroll_pauses_parent_follow_without_moving_parent_or_composer() {
    let (mut host, mut text, version) = fixture();
    assert!(parent(&host).is_following_latest());
    host.focus(Some(COMPOSER), true);
    let input = &mut host
        .get_mut(COMPOSER)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface
        .input;
    input.preedit("ni", 0..2, None);
    let before = input.external_baseline();
    assert!(wheel(&mut host, 160., 1., false).consumed);
    paint(&mut host, &mut text);
    let outer_offset = parent(&host).scroll.offset;
    assert!(inner(&host).surface.scroll.offset.y > 100.);
    assert!(wheel(&mut host, -60., 2., false).consumed);
    assert!(inner(&host).surface.scroll.offset.y < 150.);
    assert_eq!(
        parent(&host).scroll.offset,
        outer_offset,
        "same wheel must not scroll parent twice"
    );
    assert!(
        !parent(&host).is_following_latest(),
        "consumed inner history intent never reached original parent"
    );
    let before_y = host.get(INNER).unwrap().frame().min_y();
    let appended = ComponentId(53006);
    host.update_scroll_reading_document(PARENT, &version, 2, vec![INNER, TAIL, appended], |h| {
        h.insert(
            BODY,
            Content::new(appended, TextContent::new("new line\n".repeat(50))),
        )
        .unwrap();
    })
    .unwrap();
    paint(&mut host, &mut text);
    assert!((host.get(INNER).unwrap().frame().min_y() - before_y).abs() <= 1.);
    assert!(host.get(INNER).unwrap().visible().is_some());
    assert_eq!(parent(&host).scroll.offset, outer_offset);
    assert_eq!(host.focused(), Some(COMPOSER));
    let surface = &host
        .get(COMPOSER)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface;
    assert_eq!(surface.input.external_baseline(), before);
    assert!(surface.input.marked.is_some());
    assert!(host.drain_actions().is_empty());
}

#[test]
fn nested_momentum_obeys_return_latest_and_fresh_gesture_without_double_scroll() {
    let (mut host, mut text, _) = fixture();
    assert!(wheel(&mut host, 160., 1., false).consumed);
    paint(&mut host, &mut text);
    assert!(
        parent(&host).is_following_latest(),
        "downward inner reading must not pause parent"
    );
    let offset = parent(&host).scroll.offset;
    assert!(wheel(&mut host, -10., 1.01, true).consumed);
    assert!(
        !parent(&host).is_following_latest(),
        "fresh original gesture must allow its upward momentum"
    );
    assert_eq!(parent(&host).scroll.offset, offset);
    host.get_mut(PARENT)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    assert!(wheel(&mut host, -10., 1.02, true).consumed);
    assert!(
        parent(&host).is_following_latest(),
        "Return to latest must fence older nested momentum"
    );
    assert!(wheel(&mut host, -10., 2., false).consumed);
    assert!(!parent(&host).is_following_latest());
    assert_eq!(parent(&host).scroll.offset, offset);
}

#[test]
fn nonoverflow_bubbles_once_editable_stays_local_and_ineligible_sources_do_not_notify() {
    // The default readonly Content has scrolling disabled; its wheel continues
    // through the original route and the parent performs the only scroll.
    let (mut host, mut text, _) = fixture();
    let field = host
        .get_mut(INNER)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field
        .surface
        .scroll
        .set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Disabled);
    let appended = ComponentId(53007);
    host.insert(
        BODY,
        Content::new(
            appended,
            TextContent::new("large outer document\n".repeat(40)),
        ),
    )
    .unwrap();
    paint(&mut host, &mut text);
    let scroll = host
        .get_mut(PARENT)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap();
    scroll.stop_following();
    scroll.scroll.scroll_by(0., 80. - scroll.scroll.offset.y);
    paint(&mut host, &mut text);
    let before = parent(&host).scroll.offset.y;
    let point = host.get(INNER).unwrap().visible().unwrap().center();
    host.event(&ContentEvent::Scroll {
        position: point,
        delta: Point::new(0., -20.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 1.,
    });
    assert!(parent(&host).scroll.offset.y < before);

    // An editable nested surface is an editor viewport, not a readonly-history
    // source. Scrolling it must not alter a containing reading document's intent.
    let (mut host, mut text, _) = fixture();
    host.get_mut(INNER)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .read_only = false;
    paint(&mut host, &mut text);
    wheel(&mut host, 160., 1., false);
    wheel(&mut host, -60., 2., false);
    assert!(parent(&host).is_following_latest());

    let (mut host, mut text, _) = fixture();
    host.get_mut(INNER)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .disabled = true;
    paint(&mut host, &mut text);
    let before = inner(&host).surface.scroll.offset;
    wheel(&mut host, -60., 1., false);
    assert_eq!(inner(&host).surface.scroll.offset, before);
    // Disabled child leaves this wheel unhandled. Its eligible parent may
    // process that original wheel normally; that is not a child notification.
    assert!(!parent(&host).is_following_latest());

    let (mut host, _, _) = fixture();
    host.event(&ContentEvent::Scroll {
        position: Point::new(-100., -100.),
        delta: Point::new(0., -60.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 1.,
    });
    assert!(parent(&host).is_following_latest());
    host.set_window_active(false);
    wheel(&mut host, -60., 1., false);
    assert!(parent(&host).is_following_latest());
}

#[test]
fn no_deferred_notice_survives_document_or_mount_aba() {
    let (mut host, mut text, _) = fixture();
    wheel(&mut host, 160., 1., false);
    wheel(&mut host, -60., 2., false);
    assert!(!parent(&host).is_following_latest());
    for key in ["parent-B", "parent-A"] {
        host.set_scroll_reading_document(PARENT, key, 2, vec![INNER, TAIL], |_| ())
            .unwrap();
        host.get_mut(PARENT)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .follow_latest();
        paint(&mut host, &mut text);
        assert!(parent(&host).is_following_latest());
    }
    host.remove(INNER).unwrap();
    host.insert(
        BODY,
        Content::new(
            INNER,
            TextFieldContent::read_only_content("replacement", "short", proposal::TextRole::UiBody),
        ),
    )
    .unwrap();
    paint(&mut host, &mut text);
    assert!(
        parent(&host).is_following_latest(),
        "old owner notification was retained across removal/remount"
    );
}

#[test]
fn original_ancestor_path_notifies_all_nested_readers_and_no_sibling_document() {
    let (mut old, mut text, _) = fixture();
    let original = old.remove(PARENT).unwrap();
    let sibling = ComponentId(53100);
    let sibling_text = ComponentId(53101);
    let mut host = ContentHost::new(Content::new(
        ComponentId(53102),
        GridContent {
            model: tekes_ui_controls::extended::Grid::new(2),
            children: vec![
                original,
                Content::new(
                    sibling,
                    ScrollContent::new(
                        ScrollArea::default(),
                        Content::new(sibling_text, TextContent::new("sibling document")),
                    ),
                ),
            ],
        },
    ))
    .unwrap();
    host.set_scroll_reading_document(PARENT, "parent-A", 1, vec![INNER, TAIL], |_| ())
        .unwrap();
    host.set_scroll_reading_document(sibling, "sibling-B", 1, vec![sibling_text], |_| ())
        .unwrap();
    for owner in [PARENT, sibling] {
        host.get_mut(owner)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .follow_latest();
    }
    paint(&mut host, &mut text);
    wheel(&mut host, 160., 1., false);
    wheel(&mut host, -60., 2., false);
    assert!(!parent(&host).is_following_latest());
    assert!(
        host.get(sibling)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .is_following_latest()
    );

    // Same original nested viewport, mounted inside another actual ScrollContent.
    let mut original = host.remove(PARENT).unwrap();
    original
        .component_mut::<ScrollContent>()
        .unwrap()
        .viewport_height = Some(240.);
    let outer = ComponentId(53200);
    let mut host = ContentHost::new(Content::new(
        outer,
        ScrollContent::new(ScrollArea::default(), original),
    ))
    .unwrap();
    host.set_scroll_reading_document(outer, "outer", 1, vec![PARENT], |_| ())
        .unwrap();
    host.set_scroll_reading_document(PARENT, "parent-A", 1, vec![INNER, TAIL], |_| ())
        .unwrap();
    for owner in [PARENT, outer] {
        host.get_mut(owner)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .follow_latest();
    }
    paint(&mut host, &mut text);
    let before = host
        .get(outer)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    wheel(&mut host, 160., 3., false);
    wheel(&mut host, -60., 4., false);
    assert!(!parent(&host).is_following_latest());
    let outer = host
        .get(outer)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap();
    assert!(!outer.is_following_latest());
    assert_eq!(
        outer.scroll.offset, before,
        "nested notice must never execute outer wheel"
    );
}

#[test]
fn modal_and_interactive_overlay_scopes_do_not_notify_background_reading() {
    let (mut host, mut text, _) = fixture();
    let dialog = ComponentId(53300);
    let field = ComponentId(53301);
    let make_field = || {
        let mut field = TextFieldContent::read_only_content(
            "Scoped history",
            &"history\n".repeat(70),
            proposal::TextRole::UiBody,
        );
        field.viewport = TextViewport::Grow {
            min: 120.,
            max: 120.,
        };
        field
            .surface
            .scroll
            .set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Auto);
        field
    };
    host.insert(
        ROOT,
        Content::new(
            dialog,
            DialogContent::new(
                "Scoped dialog",
                Content::new(ComponentId(53302), TextContent::new("Title")),
                Content::new(field, make_field()),
                Content::new(ComponentId(53303), Button::new("Close")),
            ),
        ),
    )
    .unwrap();
    host.show_dialog(dialog).unwrap();
    paint(&mut host, &mut text);
    let point = host
        .get(field)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .inner
        .center();
    for (delta, time) in [(160., 1.), (-60., 2.)] {
        let response = host.event(&ContentEvent::Scroll {
            position: point,
            delta: Point::new(0., delta),
            precise: true,
            begin: true,
            end: false,
            momentum: false,
            time,
        });
        assert!(response.consumed);
    }
    assert!(parent(&host).is_following_latest());
    host.close_dialog();
    paint(&mut host, &mut text);
    let popup = ComponentId(53304);
    let mut content = PopoverContent::new("Scoped popup", Content::new(field, make_field()));
    content.set_dismiss_visible(false);
    // Retire the old modal tree first: stable numeric IDs may then be reused.
    host.remove(dialog).unwrap();
    host.insert(ROOT, Content::new(popup, content)).unwrap();
    host.show_popover(popup, COMPOSER).unwrap();
    paint(&mut host, &mut text);
    let point = host
        .get(field)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .inner
        .center();
    for (delta, time) in [(160., 3.), (-60., 4.)] {
        host.event(&ContentEvent::Scroll {
            position: point,
            delta: Point::new(0., delta),
            precise: true,
            begin: true,
            end: false,
            momentum: false,
            time,
        });
    }
    assert!(parent(&host).is_following_latest());
}

#[test]
fn readonly_keyboard_navigation_pauses_parent_even_when_no_parent_reveal_is_needed() {
    for command in [true, false] {
        let (mut host, mut text, _) = fixture();
        assert!(host.focus(Some(INNER), true));
        assert!(parent(&host).is_following_latest());
        let before = parent(&host).scroll.offset;
        let event = if command {
            ContentEvent::TextCommand("moveDown:".into())
        } else {
            ContentEvent::Key {
                key: tekes_ui_controls::ControlKey::Down,
                down: true,
            }
        };
        let response = host.event_with_text(&event, &mut text).unwrap();
        assert!(response.consumed);
        assert!(
            !parent(&host).is_following_latest(),
            "original readonly keyboard navigation did not notify parent"
        );
        assert_eq!(parent(&host).scroll.offset, before);
        assert_eq!(host.focused(), Some(INNER));
        assert!(!inner(&host).surface.input.text.is_empty());
    }
}

#[test]
fn original_readonly_selection_and_scrollbar_capture_pause_parent_without_copying_capture() {
    for scrollbar in [false, true] {
        let (mut host, mut text, _) = fixture();
        host.focus(Some(COMPOSER), true);
        let point = if scrollbar {
            let surface = &inner(&host).surface;
            surface
                .scroll
                .thumb(true, surface.track(true), 24.)
                .center()
        } else {
            inner(&host).surface.inner.center()
        };
        let before = parent(&host).scroll.offset;
        let response = host.event(&ContentEvent::PointerDown(point));
        assert!(response.consumed && response.capture_pointer);
        assert!(host.pointer_capture().is_some_and(|(id, _)| id == INNER));
        assert_eq!(
            inner(&host).surface.scroll.dragging_axis(),
            scrollbar.then_some(true)
        );
        assert!(
            !parent(&host).is_following_latest(),
            "original readonly pointer/scrollbar selection did not notify parent"
        );
        assert_eq!(parent(&host).scroll.offset, before);
        assert_eq!(
            host.focused(),
            Some(INNER),
            "explicit selection still focuses the original readonly owner"
        );
        let capture = host.pointer_capture();
        host.get_mut(PARENT)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .follow_latest();
        let moved = Point::new(point.x, point.y + 20.);
        host.event(&ContentEvent::PointerMove(moved));
        assert_eq!(host.pointer_capture(), capture);
        assert!(
            !parent(&host).is_following_latest(),
            "fresh retained drag did not pause a later follow intent"
        );
        host.event(&ContentEvent::PointerUp(moved));
        assert!(host.pointer_capture().is_none());
        paint(&mut host, &mut text);
    }
}

#[test]
fn programmatic_readonly_selection_is_not_a_user_history_notice() {
    let (mut host, mut text, _) = fixture();
    assert!(host.focus(Some(INNER), true));
    let surface = &mut host
        .get_mut(INNER)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface;
    surface.input.anchor = 0;
    surface.input.caret = 10;
    paint(&mut host, &mut text);
    assert!(
        parent(&host).is_following_latest(),
        "ordinary model/layout must not invent user reading input"
    );
}

#[test]
fn deferred_native_navigation_notifies_only_after_the_original_key_is_accepted() {
    for consumed_by_input_context in [false, true] {
        let (mut host, mut text, _) = fixture();
        host.focus(Some(INNER), true);
        let key = ContentKeyEvent {
            key: Some(tekes_ui_controls::ControlKey::Down),
            down: true,
            repeat: false,
            modifiers: InputModifiers::default(),
            native_code: 125,
            characters: "\u{f701}".into(),
            characters_ignoring_modifiers: "\u{f701}".into(),
        };
        host.event(&ContentEvent::TextKeyStart {
            id: 530001,
            key: key.clone(),
            composing: false,
        });
        host.event(&ContentEvent::KeyInput(key));
        assert!(
            parent(&host).is_following_latest(),
            "pending native key invented an accepted reading intent"
        );
        let response = host
            .event_with_text(
                &ContentEvent::TextKeyEnd {
                    id: 530001,
                    input_context_consumed: consumed_by_input_context,
                },
                &mut text,
            )
            .unwrap();
        assert_eq!(
            response.text_key.unwrap().disposition,
            if consumed_by_input_context {
                TextKeyDisposition::InputConsumed
            } else {
                TextKeyDisposition::EditHandled
            }
        );
        assert_eq!(
            parent(&host).is_following_latest(),
            consumed_by_input_context
        );
    }
}

#[test]
fn cancelled_native_callback_cannot_pause_the_new_original_readonly_owner() {
    let (mut host, mut text, _) = fixture();
    let other = ComponentId(53501);
    let mut field = TextFieldContent::read_only_content(
        "Other readonly",
        "another field",
        proposal::TextRole::UiBody,
    );
    field.viewport = TextViewport::Grow { min: 30., max: 30. };
    host.insert(BODY, Content::new(other, field)).unwrap();
    paint(&mut host, &mut text);
    host.focus(Some(INNER), true);
    let key = ContentKeyEvent {
        key: Some(tekes_ui_controls::ControlKey::Down),
        down: true,
        repeat: false,
        modifiers: InputModifiers::default(),
        native_code: 125,
        characters: "\u{f701}".into(),
        characters_ignoring_modifiers: "\u{f701}".into(),
    };
    host.event(&ContentEvent::TextKeyStart {
        id: 530002,
        key,
        composing: false,
    });
    host.focus(Some(other), true);
    host.get_mut(PARENT)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    let before = host.text_surface(other).unwrap().input.external_baseline();
    host.event(&ContentEvent::TextCommand("moveDown:".into()));
    let response = host.event(&ContentEvent::TextKeyEnd {
        id: 530002,
        input_context_consumed: false,
    });
    assert_eq!(
        response.text_key.unwrap().disposition,
        TextKeyDisposition::Cancelled
    );
    assert!(parent(&host).is_following_latest());
    assert_eq!(
        host.text_surface(other).unwrap().input.external_baseline(),
        before
    );
}

#[test]
fn retained_pointer_release_cancel_and_remount_do_not_replay_manual_reading_intent() {
    let (mut host, mut text, _) = fixture();
    let point = inner(&host).surface.inner.center();
    host.event(&ContentEvent::PointerDown(point));
    assert!(!parent(&host).is_following_latest());
    host.get_mut(PARENT)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    host.event(&ContentEvent::PointerUp(point));
    assert!(
        parent(&host).is_following_latest(),
        "old release replayed a new history intent"
    );
    host.event(&ContentEvent::PointerDown(point));
    assert!(host.pointer_capture().is_some());
    host.event(&ContentEvent::Cancel);
    host.get_mut(PARENT)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    host.event(&ContentEvent::PointerMove(point));
    assert!(parent(&host).is_following_latest());
    assert!(host.pointer_capture().is_none());
    host.remove(INNER).unwrap();
    host.insert(
        BODY,
        Content::new(
            INNER,
            TextFieldContent::read_only_content(
                "New mount",
                "replacement",
                proposal::TextRole::UiBody,
            ),
        ),
    )
    .unwrap();
    paint(&mut host, &mut text);
    host.event(&ContentEvent::PointerMove(point));
    assert!(parent(&host).is_following_latest());
}

#[test]
fn original_scrollbar_drag_and_track_click_never_reveal_the_old_insertion_caret() {
    for thumb_drag in [false, true] {
        let (mut host, mut text, _) = fixture();
        host.focus(Some(INNER), true);
        let surface = &inner(&host).surface;
        let track = surface.track(true);
        let point = if thumb_drag {
            surface.scroll.thumb(true, track, 24.).center()
        } else {
            Point::new(track.center().x, track.max_y() - 8.)
        };
        let selection = surface.input.selection();
        let before_parent = parent(&host).scroll.offset;
        assert!(
            host.event(&ContentEvent::PointerDown(point))
                .capture_pointer
        );
        if thumb_drag {
            host.event(&ContentEvent::PointerMove(Point::new(
                point.x,
                point.y + 20.,
            )));
        }
        let offset = inner(&host).surface.scroll.offset.y;
        assert!(
            offset > 50.,
            "old caret reveal undid original scrollbar input: {offset}"
        );
        host.event(&ContentEvent::PointerUp(point));
        assert!(
            (inner(&host).surface.scroll.offset.y - offset).abs() < 0.01,
            "release restored old caret"
        );
        assert_eq!(inner(&host).surface.input.selection(), selection);
        assert_eq!(parent(&host).scroll.offset, before_parent);
        assert!(!parent(&host).is_following_latest());
        assert!(host.pointer_capture().is_none());
        paint(&mut host, &mut text);
        assert!(
            (inner(&host).surface.scroll.offset.y - offset).abs() < 0.01,
            "next layout restored old caret"
        );
    }
}

#[test]
fn public_surface_reveal_cannot_steal_an_active_original_scrollbar_gesture() {
    let (mut host, _, _) = fixture();
    let surface = &mut host
        .get_mut(INNER)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface;
    let point = surface
        .scroll
        .thumb(true, surface.track(true), 24.)
        .center();
    assert!(surface.pointer_down(point, false, 1, 24.).is_some());
    assert!(surface.pointer_drag(Point::new(point.x, point.y + 20.), 24.));
    let offset = surface.scroll.offset;
    assert!(offset.y > 50.);
    surface.reveal();
    assert_eq!(
        surface.scroll.offset, offset,
        "raw TextSurface host revealed an old caret during owned scrollbar drag"
    );
    surface.pointer_up();
}
