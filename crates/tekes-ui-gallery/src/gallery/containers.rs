//! 页面容器：以后每个组件页都放在这两个容器里。
//!
//! - [`StateMatrix`]：固定网格，列是 rest / hover / pressed / disabled / focus，
//!   每个单元格尺寸固定，样本由调用方按 [`ComponentState`] 画。
//! - [`ReferenceSlot`]：并排的固定尺寸窗格 “TekesUI live | reference image | diff heatmap”。
//!
//! F0 没有文字，列头、行头、窗格标题都只画固定大小的占位条（[`draw_label_placeholder`]）；
//! F1 把文字排进同一个槽就行，布局不变。

use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Alpha, ColorRole, CornerRadius, Rgba, Spacing, Stroke, Theme};

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
    /// 状态矩阵的列序。
    pub const ALL: [ComponentState; 5] = [
        ComponentState::Rest,
        ComponentState::Hover,
        ComponentState::Pressed,
        ComponentState::Disabled,
        ComponentState::Focus,
    ];
}

/// 标签槽里的占位条：高 8pt，宽不超过 `max_w`，在槽里垂直居中、靠左。
/// F1 起文字排进 `slot` 本身，占位条随之删除。
pub fn draw_label_placeholder(scene: &mut Scene, theme: Theme, slot: Rect, max_w: f32) {
    let h = 8.0_f32.min(slot.size.height);
    let bar = Rect::new(
        slot.min_x(),
        slot.min_y() + (slot.size.height - h) * 0.5,
        max_w.min(slot.size.width),
        h,
    );
    let color = ColorRole::SecondaryText
        .resolve(theme)
        .with_alpha(Alpha::Soft.resolve(theme.contrast));
    scene.push_quad(Quad::new(bar, color).with_corner_radius(CornerRadius::AgentMarker.value()));
}

/// 节标题：一个槽加占位条，返回槽下沿（内容从这里往下排）。
pub const SECTION_HEADER_H: f32 = 18.0;

pub fn draw_section_header(scene: &mut Scene, theme: Theme, x: f32, y: f32) {
    draw_label_placeholder(scene, theme, Rect::new(x, y, 160.0, 10.0), 120.0);
}

const fn transparent() -> Rgba {
    Rgba::new(0.0, 0.0, 0.0, 0.0)
}

/// 状态矩阵。左边一列行头槽，上面一行列头槽，中间是 `rows x 5` 的单元格。
///
/// 所有尺寸都是常量，行数以外不随内容变化。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateMatrix {
    pub origin: Point,
    pub rows: usize,
}

impl StateMatrix {
    pub const ROW_LABEL_W: f32 = 96.0;
    pub const CELL: Size = Size::new(120.0, 56.0);
    pub const HEADER_H: f32 = 14.0;
    pub const COLUMNS: usize = ComponentState::ALL.len();
    const GAP: f32 = Spacing::Relaxed.value();
    const HEADER_GAP: f32 = Spacing::Regular.value();

    pub const fn new(origin: Point, rows: usize) -> StateMatrix {
        StateMatrix { origin, rows }
    }

    /// 整个矩阵的外框尺寸。
    pub fn size(&self) -> Size {
        let cols = Self::COLUMNS as f32;
        let rows = self.rows as f32;
        Size::new(
            Self::ROW_LABEL_W + Self::GAP + cols * Self::CELL.width + (cols - 1.0) * Self::GAP,
            Self::HEADER_H
                + Self::HEADER_GAP
                + rows * Self::CELL.height
                + (rows - 1.0).max(0.0) * Self::GAP,
        )
    }

    /// 第 `col` 列的列头槽。
    pub fn header_slot(&self, col: usize) -> Rect {
        Rect::new(
            self.cell_x(col),
            self.origin.y,
            Self::CELL.width,
            Self::HEADER_H,
        )
    }

    /// 第 `row` 行的行头槽（垂直居中于该行的单元格）。
    pub fn row_label_slot(&self, row: usize) -> Rect {
        let cell = self.cell_rect(row, 0);
        Rect::new(
            self.origin.x,
            cell.min_y() + (Self::CELL.height - Self::HEADER_H) * 0.5,
            Self::ROW_LABEL_W,
            Self::HEADER_H,
        )
    }

