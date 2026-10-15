//! The native Gallery icon catalogue, reusing the component renderer.
use super::{
    PageCtx,
    showcase::{Icon, draw_icon},
};
use crate::gallery::icon_catalog::{IconCatalog, preview_sizes};
use tekes_ui_scene::{ImageQuad, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, CornerRadius, Spacing, Stroke, proposal};

pub fn search_rect(content: Rect) -> Rect {
    Rect::new(content.min_x(), content.min_y(), content.size.width, 40.0)
}

fn matches(name: &str, query: &str) -> bool {
    name.to_lowercase().contains(&query.to_lowercase())
}

#[derive(Clone, Copy)]
enum CatalogIcon {
    Native(Icon),
    Legacy(usize),
}
fn filtered<'a>(catalog: &'a IconCatalog, query: &str) -> Vec<(CatalogIcon, &'a str)> {
    Icon::ALL
        .into_iter()
        .map(|(icon, name)| (CatalogIcon::Native(icon), name))
        .chain(
            catalog
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| (CatalogIcon::Legacy(index), entry.name.as_str())),
        )
        .filter(|(_, name)| matches(name, query))
        .collect()
}
const GRID_TOP: f32 = 112.0;
const ROW_STEP: f32 = 136.0;
fn tile_rect(content: Rect, index: usize) -> Rect {
    let gap = Spacing::Loose.value();
    let width = (content.size.width - gap * 2.0) / 3.0;
    Rect::new(
        content.min_x() + (index % 3) as f32 * (width + gap),
        content.min_y() + GRID_TOP + (index / 3) as f32 * ROW_STEP,
        width,
        120.0,
    )
}
pub fn extent(catalog: &IconCatalog, query: &str) -> Size {
    Size::new(
        super::super::layout::PAGE_CONTENT.size.width,
        (GRID_TOP + filtered(catalog, query).len().div_ceil(3) as f32 * ROW_STEP)
            .max(super::super::layout::PAGE_CONTENT.size.height),
    )
}
pub fn visible_legacy(
    catalog: &IconCatalog,
    query: &str,
    content: Rect,
    bounds: Rect,
) -> Vec<usize> {
    filtered(catalog, query)
        .into_iter()
        .enumerate()
        .filter_map(|(i, (icon, _))| {
            tile_rect(content, i).intersection(&bounds)?;
            match icon {
                CatalogIcon::Legacy(index) => Some(index),
                _ => None,
            }
        })
        .collect()
}
pub fn build(ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene, query: &str) {
    build_catalog(ctx, content, scene, query, &IconCatalog::default(), content);
}
pub fn build_catalog(
    ctx: &mut PageCtx<'_>,
    content: Rect,
    scene: &mut Scene,
    query: &str,
    catalog: &IconCatalog,
    bounds: Rect,
) {
    let found = filtered(catalog, query);
    let ink = ColorRole::PrimaryText.resolve(ctx.theme);
    let muted = ColorRole::SecondaryText.resolve(ctx.theme);
    let label = format!(
        "{} / {} icons · 名称包含匹配 · 12 / 16 / 24 pt",
        found.len(),
        Icon::ALL.len() + catalog.entries.len()
    );
    ctx.text.proposal_label_in(
        scene,
        ctx.theme,
        Rect::new(
            content.min_x(),
            content.min_y() + 48.0,
            content.size.width,
            24.0,
        ),
        &label,
        proposal::TextRole::UiCaption,
        muted,
    );
    let source_note = if catalog.source.is_some() {
        format!(
            "基础图标 10 · Tekes 图标 {} · 作废原稿不展示{}",
            catalog.entries.len(),
            if catalog.notices.is_empty() {
                String::new()
            } else {
                format!(" · {} 项加载异常", catalog.notices.len())
            }
        )
    } else {
        "完整图标集未加载：请设置 TEKESUI_ICON_DIR".into()
    };
    ctx.text.proposal_label_in(
        scene,
        ctx.theme,
        Rect::new(
            content.min_x(),
            content.min_y() + 76.0,
            content.size.width,
            24.0,
        ),
        &source_note,
        proposal::TextRole::UiCaption,
        muted,
    );
    if found.is_empty() {
        ctx.text.proposal_label_in(
            scene,
            ctx.theme,
            Rect::new(
                content.min_x(),
                content.min_y() + GRID_TOP,
                content.size.width,
                32.0,
            ),
            "没有匹配的图标",
            proposal::TextRole::UiBody,
            muted,
        );
    }
    let gap = Spacing::Loose.value();
    for (i, (icon, name)) in found.into_iter().enumerate() {
        let tile = tile_rect(content, i);
        if tile.intersection(&bounds).is_none() {
            continue;
        }
        let width = tile.size.width;
        scene.push_quad(
            Quad::new(tile, ColorRole::DocumentCanvas.resolve(ctx.theme))
                .with_corner_radius(CornerRadius::Panel.value())
                .with_border(
                    Stroke::Hairline.value(),
                    ColorRole::Border.resolve(ctx.theme),
                ),
        );
        ctx.text.proposal_label_in(
            scene,
            ctx.theme,
            Rect::new(
                tile.min_x() + gap,
                tile.min_y() + gap,
                width - gap * 2.0,
                24.0,
            ),
            name,
            proposal::TextRole::UiBody,
            ink,
        );
        for (column, size) in preview_sizes().into_iter().enumerate() {
            let center = tile.min_x() + width * (column as f32 + 0.5) / 3.0;
            let rect = Rect::new(
                center - size / 2.0,
                tile.min_y() + 58.0 - size / 2.0,
                size,
                size,
            );
            match icon {
                CatalogIcon::Native(icon) => draw_icon(ctx, scene, rect, icon),
                CatalogIcon::Legacy(index) => {
                    if let Some(image) = catalog.entries[index].images[column] {
                        scene.push_image(ImageQuad {
                            rect,
                            image,
                            opacity: 1.0,
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn name_filter_is_case_insensitive_literal_substring() {
        assert!(matches("chevron-right", "vron"));
        assert!(matches("chevron-down", "DOWN"));
        assert!(matches("search", ""));
        assert!(!matches("search", "xyz"));
        assert!(!matches("search", "%"));
        assert!(!matches("search", "_"));
        assert_eq!(
            Icon::ALL
                .iter()
                .filter(|(_, n)| matches(n, "chevron"))
                .count(),
            2
        );
    }
}

#[cfg(test)]
mod catalogue_review {
    use super::*;
    use crate::gallery::{layout, static_text::StaticText, text::TextEngine};
    use tekes_ui_render::Renderer;
    use tekes_ui_tokens::Theme;

    #[test]
    fn combined_search_and_the_last_row_use_the_same_filtered_geometry() {
        let catalog = IconCatalog::fixture(&["tekes-search", "tekes-web-search", "tekes-write"]);
        assert_eq!(filtered(&catalog, "").len(), 13);
        assert_eq!(filtered(&catalog, "SeArCh").len(), 3);
        assert_eq!(filtered(&catalog, "eb-sea").len(), 1);
        assert!(filtered(&catalog, "not-an-icon").is_empty());
        let content = layout::PAGE_CONTENT;
        let last = tile_rect(content, 12);
        assert!(last.max_y() <= content.min_y() + extent(&catalog, "").height);
        let last_bounds = Rect::new(content.min_x(), last.min_y(), content.size.width, 120.0);
        assert_eq!(visible_legacy(&catalog, "", content, last_bounds), vec![2]);
        let first_bounds = Rect::new(
            content.min_x(),
            content.min_y(),
            content.size.width,
            GRID_TOP + 120.0,
        );
        assert!(visible_legacy(&catalog, "", content, first_bounds).is_empty());
        assert_eq!(
            visible_legacy(&catalog, "write", content, first_bounds),
            vec![2]
        );
    }

    /// Explicit local review: never runs with private assets on CI or in fixed snapshots.
    #[test]
    #[ignore = "requires explicit TEKESUI_ICON_DIR and TEKESUI_ICONS_REVIEW_OUT"]
    fn local_full_catalogue_pixels_and_scroll_culling() {
        let source = std::env::var_os("TEKESUI_ICON_DIR").expect("explicit local icon source");
        let out = std::path::PathBuf::from(
            std::env::var_os("TEKESUI_ICONS_REVIEW_OUT").expect("private evidence directory"),
        );
        std::fs::create_dir_all(&out).unwrap();
        let mut catalog = IconCatalog::load(std::path::Path::new(&source));
        assert_eq!(catalog.entries.len(), 133);
        assert!(catalog.notices.is_empty(), "{:?}", catalog.notices);
        assert_eq!(filtered(&catalog, "").len(), 143);
        assert_eq!(filtered(&catalog, "SeArCh").len(), 3);
        let mut renderer = Renderer::new_headless().unwrap();
        let bounds = layout::PAGE_CONTENT;
        for theme in Theme::ALL {
            for scale in [1.0, 2.0] {
                // Traverse every row: all 133 SVGs must actually produce raster handles.
                for row in 0..143_usize.div_ceil(3) {
                    let content = bounds.offset(0.0, -(row as f32 * ROW_STEP));
                    let visible = visible_legacy(&catalog, "", content, bounds);
                    catalog.prepare(&mut renderer, theme, scale, &visible);
                }
                assert_eq!(renderer.image_count(), 133 * 3);
                assert!(
                    catalog
                        .entries
                        .iter()
                        .all(|e| e.images.iter().all(Option::is_some))
                );
                for (state, query, offset) in [
                    ("all-top", "", 0.0),
                    (
                        "all-bottom",
                        "",
                        extent(&catalog, "").height - bounds.size.height,
                    ),
                    ("search", "SeArCh", 0.0),
                    ("empty", "no-such-icon", 0.0),
                ] {
                    let content = bounds.offset(0.0, -offset);
                    let mut text = TextEngine::new();
                    text.set_scale(scale);
                    text.begin_frame();
                    let mut st = StaticText::new(None);
                    let mut ctx = PageCtx {
                        theme,
                        text: &mut text,
                        static_text: &mut st,
                    };
                    let mut scene = Scene::new();
                    scene.push_clip(bounds);
                    // Reuse the actual search surface; only its input state is scripted.
                    let mut search = crate::gallery::text_demo::TextDemo::search_fixture();
                    if !query.is_empty() {
                        search.command("insertTab:");
                        search.paste(query);
                        search.blur();
                    }
                    search.build_search(&mut ctx, search_rect(content), &mut scene);
                    build_catalog(&mut ctx, content, &mut scene, query, &catalog, bounds);
                    scene.pop_clip();
                    let width = (layout::FRAME_W * scale) as u32;
                    let height = (layout::FRAME_H * scale) as u32;
                    let rgba = renderer
                        .render_to_rgba8(
                            &scene,
                            text.atlas_mut(),
                            (width, height),
                            scale,
                            ColorRole::DocumentCanvas.resolve(theme),
                        )
                        .unwrap();
                    let path = out.join(format!(
                        "icons-{state}-{}@{scale}x.png",
                        crate::gallery::theme::theme_slug(theme)
                    ));
                    let mut encoder =
                        png::Encoder::new(std::fs::File::create(path).unwrap(), width, height);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .unwrap()
                        .write_image_data(&rgba)
                        .unwrap();
                }
            }
        }
    }
}
