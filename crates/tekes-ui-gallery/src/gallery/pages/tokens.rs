//! Token swatches 页：把 token 摊开，目测核对。
//!
//! 自上而下：
//! 1. 27 个 `ColorRole` 色块，3 行 x 9 列，顺序固定为 `ColorRole::ALL`（枚举声明顺序，
//!    即 brief 里的三组：完全中立、半中立、系统依赖），色块下面是角色名（驼峰拆成词，
//!    按色块宽度换行，最多两行）。每个色块都放在 `DocumentCanvas` 底的方砖上并带 hairline 描边，半透明的角色
//!    （边框、分隔线、各种 wash）才看得见；
//! 2. 间距刻度：`Spacing` 八档，每档一根条，条长 = 值 x 8，左边是“名字 值”；
//! 3. 圆角一排：`CornerRadius` 按枚举声明顺序，下面是“名字 值”；
//! 4. 描边一排：`Stroke` 三档，既有描边框也有同宽度的横线，框下面是“名字 值”。

use tekes_ui_scene::{Quad, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, CornerRadius, Rgba, Spacing, Stroke};

use crate::gallery::containers::{META_STYLE, SECTION_HEADER_H, draw_section_header};
use crate::gallery::pages::PageCtx;

/// 色块网格列数。
pub const SWATCH_COLUMNS: usize = 9;
const SWATCH: Size = Size::new(88.0, 56.0);
const GAP: f32 = Spacing::Relaxed.value();
/// 页内各节之间的间隔（页面内容放得下四节加两行色块名，所以比 `Spacing::Section` 小）。
const SECTION_GAP: f32 = 20.0;
/// 色块下面的标签槽：距方砖 4pt，两行（RowMeta 一行 13pt）共 26pt。
const LABEL_OFFSET: f32 = 60.0;
pub const LABEL_H: f32 = 26.0;
const SWATCH_ROW_PITCH: f32 = LABEL_OFFSET + LABEL_H + 8.0;
/// 圆角、描边那两排的名字槽：距方砖 4pt，一行 13pt。
const NAME_OFFSET: f32 = 60.0;
const NAME_H: f32 = 13.0;
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

