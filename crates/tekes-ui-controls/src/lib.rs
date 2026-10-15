//! Rust-owned scrolling and text state; platform events and drawing are adapters.
mod scroll_view;
mod text_input;
pub use scroll_view::ScrollView as ScrollArea;
pub use text_input::TextInput;
