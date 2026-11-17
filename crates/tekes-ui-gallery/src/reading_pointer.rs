//! Thin host adapter: preserves native pointer metadata at the public composition boundary.
use tekes_ui_controls::composition::ContentEvent;
use tekes_ui_macos_bridge::content_input;
use tekes_ui_platform_macos::MouseEvent;

pub fn translate(m: MouseEvent) -> ContentEvent {
    content_input::translate_pointer(m)
}
