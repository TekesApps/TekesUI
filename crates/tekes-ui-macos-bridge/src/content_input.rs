//! Single-process input and clipboard translation between [`tekes_ui_platform_macos`] and
//! [`tekes_ui_controls::composition`].

#[cfg(target_os = "macos")]
use crate::content_accessibility::content_action;
#[cfg(target_os = "macos")]
use tekes_ui_controls::{ButtonTextRenderer, composition::CompositionError};
use tekes_ui_controls::{
    ControlKey,
    composition::{
        ContentEvent, ContentHost, ContentKeyEvent, ContentResponse, ContentWheelEvent,
        ContentWheelPhase, InputModifiers, PointerButton, PointerEvent, PointerPhase,
    },
};
#[cfg(target_os = "macos")]
use tekes_ui_platform_macos::Window;
use tekes_ui_platform_macos::{
    Event, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind, Point, ScrollEvent,
    ScrollPhase, Size, TextInputState, TextKeyDisposition, TextKeyEvent,
};
use tekes_ui_scene::{Point as ScenePoint, Rect as SceneRect};

fn translate_modifiers(modifiers: tekes_ui_platform_macos::Modifiers) -> InputModifiers {
    InputModifiers {
        shift: modifiers.shift,
        control: modifiers.control,
        alt: modifiers.alt,
        command: modifiers.command,
        caps_lock: modifiers.caps_lock,
        function: modifiers.function,
    }
}

fn translate_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Other(n) => PointerButton::Other(n),
    }
}

fn pointer_event(mouse: &MouseEvent) -> PointerEvent {
    let (phase, button, click_count) = match mouse.kind {
        MouseEventKind::Down {
            button,
            click_count,
        } => (
            PointerPhase::Down,
            Some(translate_button(button)),
            click_count,
        ),
        MouseEventKind::Up {
            button,
            click_count,
        } => (
            PointerPhase::Up,
            Some(translate_button(button)),
            click_count,
        ),
        MouseEventKind::Dragged { button } => {
            (PointerPhase::Drag, Some(translate_button(button)), 0)
        }
        MouseEventKind::Moved => (PointerPhase::Move, None, 0),
        MouseEventKind::Entered => (PointerPhase::Enter, None, 0),
        MouseEventKind::Exited => (PointerPhase::Exit, None, 0),
    };
    PointerEvent {
        position: ScenePoint::new(mouse.position.x as f32, mouse.position.y as f32),
        phase,
        button,
        click_count,
        modifiers: translate_modifiers(mouse.modifiers),
    }
}

/// Convert a native [`KeyEvent`] into [`ContentKeyEvent`], preserving modifiers, repeat,
/// raw keycode, and character strings.
pub fn translate_key(k: KeyEvent) -> ContentKeyEvent {
    ContentKeyEvent {
        key: match k.key_code {
            36 | 76 => Some(ControlKey::Enter),
            49 => Some(ControlKey::Space),
            48 => Some(ControlKey::Tab {
                reverse: k.modifiers.shift,
            }),
            53 => Some(ControlKey::Escape),
            123 => Some(ControlKey::Left),
            124 => Some(ControlKey::Right),
            125 => Some(ControlKey::Down),
            126 => Some(ControlKey::Up),
            115 => Some(ControlKey::Home),
            119 => Some(ControlKey::End),
            _ => None,
        },
        down: k.kind == KeyEventKind::Down,
        repeat: k.is_repeat,
        modifiers: translate_modifiers(k.modifiers),
        native_code: k.key_code,
        characters: k.characters,
        characters_ignoring_modifiers: k.characters_ignoring_modifiers,
    }
}

/// Convert a native [`TextKeyEvent`] (key-down start / AppKit disposition end) into
/// [`ContentEvent`].
pub fn translate_text_key(event: TextKeyEvent) -> ContentEvent {
    match event {
        TextKeyEvent::Started(context) => ContentEvent::TextKeyStart {
            id: context.id,
            key: translate_key(context.key),
            composing: context.composing_at_start,
        },
        TextKeyEvent::Finished {
            context,
            disposition,
            ..
        } => ContentEvent::TextKeyEnd {
            id: context.id,
            input_context_consumed: disposition == TextKeyDisposition::InputContextConsumed,
        },
    }
}

/// Convert a native [`MouseEvent`] into `ContentEvent::Pointer`.
pub fn translate_pointer(mouse: MouseEvent) -> ContentEvent {
    ContentEvent::Pointer(pointer_event(&mouse))
}

