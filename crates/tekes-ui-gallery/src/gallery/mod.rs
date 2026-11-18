//! Gallery registry, AppHost showcase host, text engine, and headless snapshot/perf support.

pub mod apphost;
#[cfg(all(target_os = "macos", feature = "selftest"))]
mod apphost_harness;
mod apphost_panels;
mod apphost_specimens;
pub mod apphost_workbench;
pub mod brands;
pub mod catalog;
pub mod containers;
pub mod icon_catalog;
pub mod layout;
pub mod locale;
pub mod motion_preview;
pub mod navigation;
pub mod pages;
#[cfg(test)]
mod quality_effects;
mod scene3d_preview;
pub mod snapshot;
pub mod static_text;
pub mod text;
pub mod text_demo;
pub mod theme;
pub mod workbench;

#[cfg(feature = "selftest")]
pub use layout::{FRAME_H as CONTENT_HEIGHT, FRAME_W as CONTENT_WIDTH};
#[cfg(not(test))]
pub use pages::Page;
#[cfg(test)]
pub use pages::{Page, PageCtx};

/// Shared Gallery canvas color helper retained after retiring the legacy Gallery window host.
#[cfg(test)]
pub struct Gallery;

#[cfg(test)]
impl Gallery {
    pub fn background(theme: tekes_ui_tokens::Theme) -> tekes_ui_tokens::Rgba {
        tekes_ui_tokens::ColorRole::DocumentCanvas.resolve(theme)
    }
}
