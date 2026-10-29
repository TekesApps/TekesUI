//! Wire projection onto the original child ContentHost; no editing/focus model.
use anyhow::{Result, ensure};
use tekes_ui_app_protocol::component::content as wire;
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey,
    composition::{
        ComponentId, ContentEvent, ContentHost, ContentKeyEvent, ContentResponse, InputModifiers,
        PointerButton, PointerEvent, PointerPhase, TextKeyDisposition,
    },
};
use tekes_ui_scene::{Point, Rect};

fn key(value: wire::KeyEvent) -> ContentKeyEvent {
    ContentKeyEvent {
        key: value.key.map(|key| match key {
            wire::Key::Enter => ControlKey::Enter,
            wire::Key::Space => ControlKey::Space,
            wire::Key::Left => ControlKey::Left,
            wire::Key::Right => ControlKey::Right,
            wire::Key::Up => ControlKey::Up,
            wire::Key::Down => ControlKey::Down,
            wire::Key::Home => ControlKey::Home,
            wire::Key::End => ControlKey::End,
            wire::Key::Escape => ControlKey::Escape,
            wire::Key::Tab { reverse } => ControlKey::Tab { reverse },
        }),
        down: value.down,
        repeat: value.repeat,
        modifiers: InputModifiers {
            shift: value.modifiers.shift,
            control: value.modifiers.control,
            alt: value.modifiers.alt,
            command: value.modifiers.command,
            caps_lock: value.modifiers.caps_lock,
            function: value.modifiers.function,
        },
        native_code: value.native_code,
        characters: value.characters,
        characters_ignoring_modifiers: value.characters_ignoring_modifiers,
    }
}
fn rect(r: Rect) -> [f32; 4] {
    [r.min_x(), r.min_y(), r.size.width, r.size.height]
}
fn target(host: &ContentHost, id: ComponentId) -> Option<wire::Target> {
    Some(wire::Target {
        id: id.0,
        mount: host.semantic_mount(id)?,
    })
}
/// Build a read-only projection directly from the original arranged Host.
/// This deliberately avoids `dispatch`, whose legacy query lane reconciles
/// input policy and may mutate focus-wrap state.
pub(crate) fn read_only_projection(
    host: &ContentHost,
    view: u64,
    viewport: u64,
    reading: Option<wire::ReadingState>,
) -> Result<wire::Reply> {
    let focused = host.focused().and_then(|id| target(host, id));
    let text = focused
        .zip(host.text_input_snapshot())
        .map(|(owner, state)| wire::TextState {
            owner,
            text: state.text,
            caret_index: state.caret_index,
            selection: state.selection,
            marked: state.marked,
            read_only: state.read_only,
            caret: rect(state.caret.unwrap_or_default()),
            clip: rect(state.clip.unwrap_or_default()),
            carets: state
                .carets
                .into_iter()
                .map(|(index, bounds)| (index, rect(bounds)))
                .collect(),
        });
    let semantics = crate::content_accessibility::snapshot(host)?;
    Ok(wire::Reply {
        view,
        viewport,
        consumed: false,
        changed: false,
        animation_frame_requested: host.animation_frame_requested(),
        capture_pointer: false,
        activated: false,
        bubbles: false,
        pointer_focus_requested: false,
        accessibility_focus_requested: false,
        key: None,
        overlay: host
            .input_overlay_owner()
            .map(|(id, mount)| wire::Target { id: id.0, mount }),
        reading_scroll_source: None,
        reading,
        semantics: Some(semantics),
        focused,
        focus_epoch: host.focus_epoch(),
        text_binding: host.native_key_binding(),
        captured: host
            .pointer_capture_lease()
            .map(|(id, mount, serial)| wire::Capture {
                target: wire::Target { id: id.0, mount },
                serial,
            }),
        focus_targets: host
            .focus_targets()
            .into_iter()
            .filter_map(|(id, _)| target(host, id))
            .collect(),
        composing: host.is_composing(),
        text,
    })
}
pub(crate) fn event(input: &wire::Input) -> Option<ContentEvent> {
    match input {
        wire::Input::Snapshot
        | wire::Input::Semantics
        | wire::Input::Accessibility { .. }
        | wire::Input::ReadingState { .. }
        | wire::Input::RevealReadingItem { .. }
        | wire::Input::TextKeyResult { .. }
        | wire::Input::Focus { .. }
        | wire::Input::EnterFocus { .. }
        | wire::Input::CancelPointerCapture => None,
        wire::Input::Pointer(value) => {
            if !value.position.iter().all(|p| p.is_finite()) {
                return None;
            }
            Some(ContentEvent::Pointer(PointerEvent {
                position: Point::new(value.position[0], value.position[1]),
                phase: match value.phase {
                    wire::PointerPhase::Down => PointerPhase::Down,
                    wire::PointerPhase::Up => PointerPhase::Up,
                    wire::PointerPhase::Move => PointerPhase::Move,
                    wire::PointerPhase::Drag => PointerPhase::Drag,
                    wire::PointerPhase::Enter => PointerPhase::Enter,
                    wire::PointerPhase::Exit => PointerPhase::Exit,
                },
                button: value.button.map(|b| match b {
                    wire::PointerButton::Primary => PointerButton::Primary,
                    wire::PointerButton::Secondary => PointerButton::Secondary,
                    wire::PointerButton::Middle => PointerButton::Middle,
                    wire::PointerButton::Other(n) => PointerButton::Other(n),
                }),
                modifiers: InputModifiers {
                    shift: value.modifiers.shift,
                    control: value.modifiers.control,
                    alt: value.modifiers.alt,
                    command: value.modifiers.command,
                    caps_lock: value.modifiers.caps_lock,
                    function: value.modifiers.function,
                },
                click_count: value.click_count,
            }))
        }
        wire::Input::Scroll(value) => value.is_valid().then_some(ContentEvent::Scroll {
            position: Point::new(value.position[0], value.position[1]),
            delta: Point::new(value.delta[0], value.delta[1]),
            precise: value.precise,
            begin: value.begin,
            end: value.end,
            momentum: value.momentum,
            time: value.time,
        }),
        wire::Input::Key(value) => Some(ContentEvent::KeyInput(key(value.clone()))),
        wire::Input::TextKeyStart {
            id,
            key: value,
            composing,
        } => Some(ContentEvent::TextKeyStart {
            id: *id,
            key: key(value.clone()),
            composing: *composing,
        }),
        wire::Input::TextKeyEnd {
            id,
            input_context_consumed,
        } => Some(ContentEvent::TextKeyEnd {
            id: *id,
            input_context_consumed: *input_context_consumed,
        }),
        wire::Input::Commit { text, replacement } => Some(ContentEvent::Commit {
            text: text.clone(),
            replacement: replacement.clone(),
        }),
        wire::Input::Preedit {
            text,
            selection,
            clauses,
            replacement,
        } => Some(ContentEvent::Preedit {
            text: text.clone(),
            selection: selection.clone(),
            clauses: clauses.clone(),
            replacement: replacement.clone(),
        }),
        wire::Input::Unmark => Some(ContentEvent::Unmark),
        wire::Input::TextCommand(command) => Some(ContentEvent::TextCommand(command.clone())),
        wire::Input::Cancel => Some(ContentEvent::Cancel),
    }
}
pub(crate) fn dispatch(
    host: &mut ContentHost,
    text: &mut dyn ButtonTextRenderer,
    request: wire::Request,
) -> Result<wire::Reply> {
    let focus_before = host.focused();
    let query_only = matches!(
        request.input,
        wire::Input::Snapshot | wire::Input::Semantics | wire::Input::ReadingState { .. }
    );
    let capture = || {
        host.pointer_capture_lease()
            .map(|(id, mount, serial)| wire::Capture {
                target: wire::Target { id: id.0, mount },
                serial,
            })
    };
    // Finish is an original key-frame cleanup, not a new edit. The original
    // host validates its immutable key ID, owner/mount/focus/binding snapshot
    // before any fallback; a stale frame finishes as Cancelled.
    if !matches!(
        request.input,
        wire::Input::Snapshot
            | wire::Input::Semantics
            | wire::Input::ReadingState { .. }
            | wire::Input::TextKeyEnd { .. }
            | wire::Input::TextKeyResult { .. }
    ) {
        let focused = host.focused().and_then(|id| target(host, id));
        let expected = if matches!(request.input, wire::Input::Pointer(_)) {
            ensure!(
                request.capture == capture(),
                "stale Content pointer gesture"
            );
            capture().map(|c| c.target).or(focused)
        } else {
            focused
        };
        ensure!(request.owner == expected, "stale Content callback owner");
        // A held pointer keeps its capture lease across overlay focus changes.
        // Other callbacks cannot edit a blurred/refocused or rebound editor.
        if !matches!(request.input, wire::Input::Pointer(_)) || capture().is_none() {
            ensure!(
                request.focus_epoch == host.focus_epoch()
                    && request.text_binding == host.native_key_binding(),
                "stale Content focus/binding lease"
            );
        }
    }
    if let wire::Input::Pointer(pointer) = &request.input {
        ensure!(
            pointer.position.iter().all(|p| p.is_finite()),
            "invalid Content pointer position"
        );
    }
    if let wire::Input::Scroll(scroll) = &request.input {
        ensure!(scroll.is_valid(), "invalid Content scroll input");
    }
    let semantics = matches!(request.input, wire::Input::Semantics)
        .then(|| crate::content_accessibility::snapshot(host))
        .transpose()?;
    let reading = crate::content_reading::dispatch(host, text, &request.input)?;
    let event = event(&request.input);
    // The outer ContentHost owns traversal between containers. Original child
    // overlays/modals still wrap through their existing scope policy.
    host.set_focus_wrap(false);
    let delivered = event.is_some();
    let (response, bubbles) = if let Some(event) = event {
        let response = host.event_with_text(&event, text)?;
        (response, host.event_bubbled())
    } else if let wire::Input::Accessibility {
        target,
        part,
        ref binding,
        ref action,
    } = request.input
    {
        (
            crate::content_accessibility::action(host, target, part, binding.as_deref(), action)?,
            false,
        )
    } else if matches!(request.input, wire::Input::RevealReadingItem { .. }) {
        (ContentResponse::consumed(host.needs_render()), false)
    } else if let wire::Input::TextKeyResult { id } = request.input {
        let response = host.text_key_result(id);
        (response, host.event_bubbled())
    } else if matches!(request.input, wire::Input::CancelPointerCapture) {
        (
            ContentResponse::consumed(host.cancel_pointer_capture()),
            false,
        )
    } else if let wire::Input::EnterFocus { reverse } = request.input {
        ensure!(
            !host.is_composing(),
            "composition must finish before focus traversal"
        );
        let changed = host.enter_focus(reverse);
        (ContentResponse::consumed(changed), false)
    } else if let wire::Input::Focus {
        target: next,
        visible,
    } = request.input
    {
        let id = next.map(|t| ComponentId(t.id));
        if let Some(next) = next {
            ensure!(
                host.semantic_mount(ComponentId(next.id)) == Some(next.mount),
                "stale Content focus mount"
            );
            ensure!(
                host.focus_targets().iter().any(|(id, _)| id.0 == next.id),
                "unavailable Content focus target"
            );
        }
        ensure!(
            host.focused() == id || !host.is_composing(),
            "composition must finish before focus transfer"
        );
        let changed = host.focus(id, visible);
        (ContentResponse::consumed(changed), false)
    } else {
        (ContentResponse::default(), false)
    };
    let focused = host.focused().and_then(|id| target(host, id));
    let projected = focused
        .zip(host.text_input_snapshot())
        .map(|(owner, s)| wire::TextState {
            owner,
            text: s.text,
            caret_index: s.caret_index,
            selection: s.selection,
            marked: s.marked,
            read_only: s.read_only,
            caret: rect(s.caret.unwrap_or_default()),
            clip: rect(s.clip.unwrap_or_default()),
            carets: s
                .carets
                .into_iter()
                .map(|(index, r)| (index, rect(r)))
                .collect(),
        });
    let reply = wire::Reply {
        view: request.view,
        viewport: request.viewport,
        consumed: response.consumed,
        // Cancel/focus/structural reconciliation may dirty the original host
        // without a component response reporting a value change. Publish its
        // existing invalidation, so the parent does not retain pressed pixels.
        // Observing pending paint must not create a second invalidation lane.
        changed: response.changed || (!query_only && host.needs_render()),
        animation_frame_requested: host.animation_frame_requested(),
        capture_pointer: response.capture_pointer,
        activated: response.activated,
        bubbles,
        pointer_focus_requested: delivered && host.pointer_focus_requested(),
        accessibility_focus_requested: response.consumed
            && matches!(request.input, wire::Input::Accessibility { .. })
            && (host.focused() != focus_before
                || matches!(
                    request.input,
                    wire::Input::Accessibility {
                        action: wire::AccessibilityAction::Focus
                            | wire::AccessibilityAction::SetSelection(_),
                        ..
                    }
                )),
        key: response.text_key.map(|outcome| wire::KeyOutcome {
            id: outcome.id,
            target: outcome.target.map(|id| id.0),
            disposition: match outcome.disposition {
                TextKeyDisposition::Deferred => wire::KeyDisposition::Deferred,
                TextKeyDisposition::InputConsumed => wire::KeyDisposition::InputConsumed,
                TextKeyDisposition::EditHandled => wire::KeyDisposition::EditHandled,
                TextKeyDisposition::Submitted => wire::KeyDisposition::Submitted,
                TextKeyDisposition::Navigated => wire::KeyDisposition::Navigated,
                TextKeyDisposition::Unhandled => wire::KeyDisposition::Unhandled,
                TextKeyDisposition::Cancelled => wire::KeyDisposition::Cancelled,
            },
        }),
        overlay: host
            .input_overlay_owner()
            .map(|(id, mount)| wire::Target { id: id.0, mount }),
        reading_scroll_source: if matches!(request.input, wire::Input::Scroll(_)) {
            host.reading_scroll_source()
                .map(|(id, mount)| wire::Target { id: id.0, mount })
        } else {
            None
        },
        reading,
        semantics,
        focused,
        focus_epoch: host.focus_epoch(),
        text_binding: host.native_key_binding(),
        captured: host
            .pointer_capture_lease()
            .map(|(id, mount, serial)| wire::Capture {
                target: wire::Target { id: id.0, mount },
                serial,
            }),
        focus_targets: host
            .focus_targets()
            .into_iter()
            .filter_map(|(id, _)| target(host, id))
            .collect(),
        composing: host.is_composing(),
        text: projected,
    };
    if matches!(request.input, wire::Input::Semantics) {
        ensure!(
            serde_json::to_vec(&reply)?.len() <= crate::pipes::MAX_PACKET / 2,
            "Content semantic reply exceeds transport budget"
        );
    }
    Ok(reply)
}