fn valid_rect(r: SceneRect) -> bool {
    [
        r.min_x(),
        r.min_y(),
        r.size.width,
        r.size.height,
        r.max_x(),
        r.max_y(),
    ]
    .iter()
    .all(|v| v.is_finite())
        && r.size.width >= 0.
        && r.size.height >= 0.
}

fn utf16_boundary(text: &str, index: usize) -> bool {
    let mut count = 0;
    if index == 0 {
        return true;
    }
    for ch in text.chars() {
        count += ch.len_utf16();
        if count == index {
            return true;
        }
        if count > index {
            return false;
        }
    }
    false
}

/// Window adapters publish one original Host query, without branching by component.
pub fn content_text_input_state(host: &ContentHost) -> Option<TextInputState> {
    let state = host.text_input_snapshot()?;
    let valid_range = |r: &std::ops::Range<usize>| {
        r.start <= r.end
            && utf16_boundary(&state.text, r.start)
            && utf16_boundary(&state.text, r.end)
    };
    if !utf16_boundary(&state.text, state.caret_index)
        || !valid_range(&state.selection)
        || state.marked.as_ref().is_some_and(|r| !valid_range(r))
    {
        return None;
    }
    let geometry = |r: SceneRect| -> Option<(Point, Size)> {
        if !valid_rect(r) {
            return None;
        }
        let clip = state.clip.filter(|clip| valid_rect(*clip))?;
        let r = r.intersection(&clip)?;
        if r.size.height <= 0. {
            return None;
        }
        Some((
            Point {
                x: r.min_x().into(),
                y: r.min_y().into(),
            },
            Size {
                width: r.size.width.into(),
                height: r.size.height.into(),
            },
        ))
    };
    let (caret_origin, caret_size) = state.caret.and_then(geometry).unwrap_or_default();
    let mut carets = Vec::new();
    for (index, r) in state.carets {
        if !utf16_boundary(&state.text, index) {
            return None;
        }
        if let Some((p, s)) = geometry(r) {
            carets.push((index, p, s));
        }
    }
    Some(TextInputState {
        text: state.text,
        caret_index: state.caret_index,
        selection: state.selection,
        marked: state.marked,
        read_only: state.read_only,
        caret_origin,
        caret_size,
        carets,
    })
}

/// Preserve local native wheel phase facts for the original ContentHost owner.
/// IPC compatibility packets remain unchanged; this does not arbitrate containers.
pub fn content_wheel_event(scroll: &ScrollEvent) -> Option<ContentWheelEvent> {
    let phase = |phase| match phase {
        ScrollPhase::None => ContentWheelPhase::None,
        ScrollPhase::MayBegin => ContentWheelPhase::MayBegin,
        ScrollPhase::Began => ContentWheelPhase::Began,
        ScrollPhase::Ended => ContentWheelPhase::Ended,
        ScrollPhase::Cancelled => ContentWheelPhase::Cancelled,
        ScrollPhase::Changed | ScrollPhase::Stationary | ScrollPhase::Other(_) => {
            ContentWheelPhase::Changed
        }
    };
    let position = ScenePoint::new(scroll.position.x as f32, scroll.position.y as f32);
    let mut delta = ScenePoint::new(-scroll.delta.x as f32, -scroll.delta.y as f32);
    if scroll.modifiers.shift {
        delta = ScenePoint::new(delta.x + delta.y, 0.);
    }
    if !position.x.is_finite()
        || !position.y.is_finite()
        || !delta.x.is_finite()
        || !delta.y.is_finite()
        || !scroll.timestamp.is_finite()
        || scroll.timestamp < 0.
    {
        return None;
    }
    Some(ContentWheelEvent {
        position,
        delta,
        precise: scroll.precise,
        phase: phase(scroll.phase),
        momentum_phase: phase(scroll.momentum_phase),
        time: scroll.timestamp,
    })
}

