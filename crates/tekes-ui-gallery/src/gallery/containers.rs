//! 页面容器：以后每个组件页都放在这两个容器里。
//!
//! - [`StateMatrix`]：固定网格，列是 rest / hover / pressed / disabled / focus，
//!   每个单元格尺寸固定，样本由调用方按 [`ComponentState`] 画。
//! - [`ReferenceSlot`]：并排的固定尺寸窗格 “TekesUI live | reference image | diff heatmap”。
//!
//! 列头、行头、窗格标题、节标题都是真文字（`tekes-ui-text`，字号取自 `TextRole`），排在固定大小的槽里，
//! 布局不随文字变化。

use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, CornerRadius, Rgba, Spacing, Stroke, TextRole, Theme};

use super::text::{LabelStyle, TextEngine};

/// 节标题样式：`SectionHeader`（11pt 粗），次要文字色。
pub const SECTION_STYLE: LabelStyle =
    LabelStyle::new(TextRole::SectionHeader, ColorRole::SecondaryText);
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

/// 节标题：一个槽，返回槽下沿（内容从这里往下排）。
pub const SECTION_HEADER_H: f32 = 18.0;
/// 节标题槽的宽度。
pub const SECTION_HEADER_W: f32 = 240.0;
/// 节标题槽的高度。
pub const SECTION_SLOT_H: f32 = 14.0;

