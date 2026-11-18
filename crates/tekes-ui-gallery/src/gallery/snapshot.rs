//! Headless AppHost snapshots: render canonical AppHost Gallery routes to PNGs
//! using `Renderer::new_headless` and `tekes_ui_app_host::HostedWindow`.
//!
//! File names follow `<route-slug>-<theme>@<scale>x.png`. Static text snapshots
//! force `StaticText::new(None)` so output is independent of `TEKESUI_REFERENCE_DIR`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::Rect;
use tekes_ui_tokens::Theme;

use super::apphost_workbench::HeadlessGallery;
use super::layout::{FRAME_H, FRAME_W};
use super::locale::Locale;
use super::motion_preview::PresetCatalog;
use super::theme::theme_slug;
use super::workbench;

/// Default snapshot scales.
pub const SCALES: [u32; 2] = [2, 1];

/// Representative AppHost routes exported when `TEKESUI_GALLERY_PAGE` is unset.
pub const DEFAULT_ROUTES: &[&str] = &[
    "foundation-tokens",
    "foundation-shape",
    "foundation-typography",
    "foundation-motion",
    "component-button",
    "component-text-field",
    "component-scroll-area",
    "component-icon",
    "lab-calibration",
];

#[derive(Debug)]
pub enum SnapshotError {
    Render(RenderError),
    Png(png::EncodingError),
    Io(std::io::Error),
    Host(anyhow::Error),
    MotionHasNoRenderedButtons,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::Render(e) => write!(f, "渲染失败：{e}"),
            SnapshotError::Png(e) => write!(f, "PNG 编码失败：{e}"),
            SnapshotError::Io(e) => write!(f, "写文件失败：{e}"),
            SnapshotError::Host(e) => write!(f, "AppHost 快照失败：{e:#}"),
            SnapshotError::MotionHasNoRenderedButtons => {
                write!(f, "Motion 快照没有构建真实可交互按钮")
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

impl From<RenderError> for SnapshotError {
    fn from(e: RenderError) -> Self {
        SnapshotError::Render(e)
    }
}
impl From<png::EncodingError> for SnapshotError {
    fn from(e: png::EncodingError) -> Self {
        SnapshotError::Png(e)
    }
}
impl From<std::io::Error> for SnapshotError {
    fn from(e: std::io::Error) -> Self {
        SnapshotError::Io(e)
    }
}
impl From<anyhow::Error> for SnapshotError {
    fn from(e: anyhow::Error) -> Self {
        SnapshotError::Host(e)
    }
}

pub fn route_file_name(route_slug: &str, theme: Theme, scale: u32) -> String {
    format!("{route_slug}-{}@{scale}x.png", theme_slug(theme))
}

/// Render a single AppHost Gallery route at `FRAME_W x FRAME_H` pt to PNG bytes.
pub fn render_route_png(
    renderer: &mut Renderer,
    gallery: &mut HeadlessGallery,
    raw_route: &str,
    theme: Theme,
    scale: u32,
) -> Result<(String, Vec<u8>), SnapshotError> {
    gallery.switch_route(raw_route)?;
    let slug = gallery.state().borrow().current_route().to_owned();
    gallery.set_theme(theme);
    gallery.window_mut().set_type_scale(1.0);
    gallery.window_mut().set_motion_elapsed(Duration::ZERO);
    if slug == "lab-calibration" {
        gallery
            .state()
            .borrow_mut()
            .clear_calibration_reference_dir();
    }
    if slug == "foundation-motion" {
        let locale = Locale::default();
        let snapshot = render_motion_case(
            renderer,
            MotionSnapshotRequest {
                requested_route: raw_route,
                theme,
                locale,
                type_scale: 1.0,
                pixel_scale: scale,
                width: FRAME_W,
                catalog: PresetCatalog::Apple,
            },
        )?;
        if snapshot.case["rendered_button_count"].as_u64().unwrap_or(0) == 0 {
            return Err(SnapshotError::MotionHasNoRenderedButtons);
        }
        return Ok((slug, snapshot.png));
    }
    let size_pt = (FRAME_W as u32, FRAME_H as u32);
    let rgba = gallery
        .window_mut()
        .render_headless_rgba8(renderer, size_pt, scale)?;
    Ok((
        slug,
        encode_png(&rgba, size_pt.0 * scale, size_pt.1 * scale)?,
    ))
}

#[derive(Debug)]
struct MotionSnapshot {
    png: Vec<u8>,
    case: serde_json::Value,
}

#[derive(Clone, Copy)]
struct MotionSnapshotRequest<'a> {
    requested_route: &'a str,
    theme: Theme,
    locale: Locale,
    type_scale: f32,
    pixel_scale: u32,
    width: f32,
    catalog: PresetCatalog,
}

/// Captures Motion stories through the real AppHost Gallery container stack.
fn render_motion_case(
    renderer: &mut Renderer,
    request: MotionSnapshotRequest<'_>,
) -> Result<MotionSnapshot, SnapshotError> {
    let MotionSnapshotRequest {
        requested_route,
        theme,
        locale,
        type_scale,
        pixel_scale,
        width,
        catalog,
    } = request;
    let mut gallery = HeadlessGallery::open("foundation-motion/playground")?;
    gallery.set_theme(theme);
    gallery.window_mut().set_type_scale(type_scale);
    gallery.window_mut().set_motion_elapsed(Duration::ZERO);
    gallery.state().borrow_mut().set_locale(locale);
    gallery.state().borrow_mut().with_current_page(|page| {
        if let Some(motion) = page.motion_mut() {
            motion.select_catalog(catalog);
        }
    });

    let whole = Rect::new(0.0, 0.0, width, FRAME_H);
    let (scene, clear) =
        gallery
            .window_mut()
            .build_headless_scene(renderer, whole, pixel_scale as f32, true)?;

    let diag = gallery.state().borrow().with_current_page_ref(|page| {
        page.motion()
            .map(|motion| motion.motion_case_diagnostics())
            .unwrap_or(serde_json::Value::Null)
    });
    let rendered_button_count = diag["rendered_button_count"].as_u64().unwrap_or(0);
    if rendered_button_count == 0 {
        return Err(SnapshotError::MotionHasNoRenderedButtons);
    }

    let output_size = (width as u32 * pixel_scale, FRAME_H as u32 * pixel_scale);
    let rgba = renderer.render_to_rgba8(
        &scene,
        gallery.window_mut().glyph_atlas_mut(),
        output_size,
        pixel_scale as f32,
        clear,
    )?;
    let rendered_page = gallery.state().borrow().current_route().to_owned();
    let case = serde_json::json!({
        "requested_page": requested_route,
        "rendered_page": rendered_page,
        "theme": theme_slug(theme),
        "locale": locale_slug(locale),
        "width": width,
        "height": FRAME_H,
        "type_scale": type_scale,
        "pixel_scale": pixel_scale,
        "catalog": catalog_slug(catalog),
        "document_extent": diag["document_extent"],
        "scroll_range": diag["scroll_range"],
        "scene_primitives": scene.len(),
        "rendered_button_count": rendered_button_count,
        "visible_button_count": diag["visible_button_count"],
        "buttons": diag["buttons"],
    });
    Ok(MotionSnapshot {
        png: encode_png(&rgba, output_size.0, output_size.1)?,
        case,
    })
}

#[cfg(test)]
fn export_motion_matrix(renderer: &mut Renderer, root: &Path) -> Result<usize, SnapshotError> {
    fs::create_dir_all(root)?;
    let mut manifest_cases = Vec::new();
    let mut count = 0;
    for theme in Theme::ALL {
        for locale in Locale::ALL {
            for width in [720.0_f32, 1000.0, 1440.0] {
                for type_scale in [1.0_f32, 1.5] {
                    for catalog in [
                        PresetCatalog::Apple,
                        PresetCatalog::DesignSystem,
                        PresetCatalog::Recipes,
                        PresetCatalog::Symbols,
                    ] {
                        let motion = render_motion_case(
                            renderer,
                            MotionSnapshotRequest {
                                requested_route: "foundation-motion/playground",
                                theme,
                                locale,
                                type_scale,
                                pixel_scale: 1,
                                width,
                                catalog,
                            },
                        )?;
                        let stem = format!(
                            "motion-{}-{}-w{}-t{}-{}",
                            theme_slug(theme),
                            locale_slug(locale),
                            width as u32,
                            if type_scale == 1.0 { "100" } else { "150" },
                            catalog_slug(catalog),
                        );
                        let png_path = root.join(format!("{stem}.png"));
                        let case_path = root.join(format!("{stem}.json"));
                        fs::write(&png_path, &motion.png)?;
                        fs::write(&case_path, format!("{}\n", motion.case))?;
                        manifest_cases.push(serde_json::json!({
                            "id": stem,
                            "png": png_path.file_name().and_then(|name| name.to_str()),
                            "case": case_path.file_name().and_then(|name| name.to_str()),
                            "rendered_button_count": motion.case["rendered_button_count"],
                            "visible_button_count": motion.case["visible_button_count"],
                        }));
                        count += 1;
                    }
                }
            }
        }
    }
    let manifest = serde_json::json!({
        "total": count,
        "themes": Theme::ALL.map(theme_slug),
        "locales": Locale::ALL.map(locale_slug),
        "widths": [720, 1000, 1440],
        "type_scales": [1.0, 1.5],
        "catalogs": ["apple", "design-system", "recipes", "symbols"],
        "cases": manifest_cases,
    });
    fs::write(
        root.join("manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
    )?;
    Ok(count)
}

fn locale_slug(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "en",
        Locale::ZhCn => "zh-CN",
        Locale::Reference => "reference",
    }
}

fn catalog_slug(catalog: PresetCatalog) -> &'static str {
    match catalog {
        PresetCatalog::Apple => "apple",
        PresetCatalog::DesignSystem => "design-system",
        PresetCatalog::Recipes => "recipes",
        PresetCatalog::Symbols => "symbols",
        PresetCatalog::TickingNumber => "ticking-number",
    }
}

