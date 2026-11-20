//! Fixed Gallery calibration frame from the active proposal layout contract.
//! Larger windows expose background; smaller windows clip this reproducible frame.
//! The three existing pages remain calibration fixtures inside the new capture size.

use tekes_ui_scene::Rect;
use tekes_ui_tokens::{Spacing, proposal};

/// 固定内容框的尺寸（pt）。
pub const FRAME_W: f32 = proposal::CAPTURE_WIDTH;
pub const FRAME_H: f32 = proposal::CAPTURE_HEIGHT;

/// Sidebar default width from the proposal; truncation slots remain text fixtures.
pub const CATALOG_W: f32 = proposal::SIDEBAR_WIDTH;

/// 页面区左缘（目录列右侧再留 `Section`）。
pub const PAGE_X: f32 = CATALOG_W + Spacing::Section.value();
/// 页面内容区。页面内部再按自己的栅格排版。
pub const PAGE_CONTENT: Rect = Rect::new(
    PAGE_X,
    Spacing::Section.value()
        + proposal::TextRole::UiHeading.style().line_height
        + Spacing::Section.value(),
    FRAME_W - PAGE_X - Spacing::Section.value(),
    FRAME_H
        - (Spacing::Section.value()
            + proposal::TextRole::UiHeading.style().line_height
            + Spacing::Section.value())
        - Spacing::Section.value(),
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_width_uses_proposal_sidebar() {
        assert_eq!(CATALOG_W, 288.0);
    }

    #[test]
    fn page_content_fits_in_frame() {
        assert_eq!(PAGE_CONTENT.min_x(), 312.0);
        assert!(PAGE_CONTENT.max_x() <= FRAME_W);
        assert!(PAGE_CONTENT.max_y() <= FRAME_H);
    }
}