/// One native-to-Content conversion for single-process callers.
pub fn content_event(event: &Event) -> Option<ContentEvent> {
    match event {
        Event::Scroll(scroll) => content_wheel_event(scroll).map(|wheel| ContentEvent::Scroll {
            position: wheel.position,
            delta: wheel.delta,
            precise: wheel.precise,
            begin: matches!(
                wheel.phase,
                ContentWheelPhase::MayBegin | ContentWheelPhase::Began
            ),
            end: matches!(
                wheel.phase,
                ContentWheelPhase::Ended | ContentWheelPhase::Cancelled
            ) || matches!(
                wheel.momentum_phase,
                ContentWheelPhase::Ended | ContentWheelPhase::Cancelled
            ),
            momentum: wheel.momentum_phase != ContentWheelPhase::None,
            time: wheel.time,
        }),
        Event::Mouse(mouse) => {
            let pointer = pointer_event(mouse);
            (pointer.position.x.is_finite() && pointer.position.y.is_finite())
                .then_some(ContentEvent::Pointer(pointer))
        }
        Event::Key(key) => {
            if key.kind == KeyEventKind::Down
                && key.modifiers.command
                && !key.modifiers.control
                && !key.modifiers.alt
            {
                let command = match key
                    .characters_ignoring_modifiers
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "a" => Some("selectAll:"),
                    "z" if key.modifiers.shift => Some("redo:"),
                    "z" => Some("undo:"),
                    _ => None,
                };
                if let Some(command) = command {
                    return Some(ContentEvent::TextCommand(command.into()));
                }
            }
            Some(ContentEvent::KeyInput(translate_key(key.clone())))
        }
        Event::TextKey(text_key) => Some(translate_text_key(text_key.clone())),
        Event::TextCommit { text, replacement } => Some(ContentEvent::Commit {
            text: text.clone(),
            replacement: replacement.clone(),
        }),
        Event::TextPreedit {
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
        Event::TextUnmark => Some(ContentEvent::Unmark),
        Event::TextCommand(command) => Some(ContentEvent::TextCommand(command.clone())),
        _ => None,
    }
}

/// Native clipboard transport; editing and key ownership remain in [`ContentHost`].
pub trait ContentClipboard {
    fn clipboard_text(&self) -> Option<String>;
    fn set_clipboard_text(&self, text: &str);
}

#[cfg(target_os = "macos")]
impl ContentClipboard for Window {
    fn clipboard_text(&self) -> Option<String> {
        Window::clipboard_text(self)
    }
    fn set_clipboard_text(&self, text: &str) {
        Window::set_clipboard_text(self, text);
    }
}

/// One window-level adapter for single-process [`ContentHost`], including the native
/// clipboard capability. Components retain all editing/key state.
#[cfg(target_os = "macos")]
pub fn dispatch_content_window_event(
    host: &mut ContentHost,
    event: &Event,
    window: &Window,
) -> Option<ContentResponse> {
    host.set_window_active(window.is_active());
    if let Event::Accessibility { target, action } = event {
        return Some(content_action(host, *target, action.clone()));
    }
    dispatch_content_event(host, event, window)
}

/// Local container dispatch that preserves its existing font backend and pre/post input
/// layout, including before a clipboard claim reads the owner's snapshot.
#[cfg(target_os = "macos")]
pub fn dispatch_content_window_event_with_text(
    host: &mut ContentHost,
    event: &Event,
    clipboard: &dyn ContentClipboard,
    text: &mut dyn ButtonTextRenderer,
) -> Result<Option<ContentResponse>, CompositionError> {
    host.with_text_input(text, |host| dispatch_content_event(host, event, clipboard))
}