fn encode_png(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>, SnapshotError> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header()?;
        writer.write_image_data(rgba)?;
    }
    Ok(out)
}

/// Export snapshots for all selected routes x themes x scales into `dir`.
pub fn write_all(dir: &Path) -> Result<Vec<PathBuf>, SnapshotError> {
    fs::create_dir_all(dir)?;
    let routes: Vec<String> = match std::env::var("TEKESUI_GALLERY_PAGE") {
        Ok(value) if value == "all" => workbench::entries()
            .iter()
            .map(|entry| entry.slug().to_owned())
            .collect(),
        Ok(value) => vec![value],
        Err(_) => DEFAULT_ROUTES.iter().map(|s| (*s).to_owned()).collect(),
    };
    let mut renderer = Renderer::new_headless()?;
    let mut gallery = HeadlessGallery::open(&routes[0])?;
    let mut written = Vec::new();
    for route in &routes {
        for theme in Theme::ALL {
            for scale in SCALES {
                let (slug, png) =
                    render_route_png(&mut renderer, &mut gallery, route, theme, scale)?;
                let path = dir.join(route_file_name(&slug, theme, scale));
                fs::write(&path, png)?;
                written.push(path);
            }
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::text::TextEngine;

    #[test]
    fn file_names() {
        assert_eq!(
            route_file_name("foundation-tokens", Theme::LIGHT, 2),
            "foundation-tokens-light@2x.png"
        );
        assert_eq!(
            route_file_name("foundation-tokens", Theme::DARK_INCREASED, 1),
            "foundation-tokens-dark-ic@1x.png"
        );
        assert_eq!(
            route_file_name("lab-calibration", Theme::DARK, 2),
            "lab-calibration-dark@2x.png"
        );
        for route in DEFAULT_ROUTES {
            crate::gallery::apphost_workbench::resolve_route(route)
                .unwrap_or_else(|e| panic!("DEFAULT_ROUTES entry {route} failed: {e}"));
        }
    }

    #[test]
    fn ticking_number_gpu_frames_keep_fixed_width_and_finish() {
        let Ok(mut renderer) = Renderer::new_headless() else {
            eprintln!("UNVERIFIED: no headless Metal adapter; skipping ticking number GPU test");
            return;
        };
        let mut gallery = HeadlessGallery::open("foundation-motion/playground").unwrap();
        gallery.state().borrow_mut().with_current_page(|page| {
            let motion = page.motion_mut().unwrap();
            motion.select_catalog(PresetCatalog::TickingNumber);
        });
        let size_pt = (1000_u32, FRAME_H as u32);
        let whole = Rect::new(0.0, 0.0, size_pt.0 as f32, size_pt.1 as f32);
        // Prime resources so Motion receives its initial runnable frame before retargeting.
        gallery
            .window_mut()
            .build_headless_scene(&mut renderer, whole, 1.0, true)
            .unwrap();
        gallery.state().borrow_mut().with_current_page(|page| {
            page.motion_mut().unwrap().set_ticking_target(546).unwrap();
        });
        let mut text = TextEngine::new();
        let mut widths = Vec::new();
        let mut images = Vec::new();
        let mut elapsed = Duration::ZERO;
        for (index, delta) in [0.0_f64, 0.9, 1.0].into_iter().enumerate() {
            elapsed += Duration::from_secs_f64(delta);
            gallery.window_mut().set_motion_elapsed(elapsed);
            let rgba = gallery
                .window_mut()
                .render_headless_rgba8(&mut renderer, size_pt, 1)
                .unwrap();
            let width = gallery.state().borrow().with_current_page_ref(|page| {
                page.motion().unwrap().measure_ticking_width(&mut text)
            });
            widths.push(width);
            if let Ok(dir) = std::env::var("TEKESUI_TICKING_SNAPSHOT_OUT") {
                fs::create_dir_all(&dir).unwrap();
                fs::write(
                    Path::new(&dir).join(format!("ticking-{index}.png")),
                    encode_png(&rgba, size_pt.0, size_pt.1).unwrap(),
                )
                .unwrap();
            }
            images.push(rgba);
        }
        assert_eq!(widths[0], widths[1]);
        assert_eq!(widths[1], widths[2]);
        assert_ne!(images[0], images[1]);
        assert_ne!(images[1], images[2]);
        let (value, animating) = gallery.state().borrow().with_current_page_ref(|page| {
            let motion = page.motion().unwrap();
            (motion.ticking_value(), motion.ticking_needs_animation())
        });
        assert_eq!(value, 546);
        assert!(!animating);
    }

    /// Set TEKESUI_MOTION_SNAPSHOT_OUT to export the real, full AppHost Motion
    /// matrix. A missing headless GPU is reported as unverified, never as pass.
    #[test]
    fn export_live_motion_matrix_when_requested() {
        let Ok(output) = std::env::var("TEKESUI_MOTION_SNAPSHOT_OUT") else {
            eprintln!("SKIPPED: set TEKESUI_MOTION_SNAPSHOT_OUT to export Motion snapshots");
            return;
        };
        let mut renderer = match Renderer::new_headless() {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!(
                    "UNVERIFIED: no headless GPU adapter; Motion export unavailable: {error}"
                );
                return;
            }
        };
        let count = export_motion_matrix(&mut renderer, Path::new(&output)).unwrap();
        assert_eq!(count, 192);
        eprintln!("EXPORTED: {count} real Gallery Motion PNG/case pairs to {output}");
    }

    #[test]
    fn apphost_motion_snapshots_use_live_gallery_buttons() {
        let Ok(mut renderer) = Renderer::new_headless() else {
            eprintln!("UNVERIFIED: no GPU adapter for live Motion snapshot path");
            return;
        };
        for requested in ["foundation-motion", "foundation-motion/playground"] {
            let rendered = render_motion_case(
                &mut renderer,
                MotionSnapshotRequest {
                    requested_route: requested,
                    theme: Theme::LIGHT,
                    locale: Locale::En,
                    type_scale: 1.0,
                    pixel_scale: 1,
                    width: FRAME_W,
                    catalog: PresetCatalog::Apple,
                },
            )
            .unwrap();
            assert!(rendered.case["rendered_button_count"].as_u64().unwrap() > 0);
            assert!(rendered.case["visible_button_count"].as_u64().unwrap() > 0);
            assert!(rendered.case["scene_primitives"].as_u64().unwrap() > 1);
            let reader = png::Decoder::new(std::io::Cursor::new(rendered.png))
                .read_info()
                .unwrap();
            assert_eq!(
                (reader.info().width, reader.info().height),
                (FRAME_W as u32, FRAME_H as u32)
            );
        }
    }

    /// Determinism: rendering the same AppHost route twice produces byte-identical PNGs.
    #[test]
    fn rendering_twice_is_byte_identical() {
        let Ok(mut renderer) = Renderer::new_headless() else {
            eprintln!("没有可用的 GPU 适配器，跳过");
            return;
        };
        let mut gallery = HeadlessGallery::open("foundation-tokens").unwrap();
        for (route, theme, scale) in [
            ("foundation-tokens", Theme::LIGHT, 2),
            ("foundation-tokens", Theme::DARK_INCREASED, 1),
            ("lab-calibration", Theme::LIGHT, 2),
            ("lab-calibration", Theme::DARK, 1),
            ("lab-calibration", Theme::DARK_INCREASED, 2),
        ] {
            let (_, a) =
                render_route_png(&mut renderer, &mut gallery, route, theme, scale).unwrap();
            let (_, b) =
                render_route_png(&mut renderer, &mut gallery, route, theme, scale).unwrap();
            assert_eq!(a, b, "{route} {theme:?} {scale}x");
            let mut fresh_renderer = Renderer::new_headless().unwrap();
            let mut fresh_gallery = HeadlessGallery::open(route).unwrap();
            let (_, c) =
                render_route_png(&mut fresh_renderer, &mut fresh_gallery, route, theme, scale)
                    .unwrap();
            assert_eq!(a, c, "{route} {theme:?} {scale}x (fresh host/renderer)");
        }
    }

    #[test]
    fn token_snapshot_scene_contains_current_theme_colors_without_reverse_mapping() {
        let Ok(mut renderer) = Renderer::new_headless() else {
            eprintln!("没有可用的 GPU 适配器，跳过");
            return;
        };
        let mut gallery = HeadlessGallery::open("foundation-tokens").unwrap();
        let whole = Rect::new(0.0, 0.0, FRAME_W, FRAME_H);
        let mut clears = Vec::new();
        for theme in [Theme::LIGHT, Theme::DARK] {
            gallery.set_theme(theme);
            let (scene, clear) = gallery
                .window_mut()
                .build_headless_scene(&mut renderer, whole, 1.0, true)
                .unwrap();
            let expected = tekes_ui_tokens::Palette::from(theme)
                .color("surface-1")
                .unwrap();
            assert_eq!(clear, expected);
            assert!(scene.primitives().iter().any(|p| {
                matches!(p, tekes_ui_scene::Primitive::Quad(q) if q.fill == expected)
            }));
            clears.push(clear);
        }
        assert_ne!(clears[0], clears[1]);
    }
}
