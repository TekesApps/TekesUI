//! Primitives 页：共享基础样本和校准夹具。

use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Alpha, ColorRole, CornerRadius, Rgba, Spacing, Stroke, Theme, proposal};

use crate::gallery::containers::ComponentState;

const ITEM: Size = Size::new(96.0, 56.0);
const GAP: f32 = Spacing::Relaxed.value();

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
        ComponentState::Pressed => {
            proposal::color("button-pressed", theme).expect("validated proposal pressed role")
        }
        _ => Rgba::new(0.0, 0.0, 0.0, 0.0),
    };
    let (border_w, border) = if state == ComponentState::Focus {
        (
            Stroke::Rule.value(),
            ColorRole::BorderFocused.resolve(theme),
        )
    } else {
        (Stroke::Rule.value(), ColorRole::Border.resolve(theme))
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

/// Shared visual vocabulary. Foundations owns the rules; calibration reuses the
/// same samples. The original build remains a historical golden fixture only.
pub fn stroke_samples(scene: &mut Scene, theme: Theme, origin: Point) {
    let (x, top) = (origin.x, origin.y);
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
}
pub fn alpha_samples(scene: &mut Scene, theme: Theme, origin: Point) {
    let (overlap_x, top) = (origin.x, origin.y);
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
}
pub(crate) fn build_calibration_with_labels(
    theme: Theme,
    backing_scale: f32,
    content: Rect,
    scene: &mut Scene,
    mut label: impl FnMut(&mut Scene, Rect, &str),
) {
    let x = content.min_x();
    let y = content.min_y();
    for (row, offset) in [0.0, 0.5].into_iter().enumerate() {
        let top = y + row as f32 * 160.;
        label(
            scene,
            Rect::new(x, top, content.size.width, 32.),
            &format!("Raster alignment: origin + {offset} pt · backing scale {backing_scale}x"),
        );
        stroke_samples(scene, theme, Point::new(x + offset, top + 40. + offset));
    }
    label(
        scene,
        Rect::new(x, y + 340., content.size.width, 32.),
        "Live alpha compositing — inspect intersections for seams and clipping",
    );
    alpha_samples(scene, theme, Point::new(x, y + 384.));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lab_has_live_raster_checks_without_empty_reference_panes() {
        for theme in Theme::ALL {
            let mut scene = Scene::new();
            let mut labels = Vec::new();
            build_calibration_with_labels(
                theme,
                2.0,
                Rect::new(0., 0., 1104., 700.),
                &mut scene,
                |_, slot, value| labels.push((slot, value.to_owned())),
            );
            let quads: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    tekes_ui_scene::Primitive::Quad(q) => Some(q),
                    _ => None,
                })
                .collect();
            // Six alignment strokes and six intersecting color samples. No
            // reference/difference placeholder containers are constructed.
            assert_eq!(quads.len(), 12);
            assert_eq!(labels.len(), 3);
            assert!(quads.iter().all(|q| q.rect.max_y() <= 440.));
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
        assert_eq!(dis[0].border_width, Stroke::Rule.value());
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
        assert_eq!(
            fill(ComponentState::Pressed),
            proposal::color("button-pressed", Theme::LIGHT).unwrap()
        );
        assert_eq!(fill(ComponentState::Rest).a, 0.0);
    }
}
