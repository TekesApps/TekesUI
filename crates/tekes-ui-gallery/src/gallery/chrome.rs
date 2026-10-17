//! 固定的外壳：目录列、页面标题槽、内容框的右下边线。

use tekes_ui_scene::{Quad, Rect, Scene};
use tekes_ui_tokens::{ColorRole, CornerRadius, Rgba, TextRole, Theme, proposal};

use super::layout::{CATALOG_W, FRAME_H, FRAME_W, PAGE_TITLE_SLOT};
use super::pages::Page;
use super::text::{LabelStyle, TextEngine};

/// 目录行里的页面名：`Body`（13pt），主文字色。
pub const CATALOG_STYLE: LabelStyle = LabelStyle::new(TextRole::Body, ColorRole::PrimaryText);
/// 页面标题槽里的页面名：`BodyEmphasized`，主文字色。
pub const TITLE_STYLE: LabelStyle =
    LabelStyle::new(TextRole::BodyEmphasized, ColorRole::PrimaryText);

/// 目录里某一行的底色：选中用 `RowSelection`，没选中但 hover 用 `ButtonHover`，否则没有底。
/// 选中行 hover 使用提案的独立状态色，保留持久选中语义。
pub fn row_fill(theme: Theme, selected: bool, hovered: bool) -> Option<Rgba> {
    if selected && hovered {
        Some(proposal::color("row-selection-hover", theme).expect("validated proposal role"))
    } else if selected {
        Some(ColorRole::RowSelection.resolve(theme))
    } else if hovered {
        Some(ColorRole::ButtonHover.resolve(theme))
    } else {
        None
    }
}

pub fn draw(
    scene: &mut Scene,
    theme: Theme,
    text: &mut TextEngine,
    page: Page,
    hover: Option<usize>,
) {
    draw_with_navigation(
        scene,
        theme,
        text,
        page,
        hover,
        &super::navigation::Navigation::default(),
    );
}
pub fn draw_with_navigation(
    scene: &mut Scene,
    theme: Theme,
    text: &mut TextEngine,
    page: Page,
    hover: Option<usize>,
    navigation: &super::navigation::Navigation,
) {
    draw_navigation(scene, theme, text, page, hover, navigation, true);
}

