//! Primitives 页：验证渲染层的基本图元和 token 的组合。
//!
//! 自上而下的节（每节顶上一个节标题槽）：
//! 1. 直角矩形和各档 `CornerRadius` 的圆角矩形；
//! 2. `Stroke` 三档描边；`Alpha` 三档半透明叠放；
//! 3. 状态矩阵：一个“类按钮”的 quad 在 rest / hover / pressed / disabled / focus 下的样子；
//! 4. 并排参照槽：同一个样本，左边是 TekesUI 实时渲染，右边是参照图的空占位。
//!
//! 类按钮样本按 brief 的决定画：rest 透明，hover 用 `ButtonHover`，pressed 用 `HoverWash`
//! 取 `Alpha::Soft`，disabled 整体 45% 不透明度（`Alpha::Wash`），focus 是 1pt 的 `BorderFocused`
//! 轮廓；没有系统焦点环，没有阴影。样本带一圈 hairline `Border`（brief 没规定，Gallery 自己加的，
//! 否则 rest 和 disabled 是看不见的），focus 时这圈换成 `BorderFocused`。样本里的短条是文字占位。

use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Alpha, ColorRole, CornerRadius, Rgba, Spacing, Stroke, Theme};

use crate::gallery::containers::{
    ComponentState, ReferenceSlot, SECTION_HEADER_H, StateMatrix, draw_section_header,
};

/// 圆角一排：直角加七档 token（badge、row、panel、floating、input、column、control）。
pub const RADII: [(&str, f32); 8] = [
    ("square", 0.0),
    ("badge", CornerRadius::Badge.value()),
    ("row", CornerRadius::Row.value()),
    ("panel", CornerRadius::Panel.value()),
    ("floating", CornerRadius::Floating.value()),
    ("input", CornerRadius::Input.value()),
    ("column", CornerRadius::Column.value()),
    ("control", CornerRadius::Control.value()),
];

const ITEM: Size = Size::new(96.0, 56.0);
const GAP: f32 = Spacing::Relaxed.value();
const SECTION_GAP: f32 = Spacing::Section.value();

/// 类按钮样本的尺寸（`PillHeight` 高）。
pub const BUTTON: Size = Size::new(96.0, 28.0);

fn scale_alpha(c: Rgba, k: f32) -> Rgba {
    c.with_alpha(c.a * k)
}

/// 在 `cell` 中央画一个类按钮样本。
pub fn button_sample(scene: &mut Scene, cell: Rect, state: ComponentState, theme: Theme) {
    let c = cell.center();
    let rect = Rect::new(
        c.x - BUTTON.width * 0.5,
        c.y - BUTTON.height * 0.5,
        BUTTON.width,
        BUTTON.height,
    );
    let fill = match state {
        ComponentState::Hover => ColorRole::ButtonHover.resolve(theme),
        ComponentState::Pressed => ColorRole::HoverWash
            .resolve(theme)
            .with_alpha(Alpha::Soft.value()),
        _ => Rgba::new(0.0, 0.0, 0.0, 0.0),
    };
    let (border_w, border) = if state == ComponentState::Focus {
        (
            Stroke::Rule.value(),
            ColorRole::BorderFocused.resolve(theme),
        )
    } else {
        (Stroke::Hairline.value(), ColorRole::Border.resolve(theme))
    };
    let mut ink = ColorRole::PrimaryText.resolve(theme);
    let (mut fill, mut border) = (fill, border);
    if state == ComponentState::Disabled {
        let k = Alpha::Wash.value();
        fill = scale_alpha(fill, k);
        border = scale_alpha(border, k);
        ink = scale_alpha(ink, k);
    }
    scene.push_quad(
        Quad::new(rect, fill)
            .with_corner_radius(CornerRadius::Row.value())
            .with_border(border_w, border),
    );
    // 文字占位条：左对齐，距左缘 Relaxed，垂直居中。
    scene.push_quad(
        Quad::new(
            Rect::new(
                rect.min_x() + Spacing::Relaxed.value(),
                c.y - 4.0,
                48.0,
                8.0,
            ),
            ink,
        )
        .with_corner_radius(CornerRadius::AgentMarker.value()),
    );
}