/// 色块名：`ScrimX` 这样的驼峰拆成 `Scrim X`，让换行落在词边界上。
pub fn role_label(role: ColorRole) -> String {
    let name = format!("{role:?}");
    let mut out = String::with_capacity(name.len() + 3);
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// “名字 值”，值是整数时不带小数点。
pub fn name_value(name: &str, value: f32) -> String {
    format!("{name} {value}")
}

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

pub fn build(ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene) {
    let theme = ctx.theme;
    let text = &mut *ctx.text;
    let x = content.min_x();
    let mut y = content.min_y();
    let hairline = Stroke::Hairline.value();
    let hairline_color = ColorRole::FloatingPanelBorder.resolve(theme);

    // 1. 色块。
    draw_section_header(scene, theme, text, x, y, "颜色角色（ColorRole::ALL 顺序）");
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
        text.wrapped_label_in(
            scene,
            theme,
            Rect::new(
                tile.min_x(),
                tile.min_y() + LABEL_OFFSET,
                SWATCH.width,
                LABEL_H,
            ),
            &role_label(*role),
            META_STYLE,
        );
    }
    let rows = ColorRole::ALL.len().div_ceil(SWATCH_COLUMNS);
    y += SECTION_HEADER_H + (rows as f32 * SWATCH_ROW_PITCH - GAP) + SECTION_GAP;

    // 2. 间距刻度。
    draw_section_header(scene, theme, text, x, y, "间距 Spacing（条长 = 值 x 8）");
    let top = y + SECTION_HEADER_H;
    let pitch = SPACING_BAR_H + Spacing::Regular.value();
    for (i, s) in SPACING_ORDER.iter().enumerate() {
        let by = top + i as f32 * pitch;
        text.label_in(
            scene,
            theme,
            Rect::new(x, by, 76.0, SPACING_BAR_H),
            &name_value(&format!("{s:?}"), s.value()),
            META_STYLE,
        );
        // 值为 0 的一档画一条 hairline 宽的标记，免得整行空着。
        let w = (s.value() * SPACING_BAR_SCALE).max(hairline);
        scene.push_quad(Quad::new(
            Rect::new(x + 84.0, by, w, SPACING_BAR_H),
            ColorRole::Accent.resolve(theme),
        ));
    }
    y += SECTION_HEADER_H
        + (SPACING_ORDER.len() as f32 * pitch - Spacing::Regular.value())
        + SECTION_GAP;

    // 3. 圆角。
    draw_section_header(scene, theme, text, x, y, "圆角 CornerRadius");
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
        text.label_in(
            scene,
            theme,
            Rect::new(
                x + i as f32 * (SWATCH.width + GAP),
                top + NAME_OFFSET,
                SWATCH.width,
                NAME_H,
            ),
            &name_value(&format!("{r:?}"), r.value()),
            META_STYLE,
        );
    }
    y += SECTION_HEADER_H + NAME_OFFSET + NAME_H + SECTION_GAP;

    // 4. 描边：三个描边框，然后是三条同宽度的横线（整数 y，对齐像素）。
    draw_section_header(scene, theme, text, x, y, "描边 Stroke");
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
        text.label_in(
            scene,
            theme,
            Rect::new(
                x + i as f32 * (SWATCH.width + GAP),
                top + NAME_OFFSET,
                SWATCH.width,
                NAME_H,
            ),
            &name_value(&format!("{s:?}"), s.value()),
            META_STYLE,
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
    use crate::gallery::static_text::StaticText;
    use crate::gallery::text::TextEngine;
    use tekes_ui_scene::Primitive;
    use tekes_ui_tokens::Theme;

    fn scene_with(theme: Theme, text: &mut TextEngine) -> Scene {
        let mut st = StaticText::new(None);
        let mut ctx = PageCtx {
            theme,
            text,
            static_text: &mut st,
        };
        let mut s = Scene::new();
        ctx.text.begin_frame();
        build(&mut ctx, PAGE_CONTENT, &mut s);
        s
    }

    fn quads(s: &Scene) -> Vec<tekes_ui_scene::Quad> {
        s.primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Quad(q) => Some(*q),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn swatch_grid_holds_all_27_roles_in_enum_order() {
        assert_eq!(ColorRole::ALL.len(), 27);
        assert_eq!(SWATCH_COLUMNS * 3, 27);
        let mut text = TextEngine::new();
        for theme in Theme::ALL {
            let q = quads(&scene_with(theme, &mut text));
            // 每格 2 个 quad：方砖、色块（名字是字形，不是 quad）。
            for (i, role) in ColorRole::ALL.iter().enumerate() {
                let swatch = &q[2 * i + 1];
                assert_eq!(swatch.fill, role.resolve(theme), "{role:?} {theme:?}");
                let tile = &q[2 * i];
                assert_eq!(tile.fill, ColorRole::DocumentCanvas.resolve(theme));
                assert!(tile.rect.intersection(&swatch.rect) == Some(swatch.rect));
            }
        }
    }

    #[test]
    fn page_fits_in_content() {
        let mut text = TextEngine::new();
        let s = scene_with(Theme::LIGHT, &mut text);
        for p in s.primitives() {
            match p {
                Primitive::Quad(q) => {
                    assert!(q.rect.max_x() <= PAGE_CONTENT.max_x() + 0.01, "{q:?}");
                    assert!(q.rect.max_y() <= PAGE_CONTENT.max_y() + 0.01, "{q:?}");
                }
                Primitive::Glyphs(run) => {
                    for g in &run.glyphs {
                        // 字形原点（基线）在内容区内；整行放得下由 labels_fit_their_slots 保证。
                        assert!(g.origin.x >= PAGE_CONTENT.min_x() - 0.01);
                        assert!(g.origin.x <= PAGE_CONTENT.max_x(), "{g:?}");
                        assert!(g.origin.y <= PAGE_CONTENT.max_y(), "{g:?}");
                    }
                }
                _ => {}
            }
        }
    }

    #[test]
    fn color_names_wrap_into_at_most_two_lines_within_the_swatch_width() {
        let mut text = TextEngine::new();
        for role in ColorRole::ALL {
            let label = role_label(role);
            let p = text.label(
                &label,
                tekes_ui_tokens::TextRole::RowMeta,
                Some(SWATCH.width),
            );
            assert!(p.lines.len() <= 2, "{label}：{} 行", p.lines.len());
            assert!(p.width <= f64::from(SWATCH.width) + 0.01, "{label}");
            assert!(p.height <= f64::from(LABEL_H), "{label}：高 {}", p.height);
        }
        assert_eq!(
            role_label(ColorRole::FloatingPanelDivider),
            "Floating Panel Divider"
        );
        assert_eq!(role_label(ColorRole::Scrim), "Scrim");
    }

    #[test]
    fn token_name_labels_fit_their_slots() {
        let mut text = TextEngine::new();
        let meta = tekes_ui_tokens::TextRole::RowMeta;
        for r in RADIUS_ORDER {
            let p = text.label(&name_value(&format!("{r:?}"), r.value()), meta, None);
            assert!(p.width <= f64::from(SWATCH.width), "{r:?} {}", p.width);
        }
        for s in SPACING_ORDER {
            let p = text.label(&name_value(&format!("{s:?}"), s.value()), meta, None);
            assert!(p.width <= 76.0, "{s:?} {}", p.width);
        }
        for s in [Stroke::Hairline, Stroke::Rule, Stroke::Emphasis] {
            let p = text.label(&name_value(&format!("{s:?}"), s.value()), meta, None);
            assert!(p.width <= f64::from(SWATCH.width), "{s:?} {}", p.width);
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
