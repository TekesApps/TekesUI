//! Thin host conversion only; editor policy and acknowledgement live in ContentHost.
use tekes_ui_controls::composition::{ContentEvent, ContentKeyEvent};
use tekes_ui_macos_bridge::content_input;
use tekes_ui_platform_macos::{KeyEvent, TextKeyEvent};

pub fn key(k: KeyEvent) -> ContentKeyEvent {
    content_input::translate_key(k)
}

pub fn translate(event: TextKeyEvent) -> ContentEvent {
    content_input::translate_text_key(event)
}