    /// 单元格。
    pub fn cell_rect(&self, row: usize, col: usize) -> Rect {
        Rect::new(
            self.cell_x(col),
            self.origin.y
                + Self::HEADER_H
                + Self::HEADER_GAP
                + row as f32 * (Self::CELL.height + Self::GAP),
            Self::CELL.width,
            Self::CELL.height,
        )
    }

    fn cell_x(&self, col: usize) -> f32 {
        self.origin.x + Self::ROW_LABEL_W + Self::GAP + col as f32 * (Self::CELL.width + Self::GAP)
    }

    /// 画出整个矩阵。`sample(scene, row, cell, state)` 在每个单元格里画样本。
    pub fn draw(
        &self,
        scene: &mut Scene,
        theme: Theme,
        mut sample: impl FnMut(&mut Scene, usize, Rect, ComponentState),
    ) {
        for (col, _) in ComponentState::ALL.iter().enumerate() {
            draw_label_placeholder(scene, theme, self.header_slot(col), 56.0);
        }
        for row in 0..self.rows {
            draw_label_placeholder(scene, theme, self.row_label_slot(row), 64.0);
            for (col, state) in ComponentState::ALL.iter().enumerate() {
                let cell = self.cell_rect(row, col);
                scene.push_quad(
                    Quad::new(cell, transparent())
                        .with_corner_radius(CornerRadius::Row.value())
                        .with_border(
                            Stroke::Hairline.value(),
                            ColorRole::FloatingPanelDivider.resolve(theme),
                        ),
                );
                sample(scene, row, cell, *state);
            }
        }
    }
}

/// 并排参照槽：TekesUI 实时渲染 | 参照图 | 差异热力图。
///
/// 现在还没有图像图元（scene 的 `Image`，见 scene 的 crate 文档），参照窗格只画一个有边框、
/// 带淡底色的空框，让槽位可见；差异窗格只画边框。图像图元落地后，参照窗格改成按 id 画参照图，
/// 差异窗格画热力图，窗格尺寸和位置不变。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferenceSlot {
    pub origin: Point,
    /// 是否画第三个差异窗格。
    pub show_diff: bool,
}

impl ReferenceSlot {
    pub const PANE: Size = Size::new(280.0, 200.0);
    pub const HEADER_H: f32 = 14.0;
    const GAP: f32 = Spacing::Relaxed.value();
    const HEADER_GAP: f32 = Spacing::Regular.value();

    pub const fn new(origin: Point, show_diff: bool) -> ReferenceSlot {
        ReferenceSlot { origin, show_diff }
    }

    pub fn pane_count(&self) -> usize {
        if self.show_diff { 3 } else { 2 }
    }

    #[cfg(test)]
    pub fn size(&self) -> Size {
        let n = self.pane_count() as f32;
        Size::new(
            n * Self::PANE.width + (n - 1.0) * Self::GAP,
            Self::HEADER_H + Self::HEADER_GAP + Self::PANE.height,
        )
    }

    pub fn header_slot(&self, i: usize) -> Rect {
        Rect::new(
            self.pane_x(i),
            self.origin.y,
            Self::PANE.width,
            Self::HEADER_H,
        )
    }

    pub fn pane_rect(&self, i: usize) -> Rect {
        Rect::new(
            self.pane_x(i),
            self.origin.y + Self::HEADER_H + Self::HEADER_GAP,
            Self::PANE.width,
            Self::PANE.height,
        )
    }

    fn pane_x(&self, i: usize) -> f32 {
        self.origin.x + i as f32 * (Self::PANE.width + Self::GAP)
    }

