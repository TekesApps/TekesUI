//! Token swatches 页：把 token 摊开，目测核对。
//!
//! 自上而下：
//! 1. 27 个 `ColorRole` 色块，3 行 x 9 列，顺序固定为 `ColorRole::ALL`（枚举声明顺序，
//!    即 brief 里的三组：完全中立、半中立、系统依赖）。F0 没有文字，色块下面是标签槽占位条；
//!    要知道某格是哪个角色，数格子对 `ColorRole::ALL`。
//!    每个色块都放在 `DocumentCanvas` 底的方砖上并带 hairline 描边，半透明的角色
//!    （边框、分隔线、各种 wash）才看得见；
//! 2. 间距刻度：`Spacing` 八档，每档一根条，条长 = 值 x 8；
//! 3. 圆角一排：`CornerRadius` 按枚举声明顺序；
//! 4. 描边一排：`Stroke` 三档，既有描边框也有同宽度的横线。

use tekes_ui_scene::{Quad, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, CornerRadius, Rgba, Spacing, Stroke, Theme};

use crate::gallery::containers::{SECTION_HEADER_H, draw_label_placeholder, draw_section_header};

/// 色块网格列数。
pub const SWATCH_COLUMNS: usize = 9;
const SWATCH: Size = Size::new(88.0, 56.0);
const GAP: f32 = Spacing::Relaxed.value();
const SECTION_GAP: f32 = Spacing::Section.value();
/// 色块下面的标签槽：距方砖 6pt，高 8pt。
const LABEL_OFFSET: f32 = 62.0;
const LABEL_H: f32 = 8.0;
const SWATCH_ROW_PITCH: f32 = LABEL_OFFSET + LABEL_H + GAP;
/// 间距条每 pt 画多长。
pub const SPACING_BAR_SCALE: f32 = 8.0;
const SPACING_BAR_H: f32 = 12.0;

/// 圆角排的顺序：枚举声明顺序。
pub const RADIUS_ORDER: [CornerRadius; 9] = [
    CornerRadius::Square,
    CornerRadius::AgentMarker,
    CornerRadius::Badge,
    CornerRadius::Row,
    CornerRadius::Control,
    CornerRadius::Input,
    CornerRadius::Panel,
    CornerRadius::Column,
    CornerRadius::Floating,
];

/// 间距条的顺序：枚举声明顺序，从小到大。
pub const SPACING_ORDER: [Spacing; 8] = [
    Spacing::None,
    Spacing::Micro,
    Spacing::Tight,
    Spacing::Snug,
    Spacing::Regular,
    Spacing::Relaxed,
    Spacing::Loose,
    Spacing::Section,
];

const fn clear() -> Rgba {
    Rgba::new(0.0, 0.0, 0.0, 0.0)
}

/// 第 `i` 个色块方砖的位置（`i` 是 `ColorRole::ALL` 里的下标）。
pub fn swatch_rect(origin_x: f32, origin_y: f32, i: usize) -> Rect {
    let (row, col) = (i / SWATCH_COLUMNS, i % SWATCH_COLUMNS);
    Rect::new(
        origin_x + col as f32 * (SWATCH.width + GAP),
        origin_y + row as f32 * SWATCH_ROW_PITCH,
        SWATCH.width,
        SWATCH.height,
    )
}

