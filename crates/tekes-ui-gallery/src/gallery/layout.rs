//! 固定布局：几何常量与目录命中测试。
//!
//! Gallery 的内容永远排在 1200x800 pt 的固定框里，框锚在窗口左上角，**不随窗口缩放重排**：
//! 窗口更大就多露出背景，更小就裁掉右、下两侧。这样同一页每次长得一样，截图才能逐像素比较。
//! 坐标一律是逻辑点，左上角为原点，y 向下。
//!
//! ```text
//! 0                  220                                  1200
//! +------------------+-------------------------------------+ 0
//! | 目录列（固定宽） | 页面区                              |
//! |  行 0            |  页面标题槽                         |
//! |  行 1            |  页面内容 (244, 62) 932 x 714       |
//! |  ...             |                                     |
//! +------------------+-------------------------------------+ 800
//! ```

use tekes_ui_scene::{Point, Rect, Size};
use tekes_ui_tokens::{ControlSize, Spacing};

/// 固定内容框的尺寸（pt）。
pub const FRAME_W: f32 = 1200.0;
pub const FRAME_H: f32 = 800.0;

/// 目录列宽度：行内容 `TruncationSlot`（180）加列内边距（左右各 `Regular` 8）
/// 和行内边距（左右各 `Relaxed` 12），共 220。
pub const CATALOG_W: f32 = ControlSize::TruncationSlot.value()
    + 2.0 * Spacing::Regular.value()
    + 2.0 * Spacing::Relaxed.value();

/// 目录行高，等于 `ControlSize::RowHeight`。
pub const ROW_H: f32 = ControlSize::RowHeight.value();
/// 相邻两行的视觉间隔。命中区把间隔算进去（见 [`catalog_hit_rect`]），鼠标掠过间隔时不会闪烁。
pub const ROW_GAP: f32 = Spacing::Micro.value();
/// 目录第一行距顶部。
pub const CATALOG_TOP: f32 = Spacing::Loose.value();
/// 行距目录列左右边缘。
pub const ROW_INSET_X: f32 = Spacing::Regular.value();
/// 行内文字槽距行左缘（F1 放页面名）。
pub const ROW_PAD_X: f32 = Spacing::Relaxed.value();
/// 行内标签槽的尺寸；F1 起页面名排进这个槽，现在画一条固定大小的占位条。
pub const ROW_LABEL_SLOT: Size = Size::new(ControlSize::TruncationSlot.value(), 16.0);

/// 页面区左缘（目录列右侧再留 `Section`）。
pub const PAGE_X: f32 = CATALOG_W + Spacing::Section.value();
/// 页面标题槽（F1 放页面标题）。
pub const PAGE_TITLE_SLOT: Rect = Rect::new(PAGE_X, Spacing::Section.value(), 160.0, 14.0);
/// 页面内容区。页面内部再按自己的栅格排版。
pub const PAGE_CONTENT: Rect = Rect::new(
    PAGE_X,
    Spacing::Section.value() + 14.0 + Spacing::Section.value(),
    FRAME_W - PAGE_X - Spacing::Section.value(),
    FRAME_H
        - (Spacing::Section.value() + 14.0 + Spacing::Section.value())
        - Spacing::Section.value(),
);

/// 第 `i` 行的视觉矩形（选中、hover 的底）。
pub fn catalog_row_rect(i: usize) -> Rect {
    Rect::new(
        ROW_INSET_X,
        CATALOG_TOP + i as f32 * (ROW_H + ROW_GAP),
        CATALOG_W - 2.0 * ROW_INSET_X,
        ROW_H,
    )
}

/// 第 `i` 行的命中矩形：视觉矩形向外各扩半个间隔，相邻行的命中区正好拼起来。
/// `Rect::contains` 左、上包含，右、下不包含，所以一个点最多命中一行。
pub fn catalog_hit_rect(i: usize) -> Rect {
    catalog_row_rect(i).inset(-ROW_GAP / 2.0)
}

/// 点落在第几行（只考虑前 `count` 行）。不在任何行上返回 `None`。
pub fn catalog_hit_test(p: Point, count: usize) -> Option<usize> {
    (0..count).find(|&i| catalog_hit_rect(i).contains(p))
}

/// 第 `i` 行里的标签槽（垂直居中）。
pub fn catalog_label_slot(i: usize) -> Rect {
    let row = catalog_row_rect(i);
    Rect::new(
        row.min_x() + ROW_PAD_X,
        row.min_y() + (ROW_H - ROW_LABEL_SLOT.height) * 0.5,
        ROW_LABEL_SLOT.width.min(row.size.width - 2.0 * ROW_PAD_X),
        ROW_LABEL_SLOT.height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_width_is_220() {
        assert_eq!(CATALOG_W, 220.0);
    }

    #[test]
    fn page_content_fits_in_frame() {
        assert_eq!(PAGE_CONTENT.min_x(), 244.0);
        assert!(PAGE_CONTENT.max_x() <= FRAME_W);
        assert!(PAGE_CONTENT.max_y() <= FRAME_H);
    }

    #[test]
    fn hit_test_finds_rows() {
        for i in 0..3 {
            let r = catalog_row_rect(i);
            assert_eq!(catalog_hit_test(r.center(), 3), Some(i));
            // 左上角属于该行，右下角（不含）不属于。
            assert_eq!(catalog_hit_test(r.origin, 3), Some(i));
        }
    }

    #[test]
    fn hit_test_gap_belongs_to_exactly_one_row() {
        let r0 = catalog_row_rect(0);
        let r1 = catalog_row_rect(1);
        // 两行视觉矩形之间的间隔。
        let mid = Point::new(r0.center().x, (r0.max_y() + r1.min_y()) * 0.5);
        assert_eq!(catalog_hit_test(mid, 3), Some(1));
        let just_above = Point::new(mid.x, mid.y - 0.01);
        assert_eq!(catalog_hit_test(just_above, 3), Some(0));
    }

    #[test]
    fn hit_test_misses_outside_rows() {
        // 目录列顶部空白、行左右的边距、最后一行以下、页面区、窗口外。
        assert_eq!(catalog_hit_test(Point::new(100.0, 2.0), 2), None);
        assert_eq!(catalog_hit_test(Point::new(2.0, 30.0), 2), None);
        assert_eq!(catalog_hit_test(Point::new(CATALOG_W - 2.0, 30.0), 2), None);
        assert_eq!(catalog_hit_test(Point::new(100.0, 400.0), 2), None);
        assert_eq!(catalog_hit_test(Point::new(600.0, 30.0), 2), None);
        assert_eq!(catalog_hit_test(Point::new(-5.0, -5.0), 2), None);
        // 行数之外的行不命中。
        let third = catalog_row_rect(2).center();
        assert_eq!(catalog_hit_test(third, 2), None);
    }

    #[test]
    fn label_slot_is_inside_row() {
        let slot = catalog_label_slot(1);
        let row = catalog_row_rect(1);
        assert!(slot.min_x() >= row.min_x() && slot.max_x() <= row.max_x());
        assert!(slot.min_y() >= row.min_y() && slot.max_y() <= row.max_y());
    }
}
