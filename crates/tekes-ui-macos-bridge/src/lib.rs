//! Thin macOS bridge connecting [`tekes_ui_platform_macos`] and [`tekes_ui_controls`]:
//!
//! - [`content_accessibility`]: projects [`tekes_ui_controls::composition::ContentHost`] semantics
//!   into platform accessibility nodes/actions and implements
//!   [`tekes_ui_platform_macos::AccessibilityGeometryLease`].
//! - [`content_input`]: translates single-process platform events (pointer, keyboard, scroll,
//!   text key, clipboard) to composition events without IPC machinery.
//! - [`coretext_host`]: shared CoreText shaping, glyph atlas rasterization, and
//!   [`tekes_ui_controls::ButtonTextRenderer`] / [`tekes_ui_controls::TextSurfaceHost`] support.

pub mod content_accessibility;
pub mod content_input;
#[cfg(target_os = "macos")]
pub mod coretext_host;

#[cfg(target_os = "macos")]
pub use content_accessibility::WindowContentAccessibilityExt;
pub use content_input::{
    ContentClipboard, content_event, content_text_input_state, content_wheel_event,
    parse_objc_text_command, translate_key, translate_pointer, translate_text_command,
    translate_text_key,
};
#[cfg(target_os = "macos")]
pub use content_input::{dispatch_content_window_event, dispatch_content_window_event_with_text};
