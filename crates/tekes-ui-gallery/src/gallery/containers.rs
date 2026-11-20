//! 共享页容器样式与静态文字纵排参照槽。

use tekes_ui_scene::{Point, Rect, Size};
use tekes_ui_tokens::{ColorRole, Spacing, TextRole};

use super::text::LabelStyle;

/// 列头、行头、窗格标题样式：`RowMeta`（11pt），次要文字色。
pub const META_STYLE: LabelStyle = LabelStyle::new(TextRole::RowMeta, ColorRole::SecondaryText);
/// 注释样式：`RowMeta`，第三级文字色。
pub const NOTE_STYLE: LabelStyle = LabelStyle::new(TextRole::RowMeta, ColorRole::TertiaryText);

/// 组件状态：状态矩阵的列。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComponentState {
    Rest,
    Hover,
    Pressed,
    Disabled,
    Focus,
}

impl ComponentState {
    /// 列头上的名字。
    pub const fn name(self) -> &'static str {
        match self {
            ComponentState::Rest => "rest",
            ComponentState::Hover => "hover",
            ComponentState::Pressed => "pressed",
            ComponentState::Disabled => "disabled",
            ComponentState::Focus => "focus",
        }
    }

    /// 状态矩阵的列序。
    pub const ALL: [ComponentState; 5] = [
        ComponentState::Rest,
        ComponentState::Hover,
        ComponentState::Pressed,
        ComponentState::Disabled,
        ComponentState::Focus,
    ];
}

/// 参照槽的窗格标题。
pub const PANE_TITLES: [&str; 3] = ["TekesUI 实时", "参照图", "差异热图"];

/// 窗格的排列方向。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotLayout {
    /// 上下叠放。静态文字页用它：700 pt 宽的文字放不下三个并排的窗格。
    Vertical,
}

/// 参照槽：TekesUI 实时渲染 | 参照图 | 差异热力图。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferenceSlot {
    pub origin: Point,
    /// 是否画第三个差异窗格。
    pub show_diff: bool,
    pub layout: SlotLayout,
    /// 窗格尺寸（内容区；边框画在它外面 `frame_outset` 处）。
    pub pane: Size,
    /// 边框向外扩多少。静态文字页用 1，让文字区正好是文字画布。
    pub frame_outset: f32,
    /// 边框的圆角。
    pub corner_radius: f32,
}

impl ReferenceSlot {
    pub const HEADER_H: f32 = 14.0;
    const GAP: f32 = Spacing::Relaxed.value();
    const HEADER_GAP: f32 = Spacing::Regular.value();

    /// 上下叠放的槽，窗格 `pane` 大小，边框画在外面 1pt。
    pub const fn vertical(origin: Point, pane: Size) -> ReferenceSlot {
        ReferenceSlot {
            origin,
            show_diff: true,
            layout: SlotLayout::Vertical,
            pane,
            frame_outset: 1.0,
            corner_radius: 0.0,
        }
    }

    fn block_origin(&self, i: usize) -> Point {
        let i = i as f32;
        match self.layout {
            SlotLayout::Vertical => Point::new(
                self.origin.x,
                self.origin.y
                    + i * (Self::HEADER_H + Self::HEADER_GAP + self.pane.height + Self::GAP),
            ),
        }
    }

    pub fn header_slot(&self, i: usize) -> Rect {
        let o = self.block_origin(i);
        Rect::new(o.x, o.y, self.pane.width, Self::HEADER_H)
    }

    pub fn pane_rect(&self, i: usize) -> Rect {
        let o = self.block_origin(i);
        Rect::new(
            o.x,
            o.y + Self::HEADER_H + Self::HEADER_GAP,
            self.pane.width,
            self.pane.height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_state_order_matches_matrix_columns() {
        assert_eq!(
            ComponentState::ALL.map(ComponentState::name),
            ["rest", "hover", "pressed", "disabled", "focus"]
        );
    }

    #[test]
    fn vertical_slot_stacks_panes_without_overlap() {
        let s = ReferenceSlot::vertical(Point::new(10.0, 20.0), Size::new(700.0, 170.0));
        for i in 0..2 {
            let a = s.pane_rect(i);
            let b = s.pane_rect(i + 1);
            assert_eq!(a.min_x(), b.min_x());
            // 下一个窗格的标题在上一个窗格的下面。
            assert!(a.max_y() < s.header_slot(i + 1).min_y());
            assert!(s.header_slot(i + 1).max_y() <= b.min_y());
        }
        assert_eq!(s.pane_rect(0).size, Size::new(700.0, 170.0));
        // 整数位置：文字画布和参照图都要对齐像素。
        for i in 0..3 {
            let r = s.pane_rect(i);
            assert_eq!(r.origin.x, r.origin.x.round());
            assert_eq!(r.origin.y, r.origin.y.round());
        }
    }
}
