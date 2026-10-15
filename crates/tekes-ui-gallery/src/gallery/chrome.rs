//! 固定的外壳：目录列、页面标题槽、内容框的右下边线。

use tekes_ui_scene::{Quad, Rect, Scene};
use tekes_ui_tokens::{ColorRole, CornerRadius, Rgba, Theme};

use super::containers::draw_label_placeholder;
use super::layout::{
    CATALOG_W, FRAME_H, FRAME_W, PAGE_TITLE_SLOT, catalog_label_slot, catalog_row_rect,
};
use super::pages::Page;

/// 目录里某一行的底色：选中用 `RowSelection`，没选中但 hover 用 `ButtonHover`，否则没有底。
/// 选中的行 hover 时保持选中色（不再变化，因此不用重绘）。
pub fn row_fill(theme: Theme, selected: bool, hovered: bool) -> Option<Rgba> {
    if selected {
        Some(ColorRole::RowSelection.resolve(theme))
    } else if hovered {
        Some(ColorRole::ButtonHover.resolve(theme))
    } else {
        None
    }
}

pub fn draw(scene: &mut Scene, theme: Theme, page: Page, hover: Option<usize>) {
    // 目录列底与右侧分隔线。
    scene.push_quad(Quad::new(
        Rect::new(0.0, 0.0, CATALOG_W, FRAME_H),
        ColorRole::TranscriptInput.resolve(theme),
    ));
    scene.push_quad(Quad::new(
        Rect::new(CATALOG_W - 1.0, 0.0, 1.0, FRAME_H),
        ColorRole::Separator.resolve(theme),
    ));

    // 每页一行。
    for (i, _) in Page::ALL.iter().enumerate() {
        let rect = catalog_row_rect(i);
        if let Some(fill) = row_fill(theme, page.index() == i, hover == Some(i)) {
            scene.push_quad(Quad::new(rect, fill).with_corner_radius(CornerRadius::Row.value()));
        }
        // 页面名的位置：F1 起排文字，现在是固定大小的占位条。
        draw_label_placeholder(scene, theme, catalog_label_slot(i), 96.0);
    }

    // 页面标题槽。
    draw_label_placeholder(scene, theme, PAGE_TITLE_SLOT, 120.0);

    // 内容框的右、下边线：窗口比 1200x800 大时，用它标出框的边界。
    let sep = ColorRole::Separator.resolve(theme);
    scene.push_quad(Quad::new(Rect::new(FRAME_W - 1.0, 0.0, 1.0, FRAME_H), sep));
    scene.push_quad(Quad::new(Rect::new(0.0, FRAME_H - 1.0, FRAME_W, 1.0), sep));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_fill_priority() {
        let t = Theme::LIGHT;
        assert_eq!(row_fill(t, false, false), None);
        assert_eq!(
            row_fill(t, false, true),
            Some(ColorRole::ButtonHover.resolve(t))
        );
        assert_eq!(
            row_fill(t, true, false),
            Some(ColorRole::RowSelection.resolve(t))
        );
        assert_eq!(row_fill(t, true, true), row_fill(t, true, false));
    }
}