#[cfg(target_os = "macos")]
fn dispatch_content_event(
    host: &mut ContentHost,
    event: &Event,
    clipboard: &dyn ContentClipboard,
) -> Option<ContentResponse> {
    if let Event::Scroll(scroll) = event {
        return content_wheel_event(scroll).map(|wheel| host.wheel_event(&wheel));
    }
    let command = match event {
        Event::TextCommand(command) if matches!(command.as_str(), "copy:" | "cut:" | "paste:") => {
            Some(command.as_str())
        }
        Event::Key(key)
            if key.kind == KeyEventKind::Down
                && key.modifiers.command
                && !key.modifiers.control
                && !key.modifiers.alt =>
        {
            match key.characters_ignoring_modifiers.as_str() {
                "c" | "C" => Some("copy:"),
                "x" | "X" => Some("cut:"),
                "v" | "V" => Some("paste:"),
                _ => None,
            }
        }
        _ => None,
    };
    if let Some(command) = command {
        let Some(snapshot) = host.claim_clipboard_input() else {
            return Some(ContentResponse::consumed(false));
        };
        if snapshot.selection.start > snapshot.selection.end
            || !utf16_boundary(&snapshot.text, snapshot.selection.start)
            || !utf16_boundary(&snapshot.text, snapshot.selection.end)
        {
            return Some(ContentResponse::consumed(false));
        }
        if command == "paste:" {
            return Some(if !snapshot.read_only {
                clipboard
                    .clipboard_text()
                    .map(|text| {
                        host.event(&ContentEvent::Commit {
                            text,
                            replacement: None,
                        })
                    })
                    .unwrap_or_else(|| ContentResponse::consumed(false))
            } else {
                ContentResponse::consumed(false)
            });
        }
        if snapshot.selection.is_empty() || (command == "cut:" && snapshot.read_only) {
            return Some(ContentResponse::consumed(false));
        }
        let utf16: Vec<_> = snapshot.text.encode_utf16().collect();
        let selected = String::from_utf16(&utf16[snapshot.selection.clone()]).ok()?;
        clipboard.set_clipboard_text(&selected);
        return Some(if command == "cut:" {
            host.event(&ContentEvent::Commit {
                text: String::new(),
                replacement: Some(snapshot.selection),
            })
        } else {
            ContentResponse::consumed(false)
        });
    }
    content_event(event).map(|event| host.event(&event))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_platform_macos::{Modifiers, Vec2};

    #[test]
    fn wheel_translation_keeps_units_phases_and_invalid_finiteness_rejections() {
        let input = ScrollEvent {
            timestamp: 123.5,
            delta: Vec2 { x: -2., y: 16. },
            precise: true,
            phase: ScrollPhase::Began,
            momentum_phase: ScrollPhase::Changed,
            inverted: true,
            position: Point { x: 120.5, y: 75.25 },
            modifiers: Modifiers::NONE,
        };
        let Some(ContentEvent::Scroll {
            position,
            delta,
            precise,
            begin,
            end,
            momentum,
            time,
        }) = content_event(&Event::Scroll(input))
        else {
            panic!("expected ContentEvent::Scroll");
        };
        assert_eq!(position.x, 120.5);
        assert_eq!(delta.y, -16.);
        assert!(precise && begin && momentum && !end);
        assert_eq!(time, 123.5);

        let mut cancelled = input;
        cancelled.phase = ScrollPhase::Cancelled;
        cancelled.momentum_phase = ScrollPhase::None;
        assert!(matches!(
            content_event(&Event::Scroll(cancelled)),
            Some(ContentEvent::Scroll {
                end: true,
                begin: false,
                momentum: false,
                ..
            })
        ));
        for invalid in [f64::NAN, f64::INFINITY, f64::MAX] {
            let bad = ScrollEvent {
                delta: Vec2 { x: 0., y: invalid },
                ..input
            };
            assert!(content_event(&Event::Scroll(bad)).is_none());
        }
    }

    #[test]
    fn pointer_translation_preserves_buttons_counts_and_modifiers() {
        let modifiers = Modifiers {
            shift: true,
            control: true,
            alt: true,
            command: true,
            caps_lock: true,
            function: true,
        };
        for (button, expected) in [
            (MouseButton::Left, PointerButton::Primary),
            (MouseButton::Right, PointerButton::Secondary),
            (MouseButton::Middle, PointerButton::Middle),
            (MouseButton::Other(7), PointerButton::Other(7)),
        ] {
            let ContentEvent::Pointer(result) = translate_pointer(MouseEvent {
                kind: MouseEventKind::Down {
                    button,
                    click_count: 3,
                },
                position: Point { x: 123.5, y: 47.25 },
                modifiers,
            }) else {
                panic!("expected ContentEvent::Pointer");
            };
            assert_eq!(result.phase, PointerPhase::Down);
            assert_eq!(result.button, Some(expected));
            assert_eq!(result.click_count, 3);
            assert_eq!(result.position, ScenePoint::new(123.5, 47.25));
            assert!(
                result.modifiers.shift
                    && result.modifiers.control
                    && result.modifiers.alt
                    && result.modifiers.command
                    && result.modifiers.caps_lock
                    && result.modifiers.function
            );
        }
    }

    #[test]
    fn key_and_text_key_translation_preserve_mappings_and_dispositions() {
        let raw = KeyEvent {
            kind: KeyEventKind::Down,
            key_code: 48,
            characters: "\t".into(),
            characters_ignoring_modifiers: "\t".into(),
            is_repeat: true,
            modifiers: Modifiers {
                shift: true,
                ..Modifiers::NONE
            },
        };
        let translated = translate_key(raw.clone());
        assert_eq!(translated.key, Some(ControlKey::Tab { reverse: true }));
        assert!(translated.down && translated.repeat);

        let cmd_z = KeyEvent {
            kind: KeyEventKind::Down,
            key_code: 6,
            characters: "z".into(),
            characters_ignoring_modifiers: "Z".into(),
            is_repeat: false,
            modifiers: Modifiers {
                command: true,
                shift: true,
                ..Modifiers::NONE
            },
        };
        assert_eq!(
            content_event(&Event::Key(cmd_z)),
            Some(ContentEvent::TextCommand("redo:".into()))
        );
    }
}
