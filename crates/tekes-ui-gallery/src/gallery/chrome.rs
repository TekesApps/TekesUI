//! 固定的外壳：目录列、页面标题槽、内容框的右下边线。

use tekes_ui_scene::{Quad, Rect, Scene};
use tekes_ui_tokens::{ColorRole, CornerRadius, Rgba, TextRole, Theme, proposal};

use super::layout::{CATALOG_W, FRAME_H, FRAME_W, PAGE_TITLE_SLOT, catalog_label_slot};
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
    let height = FRAME_H.max(navigation.bounds().size.height);
    scene.push_quad(Quad::new(
        Rect::new(0.0, 0.0, CATALOG_W, height),
        proposal::color("surface-sidebar", theme).unwrap(),
    ));
    scene.push_quad(Quad::new(
        Rect::new(CATALOG_W - 1.0, 0.0, 1.0, height),
        ColorRole::Separator.resolve(theme),
    ));
    let offset = navigation.scroll.visual_offset().y;
    scene.push_clip(navigation.bounds());
    for section in super::navigation::sections() {
        text.proposal_label_in(
            scene,
            theme,
            section.rect.offset(0.0, -offset),
            &section.title,
            proposal::TextRole::UiCaption,
            ColorRole::SecondaryText.resolve(theme),
        );
        for (p, row) in &section.items {
            let row = row.offset(0.0, -offset);
            if let Some(fill) = row_fill(theme, *p == page, hover == Some(p.index())) {
                scene.push_quad(Quad::new(row, fill).with_corner_radius(CornerRadius::Row.value()));
            }
            let label = catalog_label_slot(p.index()).offset(0.0, -offset);
            text.label_in(scene, theme, label, p.name(), CATALOG_STYLE);
        }
    }
    scene.pop_clip();
    if navigation.scroll.limit().y > 0.0 {
        let thumb = navigation
            .scroll
            .thumb(true, navigation.track(), super::layout::ROW_H);
        scene.push_quad(
            Quad::new(thumb, ColorRole::SecondaryText.resolve(theme))
                .with_corner_radius(tekes_ui_tokens::Spacing::Micro.value()),
        );
    }
    text.label_in(scene, theme, PAGE_TITLE_SLOT, page.name(), TITLE_STYLE);
    let sep = ColorRole::Separator.resolve(theme);
    scene.push_quad(Quad::new(Rect::new(FRAME_W - 1.0, 0.0, 1.0, FRAME_H), sep));
    scene.push_quad(Quad::new(Rect::new(0.0, FRAME_H - 1.0, FRAME_W, 1.0), sep));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::layout::ROW_LABEL_SLOT;

    #[test]
    fn page_names_fit_their_slots() {
        let mut text = TextEngine::new();
        for p in Page::ALL {
            let w = text.label(p.name(), CATALOG_STYLE.role, None).width as f32;
            assert!(
                w <= catalog_label_slot(p.index()).size.width,
                "{}：{w}",
                p.name()
            );
            assert!(w <= ROW_LABEL_SLOT.width);
            let t = text.label(p.name(), TITLE_STYLE.role, None).width as f32;
            assert!(t <= PAGE_TITLE_SLOT.size.width, "{}：{t}", p.name());
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
