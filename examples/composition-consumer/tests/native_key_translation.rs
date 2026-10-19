#![cfg(target_os = "macos")]
#[path = "support/reading_key.rs"]
mod gallery;
#[path = "../examples/support/native_key.rs"]
mod independent;
use tekes_ui_controls::{ControlKey, composition::*};
use tekes_ui_platform_macos::{KeyEvent, KeyEventKind, Modifiers, TextKeyContext, TextKeyEvent};
fn native() -> KeyEvent {
    KeyEvent {
        kind: KeyEventKind::Down,
        key_code: 36,
        characters: "\r".into(),
        characters_ignoring_modifiers: "\r".into(),
        is_repeat: true,
        modifiers: Modifiers {
            shift: true,
            command: true,
            alt: true,
            control: true,
            caps_lock: true,
            function: true,
        },
    }
}
#[test]
fn real_host_adapters_preserve_all_modifiers_repeat_up_down_and_unmapped_character_keys() {
    for kind in [KeyEventKind::Down, KeyEventKind::Up] {
        for code in [36, 76, 48, 0] {
            let mut n = native();
            n.kind = kind;
            n.key_code = code;
            n.characters = "é".into();
            n.characters_ignoring_modifiers = "e".into();
            for translate in [gallery::key, independent::key] {
                let k = translate(n.clone());
                assert_eq!(k.native_code, code);
                assert_eq!(k.down, kind == KeyEventKind::Down);
                assert!(k.repeat);
                assert_eq!(k.characters, "é");
                assert_eq!(k.characters_ignoring_modifiers, "e");
                assert_eq!(
                    k.modifiers,
                    InputModifiers {
                        shift: true,
                        command: true,
                        alt: true,
                        control: true,
                        caps_lock: true,
                        function: true
                    }
                );
                assert_eq!(
                    k.key,
                    match code {
                        36 | 76 => Some(ControlKey::Enter),
                        48 => Some(ControlKey::Tab { reverse: true }),
                        _ => None,
                    }
                );
            }
        }
    }
}
#[test]
fn callback_delivery_never_gets_mislabelled_as_opaque_input_consumption() {
    use tekes_ui_platform_macos::TextKeyDisposition as D;
    let context = TextKeyContext {
        id: 99,
        key: native(),
        composing_at_start: true,
    };
    for translate in [gallery::translate, independent::translate] {
        let ContentEvent::TextKeyStart { id, key, composing } =
            translate(TextKeyEvent::Started(context.clone()))
        else {
            panic!("start")
        };
        assert_eq!(id, 99);
        assert!(composing);
        assert!(key.modifiers.command);
        for d in [
            D::TextDispatched,
            D::CommandDispatched,
            D::InputContextConsumed,
            D::Unhandled,
            D::NotOffered,
        ] {
            let ContentEvent::TextKeyEnd {
                id,
                input_context_consumed,
            } = translate(TextKeyEvent::Finished {
                context: context.clone(),
                input_context_handled: Some(true),
                disposition: d,
                text_callbacks: 0,
                command_callbacks: 0,
            })
            else {
                panic!("end")
            };
            assert_eq!(id, 99);
            assert_eq!(input_context_consumed, d == D::InputContextConsumed);
        }
    }
}