    /// 画出整个槽。`live(scene, pane)` 在第一个窗格里画 TekesUI 的实时内容。
    pub fn draw(&self, scene: &mut Scene, theme: Theme, live: impl FnOnce(&mut Scene, Rect)) {
        let radius = CornerRadius::Panel.value();
        let border = Stroke::Hairline.value();
        for i in 0..self.pane_count() {
            draw_label_placeholder(scene, theme, self.header_slot(i), 88.0);
            let pane = self.pane_rect(i);
            let (fill, border_color) = match i {
                // 实时窗格：画布底色。
                0 => (
                    ColorRole::DocumentCanvas.resolve(theme),
                    ColorRole::Border.resolve(theme),
                ),
                // 参照窗格：淡底色的空框。
                1 => (
                    ColorRole::HoverWash.resolve(theme),
                    ColorRole::Border.resolve(theme),
                ),
                // 差异窗格：只有边框。
                _ => (transparent(), ColorRole::Border.resolve(theme)),
            };
            scene.push_quad(
                Quad::new(pane, fill)
                    .with_corner_radius(radius)
                    .with_border(border, border_color),
            );
        }
        live(scene, self.pane_rect(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_has_five_state_columns_in_order() {
        assert_eq!(StateMatrix::COLUMNS, 5);
        assert_eq!(
            ComponentState::ALL,
            [
                ComponentState::Rest,
                ComponentState::Hover,
                ComponentState::Pressed,
                ComponentState::Disabled,
                ComponentState::Focus
            ]
        );
    }

    #[test]
    fn matrix_cells_do_not_overlap_and_fit_in_size() {
        let m = StateMatrix::new(Point::new(10.0, 20.0), 3);
        let size = m.size();
        let outer = Rect::new(10.0, 20.0, size.width, size.height);
        let mut cells = Vec::new();
        for r in 0..3 {
            for c in 0..5 {
                let cell = m.cell_rect(r, c);
                assert_eq!(cell.size, StateMatrix::CELL);
                assert_eq!(cell.intersection(&outer), Some(cell));
                cells.push(cell);
            }
        }
        for i in 0..cells.len() {
            for j in i + 1..cells.len() {
                assert_eq!(cells[i].intersection(&cells[j]), None);
            }
        }
        // 最后一个单元格正好贴着外框右、下边。
        let last = m.cell_rect(2, 4);
        assert_eq!(last.max_x(), outer.max_x());
        assert_eq!(last.max_y(), outer.max_y());
    }

    #[test]
    fn matrix_header_slots_sit_above_their_columns() {
        let m = StateMatrix::new(Point::new(0.0, 0.0), 1);
        for c in 0..5 {
            let h = m.header_slot(c);
            let cell = m.cell_rect(0, c);
            assert_eq!(h.min_x(), cell.min_x());
            assert!(h.max_y() <= cell.min_y());
        }
        assert!(m.row_label_slot(0).max_x() <= m.cell_rect(0, 0).min_x());
    }

    #[test]
    fn matrix_draw_calls_sample_for_every_cell_in_state_order() {
        let m = StateMatrix::new(Point::new(0.0, 0.0), 2);
        let mut scene = Scene::new();
        let mut seen = Vec::new();
        m.draw(&mut scene, Theme::LIGHT, |_, row, cell, state| {
            seen.push((row, cell, state));
        });
        assert_eq!(seen.len(), 10);
        for (i, (row, cell, state)) in seen.iter().enumerate() {
            assert_eq!(*row, i / 5);
            assert_eq!(*state, ComponentState::ALL[i % 5]);
            assert_eq!(*cell, m.cell_rect(i / 5, i % 5));
        }
    }

    #[test]
    fn reference_slot_panes_are_side_by_side() {
        let s = ReferenceSlot::new(Point::new(0.0, 0.0), true);
        assert_eq!(s.pane_count(), 3);
        for i in 0..2 {
            let a = s.pane_rect(i);
            let b = s.pane_rect(i + 1);
            assert_eq!(a.min_y(), b.min_y());
            assert!(a.max_x() < b.min_x());
        }
        let size = s.size();
        assert_eq!(s.pane_rect(2).max_x(), size.width);
        assert_eq!(s.pane_rect(2).max_y(), size.height);
        assert_eq!(
            ReferenceSlot::new(Point::new(0.0, 0.0), false).pane_count(),
            2
        );
    }

    #[test]
    fn reference_slot_draw_calls_live_with_first_pane() {
        let s = ReferenceSlot::new(Point::new(5.0, 6.0), true);
        let mut scene = Scene::new();
        let mut got = None;
        s.draw(&mut scene, Theme::DARK, |_, pane| got = Some(pane));
        assert_eq!(got, Some(s.pane_rect(0)));
    }
}
