//! 固定的外壳：目录列、页面标题槽、内容框的右下边线。

use tekes_ui_scene::{Quad, Rect, Scene};
use tekes_ui_tokens::{ColorRole, TextRole, Theme, proposal};

use super::layout::{CATALOG_W, FRAME_H, FRAME_W, PAGE_TITLE_SLOT};
use super::pages::Page;
use super::text::{LabelStyle, TextEngine};

/// 目录行里的页面名：`Body`（13pt），主文字色。
pub const CATALOG_STYLE: LabelStyle = LabelStyle::new(TextRole::Body, ColorRole::PrimaryText);
/// 页面标题槽里的页面名：`BodyEmphasized`，主文字色。
pub const TITLE_STYLE: LabelStyle =
    LabelStyle::new(TextRole::BodyEmphasized, ColorRole::PrimaryText);

pub fn draw(
    scene: &mut Scene,
    theme: Theme,
    text: &mut TextEngine,
    page: Page,
    hover: Option<usize>,
) {
    let mut navigation = super::navigation::Navigation::default();
    draw_with_navigation(scene, theme, text, page, hover, &mut navigation);
}
pub fn draw_with_navigation(
    scene: &mut Scene,
    theme: Theme,
    text: &mut TextEngine,
    page: Page,
    hover: Option<usize>,
    navigation: &mut super::navigation::Navigation,
) {
    draw_with_navigation_focus(scene, theme, text, page, hover, None, navigation);
}

pub(super) fn draw_with_navigation_focus(
    scene: &mut Scene,
    theme: Theme,
    text: &mut TextEngine,
    page: Page,
    hover: Option<usize>,
    focus: Option<Page>,
    navigation: &mut super::navigation::Navigation,
) {
    draw_navigation(scene, theme, text, page, (hover, focus), navigation);
}

fn draw_navigation(
    scene: &mut Scene,
    theme: Theme,
    text: &mut TextEngine,
    page: Page,
    _interaction: (Option<usize>, Option<Page>),
    navigation: &mut super::navigation::Navigation,
) {
    navigation.set_selected(page);
    let height = FRAME_H.max(navigation.bounds().size.height);
    scene.push_quad(Quad::new(
        Rect::new(0.0, 0.0, CATALOG_W, height),
        proposal::color("surface-sidebar", theme).unwrap(),
    ));
    scene.push_quad(Quad::new(
        Rect::new(CATALOG_W - 1.0, 0.0, 1.0, height),
        ColorRole::Separator.resolve(theme),
    ));
    navigation.render(scene, text, theme);
    let clip = navigation
        .geometry()
        .visible_viewport_rect()
        .unwrap_or_default();
    scene.push_clip(clip);
    for (p, row) in navigation.row_frames() {
        if matches!(p,Page::Workbench(i) if super::workbench::entry(i).planned()) {
            text.label_in(
                scene,
                theme,
                Rect::new(row.max_x() - 25.0, row.min_y(), 20.0, row.size.height),
                "…",
                CATALOG_STYLE,
            );
        }
    }
    scene.pop_clip();
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
    fn shared_navigation_host_paints_only_rows_in_its_visible_viewport() {
        for theme in Theme::ALL {
            for scale in [1., 2.] {
                for offset in [0., 317.25, 1800.5] {
                    let mut navigation = super::super::navigation::Navigation::default();
                    navigation.resize(360.);
                    navigation.scroll_by(tekes_ui_scene::Point::new(0., offset));
                    let mut text = TextEngine::new();
                    text.set_scale(scale);
                    text.begin_frame();
                    let page = super::super::navigation::pages()[4];
                    navigation.set_selected(page);
                    let mut scene = Scene::new();
                    draw_navigation(
                        &mut scene,
                        theme,
                        &mut text,
                        page,
                        (Some(page.index()), None),
                        &mut navigation,
                    );
                    let clip = navigation.geometry().visible_viewport_rect().unwrap();
                    let rows = navigation.row_frames();
                    assert!(!rows.is_empty(), "{theme:?}/{scale}x/offset {offset}");
                    assert!(
                        rows.iter()
                            .all(|(_, frame)| clip.intersection(frame).is_some())
                    );
                    if navigation.offset().y > 0. {
                        assert!(rows.len() < super::super::navigation::pages().len());
                    }
                    assert!(!scene.primitives().is_empty());
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
}