#[cfg(test)]
mod scroll_tests {
    use super::*;
    use tekes_ui_controls::{
        ButtonTextAlign, ScrollArea,
        composition::*,
        extended::{Axis, Stack},
        layout::MeasuredChild,
    };
    use tekes_ui_scene::{Scene, Size};
    use tekes_ui_tokens::{Rgba, Theme, proposal};
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(100., 24.)
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
    #[derive(Debug)]
    struct Row;
    impl Component for Row {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
            MeasuredChild::new(Size::new(a.width, 32.))
        }
    }
    fn render(host: &mut ContentHost) {
        let f = Rect::new(0., 0., 240., 120.);
        host.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
            .unwrap();
    }
    fn request(host: &ContentHost, scroll: wire::ScrollEvent) -> wire::Request {
        wire::Request {
            view: 7,
            viewport: 1,
            owner: None,
            focus_epoch: host.focus_epoch(),
            text_binding: None,
            capture: None,
            input: wire::Input::Scroll(scroll),
        }
    }
    fn wheel() -> wire::ScrollEvent {
        wire::ScrollEvent {
            position: [40., 40.],
            delta: [0., -12.],
            precise: true,
            begin: true,
            end: false,
            momentum: false,
            time: 1.,
        }
    }
    #[test]
    fn original_scroll_and_reading_receive_one_wire_event_without_focus_or_offset_replica() {
        let ids: Vec<_> = (3..43).map(ComponentId).collect();
        let body = Content::new(
            ComponentId(2),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: ids.iter().map(|id| Content::new(*id, Row)).collect(),
            },
        );
        let mut host = ContentHost::new(Content::new(
            ComponentId(1),
            ScrollContent::new(ScrollArea::default(), body),
        ))
        .unwrap();
        host.set_scroll_reading_document(ComponentId(1), "wire-reader", 1, ids, |_| ())
            .unwrap();
        host.get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .follow_latest();
        render(&mut host);
        let before = host
            .get(ComponentId(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset;
        let focus = host.focus_epoch();
        let input = request(&host, wheel());
        let reply = dispatch(&mut host, &mut Text, input).unwrap();
        let scroll = host
            .get(ComponentId(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap();
        assert!(reply.consumed && reply.changed && !reply.bubbles);
        assert_eq!(scroll.scroll.offset.y, before.y - 12.);
        assert!(!scroll.is_following_latest());
        assert_eq!(host.focus_epoch(), focus);
        assert_eq!(host.focused(), None);
        assert!(reply.captured.is_none());
        assert!(
            reply
                .reading_scroll_source
                .is_some_and(|target| target.id == 1)
        );
        let before = scroll.scroll.offset;
        let mut query = request(&host, wheel());
        query.input = wire::Input::Snapshot;
        assert!(
            dispatch(&mut host, &mut Text, query)
                .unwrap()
                .reading_scroll_source
                .is_none()
        );
        let bad = request(
            &host,
            wire::ScrollEvent {
                delta: [0., f32::NAN],
                ..wheel()
            },
        );
        assert!(dispatch(&mut host, &mut Text, bad).is_err());
        assert_eq!(
            host.get(ComponentId(1))
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset,
            before
        );
        let mut stale = request(&host, wheel());
        stale.focus_epoch += 1;
        assert!(dispatch(&mut host, &mut Text, stale).is_err());
        assert_eq!(
            host.get(ComponentId(1))
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset,
            before
        );
    }
    #[test]
    fn unhandled_scroll_propagates_and_invalid_projection_is_not_an_event() {
        let mut host = ContentHost::new(Content::new(ComponentId(1), Row)).unwrap();
        render(&mut host);
        let input = request(&host, wheel());
        let reply = dispatch(&mut host, &mut Text, input).unwrap();
        assert!(!reply.consumed && reply.bubbles);
        assert!(
            event(&wire::Input::Scroll(wire::ScrollEvent {
                time: f64::NAN,
                ..wheel()
            }))
            .is_none()
        );
    }
    #[derive(Debug)]
    struct InsufficientFocus {
        pointer_downs: usize,
    }
    impl Component for InsufficientFocus {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
            MeasuredChild::new(Size::new(100., 32.))
        }
        fn interactive(&self) -> bool {
            true
        }
        fn focusable(&self) -> bool {
            true
        }
        fn required_focus_space(&self) -> RequiredFocusSpace {
            RequiredFocusSpace::Insufficient
        }
        fn requires_focus_reveal(&self) -> bool {
            true
        }
        fn event(&mut self, event: &ContentEvent) -> ContentResponse {
            if matches!(event, ContentEvent::PointerDown(_)) {
                self.pointer_downs += 1;
            }
            ContentResponse::consumed(false)
        }
    }
    #[test]
    fn insufficient_pointer_focus_refuses_action_before_effect_and_publishes_no_focus() {
        let mut host = ContentHost::new(Content::new(
            ComponentId(81),
            InsufficientFocus { pointer_downs: 0 },
        ))
        .unwrap();
        render(&mut host);
        assert!(!host.focus(Some(ComponentId(81)), false));
        let request = wire::Request {
            view: 7,
            viewport: 1,
            owner: None,
            focus_epoch: host.focus_epoch(),
            text_binding: None,
            capture: None,
            input: wire::Input::Pointer(wire::PointerEvent {
                position: [10., 10.],
                phase: wire::PointerPhase::Down,
                button: Some(wire::PointerButton::Primary),
                modifiers: Default::default(),
                click_count: 1,
            }),
        };
        let reply = dispatch(&mut host, &mut Text, request).unwrap();
        assert!(!reply.consumed);
        assert_eq!(
            host.get(ComponentId(81))
                .unwrap()
                .component::<InsufficientFocus>()
                .unwrap()
                .pointer_downs,
            0
        );
        assert!(!reply.pointer_focus_requested);
        assert!(reply.focused.is_none());
        assert!(!host.pointer_focus_requested());
    }
    #[test]
    fn consumed_preserving_focus_pointer_does_not_publish_focus_intent() {
        let id = ComponentId(83);
        let mut host = ContentHost::new(
            Content::new(id, tekes_ui_controls::Button::new("Auxiliary")).preserving_focus(),
        )
        .unwrap();
        render(&mut host);
        let request = wire::Request {
            view: 7,
            viewport: 1,
            owner: None,
            focus_epoch: host.focus_epoch(),
            text_binding: None,
            capture: None,
            input: wire::Input::Pointer(wire::PointerEvent {
                position: [10., 10.],
                phase: wire::PointerPhase::Down,
                button: Some(wire::PointerButton::Primary),
                modifiers: Default::default(),
                click_count: 1,
            }),
        };
        let reply = dispatch(&mut host, &mut Text, request.clone()).unwrap();
        assert!(reply.consumed);
        assert!(!reply.pointer_focus_requested);
        assert!(reply.focused.is_none());
        assert!(!host.pointer_focus_requested());
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<tekes_ui_controls::Button>()
                .unwrap()
                .activations,
            0
        );
        let mut release = request;
        release.owner = reply.captured.map(|capture| capture.target);
        release.capture = reply.captured;
        let wire::Input::Pointer(pointer) = &mut release.input else {
            unreachable!()
        };
        pointer.phase = wire::PointerPhase::Up;
        let released = dispatch(&mut host, &mut Text, release).unwrap();
        assert!(released.consumed);
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<tekes_ui_controls::Button>()
                .unwrap()
                .activations,
            1
        );
        assert!(!released.pointer_focus_requested);
        assert!(released.focused.is_none());
        assert!(!host.pointer_focus_requested());
    }
    #[test]
    fn wire_capture_projects_current_lease_and_rejects_old_epoch_or_mount_before_delivery() {
        let id = ComponentId(82);
        let mut host =
            ContentHost::new(Content::new(id, tekes_ui_controls::Button::new("Action"))).unwrap();
        render(&mut host);
        let first = wire::Request {
            view: 7,
            viewport: 1,
            owner: None,
            focus_epoch: host.focus_epoch(),
            text_binding: None,
            capture: None,
            input: wire::Input::Pointer(wire::PointerEvent {
                position: [10., 10.],
                phase: wire::PointerPhase::Down,
                button: Some(wire::PointerButton::Primary),
                modifiers: Default::default(),
                click_count: 1,
            }),
        };
        let reply = dispatch(&mut host, &mut Text, first).unwrap();
        assert!(reply.capture_pointer);
        let capture = reply.captured.unwrap();
        assert_eq!(
            host.pointer_capture_lease(),
            Some((id, capture.target.mount, capture.serial))
        );
        assert_eq!(host.pointer_capture(), Some((id, capture.target.mount)));
        let release = wire::Request {
            view: 7,
            viewport: 1,
            owner: Some(capture.target),
            focus_epoch: reply.focus_epoch,
            text_binding: reply.text_binding,
            capture: Some(capture),
            input: wire::Input::Pointer(wire::PointerEvent {
                position: [10., 10.],
                phase: wire::PointerPhase::Up,
                button: Some(wire::PointerButton::Primary),
                modifiers: Default::default(),
                click_count: 1,
            }),
        };
        let mut old_epoch = release.clone();
        old_epoch.capture.as_mut().unwrap().serial += 1;
        assert!(dispatch(&mut host, &mut Text, old_epoch).is_err());
        let mut old_mount = release.clone();
        old_mount.capture.as_mut().unwrap().target.mount += 1;
        assert!(dispatch(&mut host, &mut Text, old_mount).is_err());
        assert_eq!(
            host.pointer_capture_lease(),
            Some((id, capture.target.mount, capture.serial))
        );
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<tekes_ui_controls::Button>()
                .unwrap()
                .activations,
            0
        );
        let released = dispatch(&mut host, &mut Text, release).unwrap();
        assert!(released.captured.is_none());
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<tekes_ui_controls::Button>()
                .unwrap()
                .activations,
            1
        );
    }
}