pub fn build(theme: Theme, content: Rect, scene: &mut Scene) {
    let x = content.min_x();
    let mut y = content.min_y();
    let hairline = Stroke::Hairline.value();
    let hairline_color = ColorRole::FloatingPanelBorder.resolve(theme);

    // 1. 色块。
    draw_section_header(scene, theme, x, y);
    let top = y + SECTION_HEADER_H;
    for (i, role) in ColorRole::ALL.iter().enumerate() {
        let tile = swatch_rect(x, top, i);
        scene.push_quad(
            Quad::new(tile, ColorRole::DocumentCanvas.resolve(theme))
                .with_corner_radius(CornerRadius::Row.value())
                .with_border(hairline, hairline_color),
        );
        scene.push_quad(
            Quad::new(tile.inset(Spacing::Tight.value()), role.resolve(theme))
                .with_corner_radius(CornerRadius::concentric(
                    CornerRadius::Row.value(),
                    Spacing::Tight.value(),
                ))
                .with_border(hairline, hairline_color),
        );
        draw_label_placeholder(
            scene,
            theme,
            Rect::new(
                tile.min_x(),
                tile.min_y() + LABEL_OFFSET,
                SWATCH.width,
                LABEL_H,
            ),
            48.0,
        );
    }
    let rows = ColorRole::ALL.len().div_ceil(SWATCH_COLUMNS);
    y += SECTION_HEADER_H + (rows as f32 * SWATCH_ROW_PITCH - GAP) + SECTION_GAP;

    // 2. 间距刻度。
    draw_section_header(scene, theme, x, y);
    let top = y + SECTION_HEADER_H;
    let pitch = SPACING_BAR_H + Spacing::Regular.value();
    for (i, s) in SPACING_ORDER.iter().enumerate() {
        let by = top + i as f32 * pitch;
        draw_label_placeholder(scene, theme, Rect::new(x, by, 64.0, SPACING_BAR_H), 40.0);
        // 值为 0 的一档画一条 hairline 宽的标记，免得整行空着。
        let w = (s.value() * SPACING_BAR_SCALE).max(hairline);
        scene.push_quad(Quad::new(
            Rect::new(x + 80.0, by, w, SPACING_BAR_H),
            ColorRole::Accent.resolve(theme),
        ));
    }
    y += SECTION_HEADER_H
        + (SPACING_ORDER.len() as f32 * pitch - Spacing::Regular.value())
        + SECTION_GAP;

    // 3. 圆角。
    draw_section_header(scene, theme, x, y);
    let top = y + SECTION_HEADER_H;
    for (i, r) in RADIUS_ORDER.iter().enumerate() {
        scene.push_quad(
            Quad::new(
                Rect::new(
                    x + i as f32 * (SWATCH.width + GAP),
                    top,
                    SWATCH.width,
                    SWATCH.height,
                ),
                ColorRole::RowSelection.resolve(theme),
            )
            .with_corner_radius(r.value())
            .with_border(Stroke::Rule.value(), ColorRole::Border.resolve(theme)),
        );
    }
    y += SECTION_HEADER_H + SWATCH.height + SECTION_GAP;

    // 4. 描边：三个描边框，然后是三条同宽度的横线（整数 y，对齐像素）。
    draw_section_header(scene, theme, x, y);
    let top = y + SECTION_HEADER_H;
    let strokes = [Stroke::Hairline, Stroke::Rule, Stroke::Emphasis];
    for (i, s) in strokes.iter().enumerate() {
        scene.push_quad(
            Quad::new(
                Rect::new(
                    x + i as f32 * (SWATCH.width + GAP),
                    top,
                    SWATCH.width,
                    SWATCH.height,
                ),
                clear(),
            )
            .with_corner_radius(CornerRadius::Row.value())
            .with_border(s.value(), ColorRole::BorderFocused.resolve(theme)),
        );
    }
    let line_x = x + 3.0 * (SWATCH.width + GAP) + SECTION_GAP;
    for (i, s) in strokes.iter().enumerate() {
        scene.push_quad(Quad::new(
            Rect::new(line_x, top + 12.0 + i as f32 * 16.0, 240.0, s.value()),
            ColorRole::PrimaryText.resolve(theme),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::layout::PAGE_CONTENT;

    fn scene(theme: Theme) -> Scene {
        let mut s = Scene::new();
        build(theme, PAGE_CONTENT, &mut s);
        s
    }

    #[test]
    fn swatch_grid_holds_all_27_roles_in_enum_order() {
        assert_eq!(ColorRole::ALL.len(), 27);
        assert_eq!(SWATCH_COLUMNS * 3, 27);
        for theme in Theme::ALL {
            let s = scene(theme);
            // 节标题占 1 个图元，之后每格 3 个：方砖、色块、标签条。
            for (i, role) in ColorRole::ALL.iter().enumerate() {
                let tekes_ui_scene::Primitive::Quad(q) = &s.primitives()[1 + 3 * i + 1] else {
                    unreachable!()
                };
                assert_eq!(q.fill, role.resolve(theme), "{role:?} {theme:?}");
                let tekes_ui_scene::Primitive::Quad(tile) = &s.primitives()[1 + 3 * i] else {
                    unreachable!()
                };
                assert_eq!(tile.fill, ColorRole::DocumentCanvas.resolve(theme));
                assert!(tile.rect.intersection(&q.rect) == Some(q.rect));
            }
        }
    }

    #[test]
    fn page_fits_in_content() {
        let s = scene(Theme::LIGHT);
        for p in s.primitives() {
            let tekes_ui_scene::Primitive::Quad(q) = p else {
                continue;
            };
            assert!(q.rect.max_x() <= PAGE_CONTENT.max_x() + 0.01, "{q:?}");
            assert!(q.rect.max_y() <= PAGE_CONTENT.max_y() + 0.01, "{q:?}");
        }
    }

    #[test]
    fn swatch_rects_are_in_row_major_order() {
        let a = swatch_rect(0.0, 0.0, 0);
        let b = swatch_rect(0.0, 0.0, 1);
        let c = swatch_rect(0.0, 0.0, SWATCH_COLUMNS);
        assert!(b.min_x() > a.max_x() && b.min_y() == a.min_y());
        assert!(c.min_y() > a.max_y() && c.min_x() == a.min_x());
    }
}