pub fn draw_section_header(
    scene: &mut Scene,
    theme: Theme,
    text: &mut TextEngine,
    x: f32,
    y: f32,
    label: &str,
) {
    text.label_in(
        scene,
        theme,
        Rect::new(x, y, SECTION_HEADER_W, SECTION_SLOT_H),
        label,
        SECTION_STYLE,
    );
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

    /// 画出整个矩阵。`row_labels[row]` 是行头文字；
    /// `sample(scene, row, cell, state)` 在每个单元格里画样本。
    pub fn draw(
        &self,
        scene: &mut Scene,
        theme: Theme,
        text: &mut TextEngine,
        row_labels: &[&str],
        mut sample: impl FnMut(&mut Scene, usize, Rect, ComponentState),
    ) {
        for (col, state) in ComponentState::ALL.iter().enumerate() {
            text.label_in(
                scene,
                theme,
                self.header_slot(col),
                state.name(),
                META_STYLE,
            );
        }
        for row in 0..self.rows {
            let label = row_labels.get(row).copied().unwrap_or("");
            text.label_in(scene, theme, self.row_label_slot(row), label, META_STYLE);
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

/// 参照槽的窗格标题。
pub const PANE_TITLES: [&str; 3] = ["TekesUI 实时", "参照图", "差异热图"];

/// 窗格的排列方向。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotLayout {
    /// 左右并排（默认；窗格 280 x 200）。
    Horizontal,
    /// 上下叠放。静态文字页用它：700 pt 宽的文字放不下三个并排的窗格。
    Vertical,
}

/// 参照槽：TekesUI 实时渲染 | 参照图 | 差异热力图。
///
/// 每个窗格上面一行标题槽（文字），下面是窗格。窗格的底色：实时窗格是画布色，
/// 参照窗格是淡底色（等着画参照图），差异窗格只有边框。
/// 图像图元（scene 的 `Image`）由页面按窗格矩形自己画，窗格尺寸和位置不变。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferenceSlot {
    pub origin: Point,
    /// 是否画第三个差异窗格。
    pub show_diff: bool,
    pub layout: SlotLayout,
    /// 窗格尺寸（内容区；边框画在它外面 `frame_outset` 处）。
    pub pane: Size,
    /// 边框向外扩多少。默认 0（边框画在窗格矩形上）；静态文字页用 1，让文字区正好是文字画布。
    pub frame_outset: f32,
    /// 边框的圆角。
    pub corner_radius: f32,
}

impl ReferenceSlot {
    pub const PANE: Size = Size::new(280.0, 200.0);
    pub const HEADER_H: f32 = 14.0;
    const GAP: f32 = Spacing::Relaxed.value();
    const HEADER_GAP: f32 = Spacing::Regular.value();

    pub const fn new(origin: Point, show_diff: bool) -> ReferenceSlot {
        ReferenceSlot {
            origin,
            show_diff,
            layout: SlotLayout::Horizontal,
            pane: Self::PANE,
            frame_outset: 0.0,
            corner_radius: CornerRadius::Panel.value(),
        }
    }

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

    pub fn pane_count(&self) -> usize {
        if self.show_diff { 3 } else { 2 }
    }

    #[cfg(test)]
    pub fn size(&self) -> Size {
        let n = self.pane_count() as f32;
        match self.layout {
            SlotLayout::Horizontal => Size::new(
                n * self.pane.width + (n - 1.0) * Self::GAP,
                Self::HEADER_H + Self::HEADER_GAP + self.pane.height,
            ),
            SlotLayout::Vertical => Size::new(
                self.pane.width,
                n * (Self::HEADER_H + Self::HEADER_GAP + self.pane.height) + (n - 1.0) * Self::GAP,
            ),
        }
    }

    fn block_origin(&self, i: usize) -> Point {
        let i = i as f32;
        match self.layout {
            SlotLayout::Horizontal => Point::new(
                self.origin.x + i * (self.pane.width + Self::GAP),
                self.origin.y,
            ),
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

    /// 画出整个槽。`titles[i]` 是第 `i` 个窗格的标题；`live(scene, pane)` 在第一个窗格里画 TekesUI 的实时内容。
    pub fn draw(
        &self,
        scene: &mut Scene,
        theme: Theme,
        text: &mut TextEngine,
        titles: &[&str],
        live: impl FnOnce(&mut Scene, &mut TextEngine, Rect),
    ) {
        let radius = self.corner_radius;
        let border = Stroke::Hairline.value();
        for i in 0..self.pane_count() {
            let title = titles.get(i).copied().unwrap_or("");
            text.label_in(scene, theme, self.header_slot(i), title, META_STYLE);
            let frame = self.pane_rect(i).inset(-self.frame_outset);
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
                Quad::new(frame, fill)
                    .with_corner_radius(radius)
                    .with_border(border, border_color),
            );
        }
        live(scene, text, self.pane_rect(0));
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
        let mut text = TextEngine::new();
        m.draw(
            &mut scene,
            Theme::LIGHT,
            &mut text,
            &["a", "b"],
            |_, row, cell, state| {
                seen.push((row, cell, state));
            },
        );
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
    fn vertical_slot_stacks_panes_without_overlap() {
        let s = ReferenceSlot::vertical(Point::new(10.0, 20.0), Size::new(700.0, 170.0));
        assert_eq!(s.pane_count(), 3);
        for i in 0..2 {
            let a = s.pane_rect(i);
            let b = s.pane_rect(i + 1);
            assert_eq!(a.min_x(), b.min_x());
            // 下一个窗格的标题在上一个窗格的下面。
            assert!(a.max_y() < s.header_slot(i + 1).min_y());
            assert!(s.header_slot(i + 1).max_y() <= b.min_y());
        }
        let size = s.size();
        assert_eq!(s.pane_rect(2).max_y(), 20.0 + size.height);
        assert_eq!(s.pane_rect(0).size, Size::new(700.0, 170.0));
        // 整数位置：文字画布和参照图都要对齐像素。
        for i in 0..3 {
            let r = s.pane_rect(i);
            assert_eq!(r.origin.x, r.origin.x.round());
            assert_eq!(r.origin.y, r.origin.y.round());
        }
    }

    #[test]
    fn reference_slot_draw_calls_live_with_first_pane() {
        let s = ReferenceSlot::new(Point::new(5.0, 6.0), true);
        let mut scene = Scene::new();
        let mut got = None;
        let mut text = TextEngine::new();
        s.draw(
            &mut scene,
            Theme::DARK,
            &mut text,
            &PANE_TITLES,
            |_, _, pane| got = Some(pane),
        );
        assert_eq!(got, Some(s.pane_rect(0)));
    }
}
