//! The native Gallery icon catalogue, reusing the component renderer.
use super::{
    PageCtx,
    showcase::{Icon, draw_icon_tinted},
};
use crate::gallery::icon_catalog::{IconCatalog, preview_sizes};
use tekes_ui_controls::layout_view::{IconRenderer, IconView};
use tekes_ui_controls::paint;
use tekes_ui_scene::{ImageId, ImageQuad, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, CornerRadius, Spacing, proposal};

pub fn search_rect(content: Rect) -> Rect {
    Rect::new(content.min_x(), content.min_y(), content.size.width, 40.0)
}

fn matches(name: &str, query: &str) -> bool {
    // The common scrolling path has no query. Avoid allocating lowercase
    // strings for every catalogue entry on every frame in that case.
    query.is_empty() || name.to_lowercase().contains(&query.to_lowercase())
}

#[derive(Clone, Copy)]
enum CatalogIcon {
    Native(Icon),
    Legacy(usize),
}

/// Keeps the icon page's existing artwork backends while delegating fitted
/// geometry and clipping to the production IconView. Legacy catalogue images
/// already live in the bounded GPU cache and are intentionally not rerasterized.
#[derive(Clone, Copy)]
enum GalleryIconArtwork {
    Native,
    Legacy(Option<ImageId>),
}

pub(crate) struct GalleryIconRenderer<'ctx, 'frame> {
    ctx: &'ctx mut PageCtx<'frame>,
    artwork: GalleryIconArtwork,
}

impl<'ctx, 'frame> GalleryIconRenderer<'ctx, 'frame> {
    pub(crate) fn native(ctx: &'ctx mut PageCtx<'frame>) -> Self {
        Self {
            ctx,
            artwork: GalleryIconArtwork::Native,
        }
    }
}

impl IconRenderer for GalleryIconRenderer<'_, '_> {
    fn draw_icon(
        &mut self,
        scene: &mut Scene,
        canonical_name: &str,
        frame: Rect,
        tint: tekes_ui_tokens::Rgba,
    ) {
        match self.artwork {
            GalleryIconArtwork::Legacy(Some(image)) => scene.push_image(ImageQuad {
                rect: frame,
                image,
                opacity: 1.0,
            }),
            GalleryIconArtwork::Legacy(None) => {}
            GalleryIconArtwork::Native => {
                // The SDK uses canonical SVG names. This native Gallery
                // backend keeps its existing chevron artwork for those inputs.
                let icon = match canonical_name {
                    "tekes-forward" => Some(Icon::Chevron),
                    "tekes-expand" => Some(Icon::Down),
                    _ => Icon::ALL
                        .iter()
                        .find(|(_, name)| *name == canonical_name)
                        .map(|(icon, _)| *icon),
                };
                if let Some(icon) = icon {
                    draw_icon_tinted(self.ctx, scene, frame, icon, tint);
                }
            }
        }
    }
}

fn render_gallery_icon(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    canonical_name: &str,
    frame: Rect,
    size: f32,
    clip: Rect,
    artwork: GalleryIconArtwork,
) {
    let palette = tekes_ui_tokens::Palette::from(ctx.theme);
    let mut renderer = GalleryIconRenderer { ctx, artwork };
    IconView::default().render(
        scene,
        &mut renderer,
        canonical_name,
        frame,
        size,
        Some(clip),
        palette,
    );
}

fn native_name(icon: Icon) -> &'static str {
    match icon {
        Icon::Check => "check",
        Icon::Chevron => "chevron-right",
        Icon::Down => "chevron-down",
        Icon::Close => "close",
        Icon::Plus => "plus",
        Icon::Search => "search",
        Icon::Folder => "folder",
        Icon::File => "file",
        Icon::Info => "info",
        Icon::Spinner => "spinner",
    }
}

