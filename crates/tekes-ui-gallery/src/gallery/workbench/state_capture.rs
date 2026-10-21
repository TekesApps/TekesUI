//! End-to-end raster captures for every implemented component state/variant story.
//!
//! These fixtures build the real Gallery route and rasterize its complete scene.
//! Pixel presence is only evidence of rendered output; it does not verify input,
//! keyboard, assistive-technology, or native-window behavior.

use super::{Entry, entries};
use crate::gallery::{Gallery, static_text::StaticText};
use tekes_ui_render::Renderer;
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Theme;

const POSITIONS: [(&str, f32); 3] = [("top", 0.), ("mid", 0.5), ("bottom", 1.)];
const VIEWPORTS: [(u32, u32, usize, f32); 2] = [(720, 512, 2, 1.5), (1440, 960, 0, 1.0)];

#[test]
fn implemented_component_state_and_variant_stories_render_at_each_viewport_position() {
    let mut renderer = Renderer::new_headless()
        .expect("headless GPU is required for real Gallery state-page captures");
    let output = std::env::var_os("TEKESUI_GOLD7_PAGE_OUT").map(std::path::PathBuf::from);
    let output = output.map(|root| root.join("state-pages"));
    if let Some(output) = &output {
        // This exact output folder is owned by this capture test. Clearing it
        // prevents stale PNGs from appearing to belong to the current run.
        if output.exists() {
            std::fs::remove_dir_all(output).expect("clear previous state-page captures");
        }
        std::fs::create_dir_all(output).expect("create state-page capture folder");
    }

    let requested = std::env::var("TEKESUI_STATE_COMPONENTS")
        .ok()
        .map(|value| value.split(',').map(str::to_owned).collect::<Vec<_>>());
    if let Some(requested) = &requested {
        assert!(!requested.is_empty());
        for id in requested {
            assert!(
                entries().iter().any(|entry| entry.component() == id),
                "unknown component {id}"
            );
        }
    }
    let scale: f32 = std::env::var("TEKESUI_STATE_SCALE")
        .ok()
        .map(|value| value.parse().expect("numeric capture scale"))
        .unwrap_or(1.0);
    assert!([1.0, 2.0].contains(&scale));
    let brand: usize = std::env::var("TEKESUI_STATE_BRAND")
        .ok()
        .map(|value| value.parse().expect("numeric existing brand index"))
        .unwrap_or(0);
    assert!(tekes_ui_tokens::Palette::new(Theme::LIGHT, brand).is_some());
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        let theme_name = theme_slug(theme);
        for (width, height, text_scale, type_scale) in VIEWPORTS {
            for (entry_index, entry) in entries().iter().enumerate().filter(|(_, entry)| {
                is_implemented_component(entry)
                    && requested
                        .as_ref()
                        .is_none_or(|ids| ids.iter().any(|id| id == entry.component()))
                    && (entry.supports_story("states") || entry.supports_story("variants"))
            }) {
                for story in ["states", "variants"]
                    .into_iter()
                    .filter(|story| entry.supports_story(story))
                {
                    let route = format!("{}/{story}", entry.slug());
                    let mut gallery = Gallery::with_static_text(StaticText::new(None));
                    gallery.set_system_theme(theme);
                    gallery.set_reduced_motion(true);
                    gallery.workbench.text_scale = text_scale;
                    gallery.text.set_scale(scale);
                    assert!(gallery.open_route(&route), "route must resolve: {route}");
                    gallery.set_view_size(width as f32, height as f32);
                    gallery.workbench.state(entry_index).brand = brand;

                    stabilize_document(&mut gallery);
                    let range = gallery.page_scroll_range().max(0.);

                    for (position, fraction) in POSITIONS {
                        let requested_offset = range * fraction;
                        let (actual_offset, actual_range) =
                            settle_scroll(&mut gallery, fraction, requested_offset);
                        assert!(
                            (actual_offset - actual_range * fraction).abs() < 0.5,
                            "{route}/{theme_name}/{width}x{height}/{position} must use its real document offset"
                        );
                        let mut scene = Scene::new();
                        gallery.build_scene(&mut scene);
                        assert!(!scene.is_empty(), "empty Gallery scene at {route}");

                        let rgba = renderer
                            .render_to_rgba8(
                                &scene,
                                gallery.atlas_mut(),
                                (
                                    (width as f32 * scale) as u32,
                                    (height as f32 * scale) as u32,
                                ),
                                scale,
                                Gallery::background(theme),
                            )
                            .unwrap_or_else(|error| {
                                panic!(
                                    "GPU render failed for {route}/{theme_name}/{position}: {error}"
                                )
                            });
                        let has_non_background_pixels =
                            rgba.chunks_exact(4).any(|pixel| pixel != &rgba[0..4]);
                        assert!(
                            has_non_background_pixels,
                            "blank raster for {route}/{theme_name}/{width}x{height}/{position}"
                        );

                        let file_name = format!(
                            "{}__{}__{}__{}x{}__type-{}__{}.png",
                            entry.slug(),
                            story,
                            theme_name,
                            width,
                            height,
                            (type_scale * 100.).round() as u32,
                            position
                        );
                        if let Some(output) = &output {
                            write_png(
                                output.join(&file_name),
                                (width as f32 * scale) as u32,
                                (height as f32 * scale) as u32,
                                &rgba,
                            );
                        }
                        cases.push(serde_json::json!({
                            "entry_index": entry_index,
                            "slug": entry.slug(),
                            "renderer": entry.renderer(),
                            "story": story,
                            "route": route,
                            "theme": theme_name,
                            "brand": brand,
                            "window": {"width": width, "height": height},
                            "type_scale": type_scale,
                            "backing_scale": scale,
                            "document_height": gallery.page_document_height(),
                            "position": position,
                            "requested_offset": requested_offset,
                            "actual_offset": actual_offset,
                            "actual_scroll_range": actual_range,
                            "has_non_background_pixels": has_non_background_pixels,
                            "evidence": "real Gallery scene rasterized on headless GPU; pixel presence is not interaction acceptance"
                        }));
                    }
                }
            }
        }
    }

    assert!(
        !cases.is_empty(),
        "there must be implemented state or variant stories"
    );
    if let Some(output) = output {
        std::fs::write(
            output.join("manifest.json"),
            serde_json::to_vec_pretty(&cases).expect("serialize capture manifest"),
        )
        .expect("write capture manifest");
    }
}