fn draw_navigation(
    scene: &mut Scene,
    theme: Theme,
    text: &mut TextEngine,
    page: Page,
    hover: Option<usize>,
    navigation: &super::navigation::Navigation,
    cull: bool,
) {
    let height = FRAME_H.max(navigation.bounds().size.height);
    scene.push_quad(Quad::new(
        Rect::new(0.0, 0.0, CATALOG_W, height),
        proposal::color("surface-sidebar", theme).unwrap(),
    ));
    scene.push_quad(Quad::new(
        Rect::new(CATALOG_W - 1.0, 0.0, 1.0, height),
        ColorRole::Separator.resolve(theme),
    ));
    let geometry = navigation.geometry();
    let origin = geometry.content_origin();
    let visible_document = geometry.visible_document_rect();
    scene.push_clip(geometry.visible_viewport_rect().unwrap_or_default());
    for section in navigation.sections() {
        let header = section.rect.offset(origin.x, origin.y);
        let header_visible =
            visible_document.is_some_and(|visible| section.rect.intersection(&visible).is_some());
        if !cull || header_visible {
            text.proposal_label_in(
                scene,
                theme,
                header,
                &section.title,
                proposal::TextRole::UiCaption,
                ColorRole::SecondaryText.resolve(theme),
            );
        }
        for (p, row) in &section.items {
            let row_document = *row;
            let row = row.offset(origin.x, origin.y);
            if cull
                && visible_document
                    .is_none_or(|visible| row_document.intersection(&visible).is_none())
            {
                continue;
            }
            if let Some(fill) = row_fill(theme, *p == page, hover == Some(p.index())) {
                scene.push_quad(Quad::new(row, fill).with_corner_radius(CornerRadius::Row.value()));
            }
            let label = tekes_ui_scene::Rect::new(
                row.min_x() + super::layout::ROW_PAD_X,
                row.min_y(),
                row.size.width - 2.0 * super::layout::ROW_PAD_X,
                row.size.height,
            );
            text.label_in(scene, theme, label, p.name(), CATALOG_STYLE);
            if matches!(p,Page::Workbench(i) if super::workbench::entry(*i).planned()) {
                text.label_in(
                    scene,
                    theme,
                    Rect::new(row.max_x() - 25.0, row.min_y(), 20.0, row.size.height),
                    "…",
                    CATALOG_STYLE,
                );
            }
        }
    }
    scene.pop_clip();
    if navigation.scroll.scrollbar_visible(true) {
        let thumb = navigation
            .scroll
            .thumb(true, navigation.track(), super::layout::ROW_H);
        scene.push_quad(
            Quad::new(thumb, ColorRole::SecondaryText.resolve(theme))
                .with_corner_radius(tekes_ui_tokens::Spacing::Micro.value()),
        );
    }
    text.label_in(
        scene,
        theme,
        Rect::new(
            PAGE_TITLE_SLOT.min_x(),
            PAGE_TITLE_SLOT.min_y(),
            600.0,
            PAGE_TITLE_SLOT.size.height,
        ),
        page.name(),
        TITLE_STYLE,
    );
    let sep = ColorRole::Separator.resolve(theme);
    scene.push_quad(Quad::new(Rect::new(FRAME_W - 1.0, 0.0, 1.0, FRAME_H), sep));
    scene.push_quad(Quad::new(Rect::new(0.0, FRAME_H - 1.0, FRAME_W, 1.0), sep));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_culling_preserves_pixels_at_clip_edges_and_reduces_glyph_work() {
        let mut renderer = tekes_ui_render::Renderer::new_headless().unwrap();
        for theme in Theme::ALL {
            for scale in [1., 2.] {
                for offset in [0., 317.25, 1800.5] {
                    let mut navigation = super::super::navigation::Navigation::default();
                    navigation.resize(360.);
                    navigation.scroll.scroll_by(0., offset);
                    let mut text = TextEngine::new();
                    text.set_scale(scale);
                    text.begin_frame();
                    let page = super::super::navigation::pages()[4];
                    let mut reference = Scene::new();
                    draw_navigation(
                        &mut reference,
                        theme,
                        &mut text,
                        page,
                        Some(page.index()),
                        &navigation,
                        false,
                    );
                    let size = (1440 * scale as u32, 360 * scale as u32);
                    let before = renderer
                        .render_to_rgba8(
                            &reference,
                            text.atlas_mut(),
                            size,
                            scale,
                            ColorRole::DocumentCanvas.resolve(theme),
                        )
                        .unwrap();
                    let mut culled = Scene::new();
                    draw_navigation(
                        &mut culled,
                        theme,
                        &mut text,
                        page,
                        Some(page.index()),
                        &navigation,
                        true,
                    );
                    let after = renderer
                        .render_to_rgba8(
                            &culled,
                            text.atlas_mut(),
                            size,
                            scale,
                            ColorRole::DocumentCanvas.resolve(theme),
                        )
                        .unwrap();
                    assert_eq!(before, after, "{theme:?}/{scale}x/offset {offset}");
                    let glyphs = |scene: &Scene| {
                        scene
                            .primitives()
                            .iter()
                            .map(|p| match p {
                                tekes_ui_scene::Primitive::Glyphs(run) => run.glyphs.len(),
                                _ => 0,
                            })
                            .sum::<usize>()
                    };
                    assert!(glyphs(&culled) * 2 < glyphs(&reference));
                }
            }
        }
    }

    #[test]
    fn page_names_fit_their_slots() {
        let mut text = TextEngine::new();
        for p in Page::ALL {
            let w = text.label(p.name(), CATALOG_STYLE.role, None).width as f32;
            assert!(
                w <= CATALOG_W - 2.0 * super::super::layout::ROW_PAD_X - 16.0,
                "{}: {w}",
                p.name()
            );
            let t = text.label(p.name(), TITLE_STYLE.role, None).width as f32;
            assert!(t <= 600.0, "{}：{t}", p.name());
        }
    }

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
        assert_eq!(
            row_fill(t, true, true),
            proposal::color("row-selection-hover", t)
        );
        assert_ne!(row_fill(t, true, true), row_fill(t, true, false));
    }
}