pub fn build(theme: Theme, content: Rect, scene: &mut Scene) {
    let x = content.min_x();
    let mut y = content.min_y();

    // 1. 圆角。
    draw_section_header(scene, theme, x, y);
    for (i, (_, radius)) in RADII.iter().enumerate() {
        let r = Rect::new(
            x + i as f32 * (ITEM.width + GAP),
            y + SECTION_HEADER_H,
            ITEM.width,
            ITEM.height,
        );
        scene.push_quad(
            Quad::new(r, ColorRole::RowSelection.resolve(theme))
                .with_corner_radius(*radius)
                .with_border(Stroke::Rule.value(), ColorRole::Border.resolve(theme)),
        );
    }
    y += SECTION_HEADER_H + ITEM.height + SECTION_GAP;

    // 2. 描边三档，然后是三档 alpha 叠放。
    draw_section_header(scene, theme, x, y);
    let top = y + SECTION_HEADER_H;
    for (i, stroke) in [Stroke::Hairline, Stroke::Rule, Stroke::Emphasis]
        .iter()
        .enumerate()
    {
        let r = Rect::new(
            x + i as f32 * (ITEM.width + GAP),
            top,
            ITEM.width,
            ITEM.height,
        );
        scene.push_quad(
            Quad::new(r, Rgba::new(0.0, 0.0, 0.0, 0.0))
                .with_corner_radius(CornerRadius::Row.value())
                .with_border(stroke.value(), ColorRole::BorderFocused.resolve(theme)),
        );
    }
    let overlap_x = x + 3.0 * (ITEM.width + GAP) + SECTION_GAP;
    for (i, alpha) in [Alpha::Faint, Alpha::Soft, Alpha::Wash].iter().enumerate() {
        let a = alpha.resolve(theme.contrast);
        let gx = overlap_x + i as f32 * (120.0 + GAP);
        scene.push_quad(
            Quad::new(
                Rect::new(gx, top, 72.0, ITEM.height),
                ColorRole::Accent.resolve(theme).with_alpha(a),
            )
            .with_corner_radius(CornerRadius::Row.value()),
        );
        scene.push_quad(
            Quad::new(
                Rect::new(gx + 36.0, top + 8.0, 72.0, ITEM.height - 16.0),
                ColorRole::StatusFailed.resolve(theme).with_alpha(a),
            )
            .with_corner_radius(CornerRadius::Row.value()),
        );
    }
    y += SECTION_HEADER_H + ITEM.height + SECTION_GAP;

    // 3. 状态矩阵，一行。
    draw_section_header(scene, theme, x, y);
    let matrix = StateMatrix::new(Point::new(x, y + SECTION_HEADER_H), 1);
    matrix.draw(scene, theme, |scene, _row, cell, state| {
        button_sample(scene, cell, state, theme);
    });
    y += SECTION_HEADER_H + matrix.size().height + SECTION_GAP;

    // 4. 并排参照槽：live 窗格里画同一个样本（rest）。
    draw_section_header(scene, theme, x, y);
    let slot = ReferenceSlot::new(Point::new(x, y + SECTION_HEADER_H), true);
    slot.draw(scene, theme, |scene, pane| {
        button_sample(scene, pane, ComponentState::Rest, theme);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_quads(theme: Theme) -> usize {
        let mut s = Scene::new();
        build(theme, crate::gallery::layout::PAGE_CONTENT, &mut s);
        s.len()
    }

    #[test]
    fn page_builds_and_fits_in_content() {
        use crate::gallery::layout::PAGE_CONTENT;
        let mut s = Scene::new();
        build(Theme::LIGHT, PAGE_CONTENT, &mut s);
        assert!(s.len() > 40);
        for p in s.primitives() {
            let tekes_ui_scene::Primitive::Quad(q) = p else {
                continue;
            };
            assert!(q.rect.max_x() <= PAGE_CONTENT.max_x() + 0.01, "{q:?}");
            assert!(q.rect.max_y() <= PAGE_CONTENT.max_y() + 0.01, "{q:?}");
            assert!(q.rect.min_x() >= PAGE_CONTENT.min_x() - 0.01, "{q:?}");
        }
    }

    #[test]
    fn quad_count_does_not_depend_on_theme() {
        let n = count_quads(Theme::LIGHT);
        for t in Theme::ALL {
            assert_eq!(count_quads(t), n);
        }
    }

    #[test]
    fn disabled_sample_is_45_percent_of_rest() {
        let cell = Rect::new(0.0, 0.0, 120.0, 56.0);
        let quads = |state| {
            let mut s = Scene::new();
            button_sample(&mut s, cell, state, Theme::LIGHT);
            s.primitives()
                .iter()
                .map(|p| match p {
                    tekes_ui_scene::Primitive::Quad(q) => *q,
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>()
        };
        let rest = quads(ComponentState::Rest);
        let dis = quads(ComponentState::Disabled);
        assert_eq!(dis[1].fill.a, rest[1].fill.a * 0.45);
        assert_eq!(dis[0].border_color.a, rest[0].border_color.a * 0.45);
        assert_eq!(dis[0].border_width, Stroke::Hairline.value());
    }

    #[test]
    fn focus_sample_has_one_point_focused_border_and_no_fill() {
        let cell = Rect::new(0.0, 0.0, 120.0, 56.0);
        let mut s = Scene::new();
        button_sample(&mut s, cell, ComponentState::Focus, Theme::DARK);
        let tekes_ui_scene::Primitive::Quad(q) = s.primitives()[0].clone() else {
            unreachable!()
        };
        assert_eq!(q.border_width, 1.0);
        assert_eq!(
            q.border_color,
            ColorRole::BorderFocused.resolve(Theme::DARK)
        );
        assert_eq!(q.fill.a, 0.0);
    }

    #[test]
    fn hover_and_pressed_fills() {
        let cell = Rect::new(0.0, 0.0, 120.0, 56.0);
        let fill = |state| {
            let mut s = Scene::new();
            button_sample(&mut s, cell, state, Theme::LIGHT);
            let tekes_ui_scene::Primitive::Quad(q) = s.primitives()[0].clone() else {
                unreachable!()
            };
            q.fill
        };
        assert_eq!(
            fill(ComponentState::Hover),
            ColorRole::ButtonHover.resolve(Theme::LIGHT)
        );
        assert_eq!(fill(ComponentState::Pressed).a, 0.14);
        assert_eq!(fill(ComponentState::Rest).a, 0.0);
    }
}