fn filtered<'a>(catalog: &'a IconCatalog, query: &str) -> Vec<(CatalogIcon, &'a str)> {
    let mut found: Vec<_> = Icon::ALL
        .into_iter()
        .map(|(icon, name)| (CatalogIcon::Native(icon), name))
        .chain(
            catalog
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| (CatalogIcon::Legacy(index), entry.name.as_str())),
        )
        .filter(|(icon, name)| {
            matches(name, query)
                || match icon {
                    CatalogIcon::Legacy(i) => matches(&catalog.entries[*i].label, query),
                    _ => false,
                }
        })
        .collect();
    found.sort_by_key(|(icon, _)| match icon {
        CatalogIcon::Legacy(i) if catalog.entries[*i].kind == "refine" => 0,
        CatalogIcon::Legacy(i) if !catalog.entries[*i].kind.is_empty() => 1,
        CatalogIcon::Native(_) => 2,
        _ => 3,
    });
    found
}
const GRID_TOP: f32 = 112.0;
const ROW_STEP: f32 = 164.0;
const TILE_HEIGHT: f32 = 148.0;
fn normalized_type_scale(type_scale: f32) -> f32 {
    if type_scale.is_finite() {
        type_scale.clamp(1.0, 1.5)
    } else {
        1.0
    }
}
fn scaled_tile_height(type_scale: f32) -> f32 {
    TILE_HEIGHT * normalized_type_scale(type_scale)
}
fn scaled_row_step(type_scale: f32) -> f32 {
    if normalized_type_scale(type_scale) == 1.0 {
        ROW_STEP
    } else {
        scaled_tile_height(type_scale) + Spacing::Loose.value()
    }
}
fn columns(width: f32) -> usize {
    let min_tile = 2.0 * (3.0 * preview_sizes()[2] + 4.0 * Spacing::Loose.value());
    if width >= min_tile * 2.0 + Spacing::Loose.value() {
        2
    } else {
        1
    }
}
#[cfg(test)]
fn tile_rect(content: Rect, index: usize) -> Rect {
    tile_rect_at_scale(content, index, 1.0)
}
fn tile_rect_at_scale(content: Rect, index: usize, type_scale: f32) -> Rect {
    let gap = Spacing::Loose.value();
    let columns = columns(content.size.width);
    let width = (content.size.width - gap * (columns - 1) as f32) / columns as f32;
    Rect::new(
        content.min_x() + (index % columns) as f32 * (width + gap),
        content.min_y() + GRID_TOP + (index / columns) as f32 * scaled_row_step(type_scale),
        width,
        scaled_tile_height(type_scale),
    )
}

#[derive(Clone, Copy)]
struct ScaledCardTextLayout {
    title_height: f32,
    caption_top: f32,
    caption_height: f32,
    preview_center_y: f32,
    size_label_top: f32,
    size_label_height: f32,
}