fn is_implemented_component(entry: &Entry) -> bool {
    entry.get("kind") == "component" && !entry.planned()
}

pub(super) fn stabilize_document(gallery: &mut Gallery) {
    let mut previous = None;
    for _ in 0..4 {
        gallery.build_scene(&mut Scene::new());
        let current = (gallery.page_document_height(), gallery.page_scroll_range());
        if previous.is_some_and(|(height, range): (f32, f32)| {
            (current.0 - height).abs() < 0.1 && (current.1 - range).abs() < 0.1
        }) {
            return;
        }
        previous = Some(current);
    }
    // Workbench story extents are allowed one extra host pass to settle. The
    // last measured values remain the actual range used by the captures.
    gallery.build_scene(&mut Scene::new());
}

pub(super) fn settle_scroll(gallery: &mut Gallery, fraction: f32, mut target: f32) -> (f32, f32) {
    let mut actual = 0.;
    let mut range = gallery.page_scroll_range().max(0.);
    for _ in 0..4 {
        gallery.scroll_page_to(target);
        gallery.build_scene(&mut Scene::new());
        range = gallery.page_scroll_range().max(0.);
        actual = gallery.viewport.offset.y;
        let next = range * fraction;
        if (actual - next).abs() < 0.5 {
            break;
        }
        target = next;
    }
    (actual, range)
}

fn theme_slug(theme: Theme) -> String {
    format!("{:?}-{:?}", theme.appearance, theme.contrast).to_lowercase()
}

fn write_png(path: std::path::PathBuf, width: u32, height: u32, rgba: &[u8]) {
    let file = std::fs::File::create(path).expect("create state-page PNG");
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("write PNG header")
        .write_image_data(rgba)
        .expect("write PNG pixels");
}