fn scaled_card_text_layout(
    tile: Rect,
    title_height: f32,
    caption_height: f32,
    size_label_height: f32,
) -> ScaledCardTextLayout {
    let inset = Spacing::Loose.value();
    let caption_top = inset + title_height + Spacing::Tight.value();
    let size_label_height = size_label_height.max(Spacing::Relaxed.value());
    let size_label_top = tile.size.height - Spacing::Micro.value() - size_label_height;
    let preview_center_y = size_label_top - Spacing::Tight.value() - preview_sizes()[2] / 2.0;
    ScaledCardTextLayout {
        title_height,
        caption_top,
        caption_height,
        preview_center_y,
        size_label_top,
        size_label_height,
    }
}
pub fn extent(catalog: &IconCatalog, query: &str, width: f32) -> Size {
    extent_at_scale(catalog, query, width, 1.0)
}
pub fn extent_at_scale(catalog: &IconCatalog, query: &str, width: f32, type_scale: f32) -> Size {
    // Computing height needs a count, not a sorted temporary catalogue.
    let count = if query.is_empty() {
        Icon::ALL.len() + catalog.entries.len()
    } else {
        filtered(catalog, query).len()
    };
    Size::new(
        width,
        (GRID_TOP + count.div_ceil(columns(width)) as f32 * scaled_row_step(type_scale))
            .max(super::super::layout::PAGE_CONTENT.size.height),
    )
}
pub fn visible_legacy(
    catalog: &IconCatalog,
    query: &str,
    content: Rect,
    bounds: Rect,
) -> Vec<usize> {
    visible_legacy_at_scale(catalog, query, content, bounds, 1.0)
}
pub fn visible_legacy_at_scale(
    catalog: &IconCatalog,
    query: &str,
    content: Rect,
    bounds: Rect,
    type_scale: f32,
) -> Vec<usize> {
    filtered(catalog, query)
        .into_iter()
        .enumerate()
        .filter_map(|(i, (icon, _))| {
            tile_rect_at_scale(content, i, type_scale).intersection(&bounds)?;
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
    let defaults = catalog
        .entries
        .iter()
        .filter(|e| tekes_ui_icons::Icon::named(&e.name).is_some())
        .count();
    let comparisons = catalog
        .entries
        .iter()
        .filter(|e| e.before.is_some())
        .count();
    let proposals = catalog
        .entries
        .iter()
        .filter(|e| !e.kind.is_empty())
        .count();
    let source_note = format!(
        "图标增补 {proposals} · 已设为默认 {defaults} · 新旧对比 {comparisons} · {}{}",
        if catalog.source.is_some() {
            "原有图标集已加载"
        } else {
            "原有图标集未加载"
        },
        if catalog.notices.is_empty() {
            String::new()
        } else {
            format!(" · {} 项加载异常", catalog.notices.len())
        }
    );
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
    let scaled_type = ctx.text.type_scale() > 1.0;
    for (i, (icon, name)) in found.into_iter().enumerate() {
        let type_scale = ctx.text.type_scale();
        let tile = tile_rect_at_scale(content, i, type_scale);
        if tile.intersection(&bounds).is_none() {
            continue;
        }
        let width = tile.size.width;
        let (border_width, border_ink) = paint::decorative_border(ctx.theme, ctx.text.scale());
        scene.push_quad(
            Quad::new(tile, ColorRole::DocumentCanvas.resolve(ctx.theme))
                .with_corner_radius(CornerRadius::Panel.value())
                .with_border(border_width, border_ink),
        );
        let title_width = width - gap * 2.0;
        let entry = match icon {
            CatalogIcon::Legacy(index) => Some(&catalog.entries[index]),
            _ => None,
        };
        let paired = entry.is_some_and(|e| e.before.is_some());
        let panels = if paired { 2 } else { 1 };
        let captions: Vec<String> = (0..panels)
            .map(|panel| match entry {
                Some(e) if paired => {
                    if panel == 0 {
                        "旧稿".into()
                    } else {
                        format!("默认使用 · {}", e.label)
                    }
                }
                Some(e) if e.kind == "refine" => format!("默认使用 · {} · 旧稿未加载", e.label),
                Some(e) if e.kind == "rebuild" => format!("重绘稿 · {}", e.label),
                Some(e) if e.kind == "new" => format!("新增 · {}", e.label),
                Some(_) => "原有图标".into(),
                None => "基础图标".into(),
            })
            .collect();
        let scaled_layout = if scaled_type {
            let title_height = (ctx
                .text
                .proposal_label(name, proposal::TextRole::UiBody, Some(title_width))
                .height as f32)
                .max(Spacing::Relaxed.value());
            let caption_height = captions
                .iter()
                .map(|caption| {
                    let panel_width = width / panels as f32;
                    ctx.text
                        .proposal_label(
                            caption,
                            proposal::TextRole::UiCaption,
                            Some(panel_width - gap * 2.0),
                        )
                        .height as f32
                })
                .fold(Spacing::Relaxed.value(), f32::max);
            let size_label_height = ctx
                .text
                .proposal_label("24", proposal::TextRole::UiCaption, Some(32.0))
                .height as f32;
            Some(scaled_card_text_layout(
                tile,
                title_height,
                caption_height,
                size_label_height,
            ))
        } else {
            None
        };
        if let Some(layout) = scaled_layout {
            ctx.text.proposal_wrapped_in(
                scene,
                ctx.theme,
                Rect::new(
                    tile.min_x() + gap,
                    tile.min_y() + gap,
                    title_width,
                    layout.title_height,
                ),
                name,
                proposal::TextRole::UiBody,
                ink,
            );
        } else {
            ctx.text.proposal_label_in(
                scene,
                ctx.theme,
                Rect::new(tile.min_x() + gap, tile.min_y() + gap, title_width, 24.0),
                name,
                proposal::TextRole::UiBody,
                ink,
            );
        }
        for (panel, caption) in captions.iter().enumerate().take(panels) {
            let left = tile.min_x() + width * panel as f32 / panels as f32;
            let panel_width = width / panels as f32;
            let variant = entry.map(|e| {
                if paired && panel == 0 {
                    e.before.as_deref().unwrap()
                } else {
                    e
                }
            });
            if let Some(layout) = scaled_layout {
                ctx.text.proposal_wrapped_in(
                    scene,
                    ctx.theme,
                    Rect::new(
                        left + gap,
                        tile.min_y() + layout.caption_top,
                        panel_width - gap * 2.0,
                        layout.caption_height,
                    ),
                    caption,
                    proposal::TextRole::UiCaption,
                    muted,
                );
            } else {
                ctx.text.proposal_label_in(
                    scene,
                    ctx.theme,
                    Rect::new(
                        left + gap,
                        tile.min_y() + 42.0,
                        panel_width - gap * 2.0,
                        20.0,
                    ),
                    caption,
                    proposal::TextRole::UiCaption,
                    muted,
                );
            }
            for (column, size) in preview_sizes().into_iter().enumerate() {
                let center = left + panel_width * (column as f32 + 0.5) / 3.0;
                let preview_center_y = scaled_layout.map_or(tile.min_y() + 92.0, |layout| {
                    tile.min_y() + layout.preview_center_y
                });
                let rect = Rect::new(
                    center - size / 2.0,
                    preview_center_y - size / 2.0,
                    size,
                    size,
                );
                let artwork = match icon {
                    CatalogIcon::Native(_) => GalleryIconArtwork::Native,
                    CatalogIcon::Legacy(_) => {
                        GalleryIconArtwork::Legacy(variant.unwrap().images[column])
                    }
                };
                // IconView owns the artwork's fitted frame and uses the same
                // viewport clip for paint and hit geometry. Preserve the
                // renderer/cache path that produced the approved Gallery art.
                let canonical_name = match icon {
                    CatalogIcon::Native(icon) => Some(native_name(icon)),
                    CatalogIcon::Legacy(index) => Some(catalog.entries[index].name.as_str()),
                };
                if let Some(canonical_name) = canonical_name {
                    render_gallery_icon(ctx, scene, canonical_name, rect, size, bounds, artwork);
                }
                let size_label = format!("{}", size as u32);
                if let Some(layout) = scaled_layout {
                    ctx.text.proposal_wrapped_in(
                        scene,
                        ctx.theme,
                        Rect::new(
                            center - 16.0,
                            tile.min_y() + layout.size_label_top,
                            32.0,
                            layout.size_label_height,
                        ),
                        &size_label,
                        proposal::TextRole::UiCaption,
                        muted,
                    );
                } else {
                    ctx.text.proposal_label_in(
                        scene,
                        ctx.theme,
                        Rect::new(center - 16.0, tile.min_y() + 116.0, 32.0, 20.0),
                        &size_label,
                        proposal::TextRole::UiCaption,
                        muted,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_theme_switch_repaints_the_actual_catalog_native_adapter() {
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};

        let mut text = crate::gallery::text::TextEngine::new();
        text.set_motion_enabled(true);
        text.set_motion_context(std::time::Duration::ZERO, true);
        let mut static_text = crate::gallery::static_text::StaticText::new(None);
        let side = proposal::metric("size", "size-icon-preview").unwrap();
        let frame = Rect::new(20., 30., side, side);
        let mut original_geometry = None;
        let mut previous_ink = None;
        for theme in Theme::ALL {
            text.begin_frame();
            let mut ctx = PageCtx {
                theme,
                text: &mut text,
                static_text: &mut static_text,
            };
            let mut scene = Scene::new();
            // Exact helper used by build_catalog; this path is pure IconView,
            // distinct from the unified Button image/artwork slot.
            render_gallery_icon(
                &mut ctx,
                &mut scene,
                "plus",
                frame,
                side,
                frame,
                GalleryIconArtwork::Native,
            );
            assert!(scene.composition_error().is_none());
            let quads: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Quad(q) => Some(q),
                    _ => None,
                })
                .collect();
            assert!(!quads.is_empty(), "actual native catalog artwork paints");
            let ink = Palette::from(theme).color("primary-text").unwrap();
            assert!(
                quads.iter().all(|q| q.fill == ink),
                "Gallery adapter consumes current Theme tint"
            );
            let artboard = Rect::new(
                frame.center().x - side / 2.,
                frame.center().y - side / 2.,
                side,
                side,
            );
            let stroke =
                tekes_ui_icons::REGULAR_STROKE_WIDTH * side / tekes_ui_icons::ARTBOARD_SIZE;
            assert!(quads.iter().all(|q| {
                q.rect.min_x() >= artboard.min_x()
                    && q.rect.max_x() <= artboard.max_x()
                    && q.rect.min_y() >= artboard.min_y()
                    && q.rect.max_y() <= artboard.max_y()
                    && q.rect.size.width.min(q.rect.size.height) == stroke
            }));
            let geometry: Vec<_> = quads.iter().map(|q| q.rect).collect();
            if let Some(original) = &original_geometry {
                assert_eq!(&geometry, original);
            } else {
                original_geometry = Some(geometry);
            }
            if let Some(previous) = previous_ink
                && theme.is_dark()
            {
                assert_ne!(ink, previous);
            }
            previous_ink = Some(ink);
            let mut hidden = Scene::new();
            render_gallery_icon(
                &mut ctx,
                &mut hidden,
                "plus",
                frame,
                side,
                Rect::new(500., 500., 10., 10.),
                GalleryIconArtwork::Native,
            );
            assert!(hidden.primitives().is_empty());
        }
    }

    #[test]
    fn decorative_icon_cards_follow_renderer_backing_scale() {
        let mut text = crate::gallery::text::TextEngine::new();
        let mut static_text = crate::gallery::static_text::StaticText::new(None);
        let theme = tekes_ui_tokens::Theme::LIGHT;
        for scale in [1.0, 2.0] {
            text.set_scale(scale);
            let mut ctx = PageCtx {
                theme,
                text: &mut text,
                static_text: &mut static_text,
            };
            let mut scene = Scene::new();
            let content = Rect::new(0.0, 0.0, 960.0, 1200.0);
            build_catalog(
                &mut ctx,
                content,
                &mut scene,
                "",
                &IconCatalog::default(),
                content,
            );
            let (expected_width, expected_ink) = paint::decorative_border(theme, scale);
            let card = scene
                .primitives()
                .iter()
                .find_map(|primitive| match primitive {
                    tekes_ui_scene::Primitive::Quad(q)
                        if q.fill == ColorRole::DocumentCanvas.resolve(theme)
                            && q.border_color == expected_ink
                            && q.border_width == expected_width =>
                    {
                        Some(q)
                    }
                    _ => None,
                })
                .expect("decorative icon card boundary");
            assert_eq!(card.border_width, expected_width);
            assert_eq!(card.border_color, expected_ink);
        }
    }

    #[test]
    fn scaled_icon_card_slots_fit_measured_title_caption_and_size_labels() {
        let mut text = crate::gallery::text::TextEngine::new();
        text.set_locale(crate::gallery::locale::Locale::En);
        for scale in [1.25, 1.5] {
            text.set_type_scale(scale);
            let title = text.proposal_label(
                "tekes-goal-completed",
                proposal::TextRole::UiBody,
                Some(242.0),
            );
            let original =
                text.proposal_label("Original", proposal::TextRole::UiCaption, Some(105.0));
            let default = text.proposal_label(
                "Default · Component",
                proposal::TextRole::UiCaption,
                Some(105.0),
            );
            if scale == 1.5 {
                assert_eq!(title.lines.len(), 1);
                assert_eq!(original.lines.len(), 1);
                assert_eq!(default.lines.len(), 2);
                assert!(original.height > 20.0);
                assert!(default.height > 20.0);
            }
            let size = text.proposal_label("24", proposal::TextRole::UiCaption, Some(32.0));
            let layout = scaled_card_text_layout(
                Rect::new(0.0, 0.0, 274.0, 148.0),
                title.height as f32,
                original.height.max(default.height) as f32,
                size.height as f32,
            );
            let caption_bottom = layout.caption_top + layout.caption_height;
            let preview_top = layout.preview_center_y - preview_sizes()[2] / 2.0;
            let size_label_bottom = layout.size_label_top + layout.size_label_height;

            assert!(layout.title_height >= title.height as f32);
            assert!(layout.caption_height >= original.height as f32);
            assert!(layout.caption_height >= default.height as f32);
            assert!(caption_bottom <= preview_top);
            assert!(size_label_bottom <= 148.0);
        }
    }

    #[test]
    fn three_line_goal_caption_clears_preview_and_matches_scaled_scroll_geometry() {
        let scale = 1.5;
        let mut text = crate::gallery::text::TextEngine::new();
        text.set_locale(crate::gallery::locale::Locale::En);
        text.set_type_scale(scale);
        let caption = text.proposal_label(
            "默认使用 · 完成目标",
            proposal::TextRole::UiCaption,
            Some(105.0),
        );
        assert_eq!(caption.lines.len(), 3);

        let title = text.proposal_label(
            "tekes-goal-completed",
            proposal::TextRole::UiBody,
            Some(242.0),
        );
        let size = text.proposal_label("24", proposal::TextRole::UiCaption, Some(32.0));
        let tile = Rect::new(0.0, 0.0, 274.0, scaled_tile_height(scale));
        let layout = scaled_card_text_layout(
            tile,
            title.height as f32,
            caption.height as f32,
            size.height as f32,
        );
        let caption_bottom = layout.caption_top + layout.caption_height;
        let preview_top = layout.preview_center_y - preview_sizes()[2] / 2.0;
        assert!(caption_bottom <= preview_top);

        let catalog = IconCatalog::fixture(&[
            "tekes-goal-00",
            "tekes-goal-01",
            "tekes-goal-02",
            "tekes-goal-03",
            "tekes-goal-04",
            "tekes-goal-05",
            "tekes-goal-06",
            "tekes-goal-07",
            "tekes-goal-08",
            "tekes-goal-09",
            "tekes-goal-10",
            "tekes-goal-11",
            "tekes-goal-12",
            "tekes-goal-13",
            "tekes-goal-14",
            "tekes-goal-15",
            "tekes-goal-16",
            "tekes-goal-17",
            "tekes-goal-18",
            "tekes-goal-19",
            "tekes-goal-20",
            "tekes-goal-21",
            "tekes-goal-22",
            "tekes-goal-23",
        ]);
        let content = Rect::new(312.0, 104.0, 564.0, 300.0);
        let scaled_extent = extent_at_scale(&catalog, "goal", content.size.width, scale);
        let last_row_offset = 11.0 * scaled_row_step(scale);
        let last_content = content.offset(0.0, -last_row_offset);
        let last_bounds = Rect::new(content.min_x(), content.min_y(), content.size.width, 300.0);
        let last_tiles = [22, 23].map(|index| tile_rect_at_scale(content, index, scale));
        assert!(
            last_tiles
                .iter()
                .all(|tile| { tile.max_y() <= content.min_y() + scaled_extent.height })
        );
        assert_eq!(
            visible_legacy_at_scale(&catalog, "goal", last_content, last_bounds, scale),
            vec![20, 21, 22, 23]
        );
        assert!(
            extent_at_scale(&catalog, "goal", content.size.width, scale).height
                > extent_at_scale(&catalog, "goal", content.size.width, 1.0).height
        );
    }

    #[derive(Default)]
    struct CountingRenderer {
        frames: Vec<Rect>,
    }

    impl IconRenderer for CountingRenderer {
        fn draw_icon(&mut self, _: &mut Scene, _: &str, frame: Rect, _: tekes_ui_tokens::Rgba) {
            self.frames.push(frame);
        }
    }

    #[test]
    fn component_icon_frame_is_fitted_and_renderer_is_skipped_when_fully_clipped() {
        let palette = tekes_ui_tokens::Palette::from(tekes_ui_tokens::Theme::LIGHT);
        for size in [12.0, 24.0, 48.0] {
            let mut scene = Scene::new();
            let mut renderer = CountingRenderer::default();
            let frame = Rect::new(10.0, 20.0, 80.0, 64.0);
            let layout = IconView::default().render(
                &mut scene,
                &mut renderer,
                "check",
                frame,
                size,
                Some(Rect::new(0.0, 0.0, 120.0, 120.0)),
                palette,
            );
            assert_eq!(layout.artboard.size.width, size);
            assert_eq!(layout.artboard.size.height, size);
            assert_eq!(renderer.frames, vec![layout.artboard]);

            let mut renderer = CountingRenderer::default();
            let clipped = IconView::default().render(
                &mut scene,
                &mut renderer,
                "check",
                frame,
                size,
                Some(Rect::new(200.0, 200.0, 10.0, 10.0)),
                palette,
            );
            assert!(clipped.visible.is_none());
            assert!(renderer.frames.is_empty());
        }
    }

    #[test]
    fn component_icon_rejects_non_finite_geometry_and_fits_narrow_bounds() {
        let palette = tekes_ui_tokens::Palette::from(tekes_ui_tokens::Theme::LIGHT);
        let mut scene = Scene::new();
        let mut renderer = CountingRenderer::default();
        let invalid = IconView::default().render(
            &mut scene,
            &mut renderer,
            "check",
            Rect::new(f32::NAN, 0.0, 24.0, 24.0),
            24.0,
            Some(Rect::new(0.0, 0.0, 100.0, 100.0)),
            palette,
        );
        assert!(invalid.visible.is_none());
        assert!(renderer.frames.is_empty());

        let narrow = IconView::default().render(
            &mut scene,
            &mut renderer,
            "check",
            Rect::new(4.0, 8.0, 10.0, 18.0),
            48.0,
            Some(Rect::new(0.0, 0.0, 100.0, 100.0)),
            palette,
        );
        assert_eq!(narrow.artboard.size.width, 10.0);
        assert_eq!(narrow.artboard.size.height, 18.0_f32.min(10.0));
        assert_eq!(renderer.frames.len(), 1);
    }

    #[test]
    fn icon_tiles_reflow_inside_narrow_viewports_without_horizontal_scroll() {
        for width in [384.0, 664.0, 1104.0] {
            let content = Rect::new(312.0, 104.0, width, 400.0);
            for index in 0..12 {
                let tile = tile_rect(content, index);
                assert!(tile.min_x() >= content.min_x());
                assert!(tile.max_x() <= content.max_x());
            }
        }
        assert_eq!(columns(384.0), 1);
        assert_eq!(columns(1104.0), 2);
    }

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
        assert!(
            last.max_y()
                <= content.min_y()
                    + extent(
                        &catalog,
                        "",
                        super::super::super::layout::PAGE_CONTENT.size.width
                    )
                    .height
        );
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

    #[test]
    fn new_old_and_refined_share_name_search_and_comparison_culling() {
        let mut catalog = IconCatalog::fixture(&["tekes-settings", "tekes-search"]);
        catalog.add_proposals();
        assert_eq!(filtered(&catalog, "SETT").len(), 1);
        assert_eq!(filtered(&catalog, "cad-").len(), 14);
        assert_eq!(filtered(&catalog, "drama-").len(), 14);
        assert_eq!(filtered(&catalog, "SeArCh").len(), 2);
        assert_eq!(filtered(&catalog, "设置").len(), 1);
        let visible = visible_legacy(
            &catalog,
            "settings",
            layout::PAGE_CONTENT,
            layout::PAGE_CONTENT,
        );
        assert_eq!(visible.len(), 1);
        assert!(catalog.entries[visible[0]].before.is_some());
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
        catalog.add_proposals();
        assert_eq!(catalog.entries.len(), 176);
        assert_eq!(
            catalog
                .entries
                .iter()
                .filter(|e| e.before.is_some())
                .count(),
            8
        );
        assert!(catalog.notices.is_empty(), "{:?}", catalog.notices);
        assert_eq!(filtered(&catalog, "").len(), 186);
        assert_eq!(filtered(&catalog, "SeArCh").len(), 3);
        let mut renderer = Renderer::new_headless().unwrap();
        let bounds = layout::PAGE_CONTENT;
        for theme in Theme::ALL {
            for scale in [1.0, 2.0] {
                // Traverse every row: all 133 SVGs must actually produce raster handles.
                for row in 0..186_usize.div_ceil(2) {
                    let content = bounds.offset(0.0, -(row as f32 * ROW_STEP));
                    let visible = visible_legacy(&catalog, "", content, bounds);
                    catalog.prepare(&mut renderer, theme, scale, &visible);
                }
                assert_eq!(renderer.image_count(), (176 + 8) * 3);
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
                        extent(
                            &catalog,
                            "",
                            super::super::super::layout::PAGE_CONTENT.size.width,
                        )
                        .height
                            - bounds.size.height,
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
