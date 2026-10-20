//! 无头快照：用 `Renderer::new_headless` 把每一页在每个主题、每个缩放下渲染成 PNG。
//!
//! 文件名 `<页>-<主题>@<缩放>x.png`，例如 `primitives-dark-ic@2x.png`。
//! 页名与主题名见 `Page::slug`、`theme_slug`。尺寸是 JSON 捕获框乘缩放。
//! 这是以后自动对比（章程第 2 节）共用的固定夹具：同样的输入逐字节得到同样的 PNG。
//!
//! 静态文字页固定为默认状态：夹具 sentences-a、宽度 700，**不带参照目录**（参照窗格是占位文字，
//! 差异窗格写“需要参照图才能比较”），所以快照与环境变量 `TEKESUI_REFERENCE_DIR` 无关。

use std::fs;
use std::path::{Path, PathBuf};

use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Theme;

use super::layout::{FRAME_H, FRAME_W};
use super::locale::Locale;
use super::motion_preview::PresetCatalog;
use super::static_text::StaticText;
use super::text::TextEngine;
use super::theme::theme_slug;
use super::{Gallery, Page, PageCtx, compose};

/// 默认要输出的缩放。
pub const SCALES: [u32; 2] = [2, 1];

#[derive(Debug)]
pub enum SnapshotError {
    Render(RenderError),
    Png(png::EncodingError),
    Io(std::io::Error),
    MotionHasNoRenderedButtons,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::Render(e) => write!(f, "渲染失败：{e}"),
            SnapshotError::Png(e) => write!(f, "PNG 编码失败：{e}"),
            SnapshotError::Io(e) => write!(f, "写文件失败：{e}"),
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

pub fn file_name(page: Page, theme: Theme, scale: u32) -> String {
    format!("{}-{}@{}x.png", page.slug(), theme_slug(theme), scale)
}

/// 渲染一页，返回 PNG 字节。`static_text` 用 `StaticText::new(None)` 的默认状态。
pub fn render_png(
    renderer: &mut Renderer,
    text: &mut TextEngine,
    static_text: &mut StaticText,
    page: Page,
    theme: Theme,
    scale: u32,
) -> Result<Vec<u8>, SnapshotError> {
    if is_motion_snapshot_page(page) {
        let locale = text.locale();
        let type_scale = text.type_scale();
        let snapshot = render_motion_case(
            renderer,
            MotionSnapshotRequest {
                requested_page: page,
                theme,
                locale,
                type_scale,
                pixel_scale: scale,
                width: FRAME_W,
                catalog: PresetCatalog::Apple,
            },
        )?;
        if snapshot.case["rendered_button_count"].as_u64().unwrap_or(0) == 0 {
            return Err(SnapshotError::MotionHasNoRenderedButtons);
        }
        return Ok(snapshot.png);
    }
    text.set_scale(scale as f32);
    text.begin_frame();
    let mut scene = Scene::new();
    {
        let mut ctx = PageCtx {
            theme,
            text: &mut *text,
            static_text,
        };
        compose(page, None, &mut ctx, &mut scene);
    }
    let size = (FRAME_W as u32 * scale, FRAME_H as u32 * scale);
    let rgba = renderer.render_to_rgba8(
        &scene,
        text.atlas_mut(),
        size,
        scale as f32,
        Gallery::background(theme),
    )?;
    encode_png(&rgba, size.0, size.1)
}

fn is_motion_snapshot_page(page: Page) -> bool {
    match page {
        Page::Motion => true,
        Page::Workbench(index) => super::workbench::entry(index).renderer() == "motion",
        _ => false,
    }
}

#[cfg(test)]
fn export_motion_matrix(renderer: &mut Renderer, root: &Path) -> Result<usize, SnapshotError> {
    use super::workbench;

    fs::create_dir_all(root)?;
    let page = workbench::route("foundation-motion/playground")
        .ok_or(SnapshotError::MotionHasNoRenderedButtons)?;
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
                                requested_page: page,
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

#[derive(Debug)]
struct MotionSnapshot {
    png: Vec<u8>,
    case: serde_json::Value,
}

#[derive(Clone, Copy)]
struct MotionSnapshotRequest {
    requested_page: Page,
    theme: Theme,
    locale: Locale,
    type_scale: f32,
    pixel_scale: u32,
    width: f32,
    catalog: PresetCatalog,
}

/// Canonical and legacy Motion captures share the actual Gallery host renderer
/// so their buttons, chrome, viewport and live component models are present.
fn render_motion_case(
    renderer: &mut Renderer,
    request: MotionSnapshotRequest,
) -> Result<MotionSnapshot, SnapshotError> {
    let MotionSnapshotRequest {
        requested_page,
        theme,
        locale,
        type_scale,
        pixel_scale,
        width,
        catalog,
    } = request;
    let mut gallery = Gallery::with_static_text(StaticText::new(None));
    gallery.theme.system = theme;
    gallery.set_locale(locale);
    gallery.text.set_scale(pixel_scale as f32);
    gallery.workbench.text_scale = if type_scale >= 1.375 {
        2
    } else if type_scale >= 1.125 {
        1
    } else {
        0
    };
    gallery.motion.select_catalog(catalog);

    // Preserve canonical Workbench routes where supplied. The old Motion ID
    // maps to the canonical story because only it owns the full live page host.
    gallery.page = match requested_page {
        Page::Motion => super::workbench::route("foundation-motion/playground")
            .ok_or(SnapshotError::MotionHasNoRenderedButtons)?,
        page => page,
    };
    gallery.set_view_size(width, FRAME_H);
    gallery.prepare(renderer, pixel_scale as f32);
    let mut scene = Scene::new();
    gallery.build_scene(&mut scene);

    let actions = super::pages::motion::Layout::for_state(
        gallery.live_content_bounds(),
        &gallery.motion,
        &mut gallery.text,
    )
    .state_actions(&gallery.motion);
    let buttons: Vec<_> = actions
        .iter()
        .filter_map(|(action, _)| {
            let id = super::pages::motion::Layout::action_id(*action);
            let button = gallery.motion.button_models.get(&id)?;
            let layout = button.layout()?;
            let visible = layout.visible.filter(|rect| !rect.is_empty());
            Some(serde_json::json!({
                "id": format!("{id:016x}"),
                "action": format!("{action:?}"),
                "frame": rect_json(layout.frame),
                "content": rect_json(layout.content),
                "visible": visible.map(rect_json),
                "label_size": {"width": layout.label_size.width, "height": layout.label_size.height},
            }))
        })
        .collect();
    if buttons.is_empty() {
        return Err(SnapshotError::MotionHasNoRenderedButtons);
    }

    let output_size = (width as u32 * pixel_scale, FRAME_H as u32 * pixel_scale);
    let rgba = renderer.render_to_rgba8(
        &scene,
        gallery.atlas_mut(),
        output_size,
        pixel_scale as f32,
        Gallery::background(theme),
    )?;
    let case = serde_json::json!({
        "requested_page": requested_page.slug(),
        "rendered_page": gallery.page().slug(),
        "theme": theme_slug(theme),
        "locale": locale_slug(locale),
        "width": width,
        "height": FRAME_H,
        "type_scale": gallery.text.type_scale(),
        "pixel_scale": pixel_scale,
        "catalog": catalog_slug(catalog),
        "document_extent": gallery.page_document_height(),
        "scroll_range": gallery.page_scroll_range(),
        "scene_primitives": scene.len(),
        "rendered_button_count": buttons.len(),
        "visible_button_count": buttons.iter().filter(|button| !button["visible"].is_null()).count(),
        "buttons": buttons,
    });
    Ok(MotionSnapshot {
        png: encode_png(&rgba, output_size.0, output_size.1)?,
        case,
    })
}

fn rect_json(rect: tekes_ui_scene::Rect) -> serde_json::Value {
    serde_json::json!({
        "x": rect.min_x(),
        "y": rect.min_y(),
        "width": rect.size.width,
        "height": rect.size.height,
    })
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

/// 渲染全部页 x 主题 x 缩放，写进 `dir`，返回写出的文件。
pub fn write_all(dir: &Path) -> Result<Vec<PathBuf>, SnapshotError> {
    fs::create_dir_all(dir)?;
    let mut renderer = Renderer::new_headless()?;
    let mut text = TextEngine::new();
    let mut static_text = StaticText::new(None);
    let mut written = Vec::new();
    for page in Page::ALL {
        for theme in Theme::ALL {
            for scale in SCALES {
                let png = render_png(
                    &mut renderer,
                    &mut text,
                    &mut static_text,
                    page,
                    theme,
                    scale,
                )?;
                let path = dir.join(file_name(page, theme, scale));
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

    #[test]
    fn file_names() {
        assert_eq!(
            file_name(Page::Primitives, Theme::LIGHT, 2),
            "primitives-light@2x.png"
        );
        assert_eq!(
            file_name(Page::Tokens, Theme::DARK_INCREASED, 1),
            "tokens-dark-ic@1x.png"
        );
        assert_eq!(
            file_name(Page::StaticText, Theme::DARK, 2),
            "static-text-dark@2x.png"
        );
    }

    /// Set TEKESUI_MOTION_SNAPSHOT_OUT to export the real, full Gallery Motion
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
        // Keep one renderer for the complete matrix to exercise repeated atlas,
        // texture and drawable reuse across every catalog and viewport size.
        let count = export_motion_matrix(&mut renderer, Path::new(&output)).unwrap();
        assert_eq!(count, 192);
        eprintln!("EXPORTED: {count} real Gallery Motion PNG/case pairs to {output}");
    }

    #[test]
    fn legacy_and_canonical_motion_snapshots_use_live_gallery_buttons() {
        let Ok(mut renderer) = Renderer::new_headless() else {
            eprintln!("UNVERIFIED: no GPU adapter for live Motion snapshot path");
            return;
        };
        let canonical = super::super::workbench::route("foundation-motion/playground").unwrap();
        for requested in [Page::Motion, canonical] {
            let rendered = render_motion_case(
                &mut renderer,
                MotionSnapshotRequest {
                    requested_page: requested,
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

    /// 同一输入渲染两次，PNG 逐字节相同。没有 GPU 适配器时跳过。
    #[test]
    fn rendering_twice_is_byte_identical() {
        let Ok(mut renderer) = Renderer::new_headless() else {
            eprintln!("没有可用的 GPU 适配器，跳过");
            return;
        };
        let mut text = TextEngine::new();
        let mut st = StaticText::new(None);
        for (page, theme, scale) in [
            (Page::Primitives, Theme::LIGHT, 2),
            (Page::Tokens, Theme::DARK_INCREASED, 1),
            (Page::StaticText, Theme::LIGHT, 2),
            (Page::StaticText, Theme::DARK, 1),
            (Page::StaticText, Theme::DARK_INCREASED, 2),
        ] {
            let a = render_png(&mut renderer, &mut text, &mut st, page, theme, scale).unwrap();
            let b = render_png(&mut renderer, &mut text, &mut st, page, theme, scale).unwrap();
            assert_eq!(a, b, "{page:?} {theme:?} {scale}x");
            // 用新的渲染器、新的文字引擎再来一次，排除内部状态（atlas 打包顺序、标签缓存）的影响。
            let mut fresh = Renderer::new_headless().unwrap();
            let c = render_png(
                &mut fresh,
                &mut TextEngine::new(),
                &mut StaticText::new(None),
                page,
                theme,
                scale,
            )
            .unwrap();
            assert_eq!(a, c, "{page:?} {theme:?} {scale}x（新渲染器）");
        }
    }

    #[test]
    fn original_three_page_content_matches_the_committed_proposal_review() {
        // CoreText's automatic CJK cascade depends on preferred languages.
        // Run this historical fixture in its own process: changing defaults in
        // the parallel test process would also change unrelated text tests.
        // NSArgumentDomain is process-only; no system preference is written.
        let profile: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../design/reviews/proposal1/reference-language-profile-20261019.json"
        )))
        .unwrap();
        let expected: Vec<_> = profile["preferred_languages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect();
        if std::env::var_os("TEKESUI_GOLDEN_REFERENCE_CHILD").is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "gallery::snapshot::tests::original_three_page_content_matches_the_committed_proposal_review",
                    "--nocapture",
                    "--",
                    "-AppleLanguages",
                    &format!("({})", expected.join(",")),
                ])
                .env("TEKESUI_GOLDEN_REFERENCE_CHILD", "1")
                .output()
                .expect("launch isolated reference render");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("{stdout}{stderr}");
            assert!(output.status.success(), "isolated historical render failed");
            return;
        }
        let defaults = objc2_foundation::NSUserDefaults::standardUserDefaults();
        let languages = defaults
            .stringArrayForKey(&objc2_foundation::NSString::from_str("AppleLanguages"))
            .map(|values| {
                values
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
            });
        eprintln!("reference AppleLanguages: {languages:?}");
        assert_eq!(languages.as_deref(), Some(expected.as_slice()));
        let Ok(mut renderer) = Renderer::new_headless() else {
            eprintln!("UNVERIFIED: no GPU adapter for content regression");
            return;
        };
        fn rgba(bytes: Vec<u8>) -> (u32, Vec<u8>) {
            let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
                .read_info()
                .unwrap();
            let mut output = vec![0; reader.output_buffer_size()];
            let frame = reader.next_frame(&mut output).unwrap();
            assert_eq!(frame.color_type, png::ColorType::Rgba);
            output.truncate(frame.buffer_size());
            (frame.width, output)
        }
        let mut text = TextEngine::new();
        text.set_locale(super::super::locale::Locale::Reference);
        let mut fixture = StaticText::new(None);
        let historical = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../design/reviews/proposal1/screenshots");
        for page in [Page::Primitives, Page::Tokens, Page::StaticText] {
            for theme in Theme::ALL {
                for scale in SCALES {
                    let name = file_name(page, theme, scale);
                    let (width, before) = rgba(fs::read(historical.join(&name)).unwrap());
                    text.set_scale(scale as f32);
                    text.begin_frame();
                    let mut original_scene = Scene::new();
                    compose(
                        page,
                        None,
                        &mut PageCtx {
                            theme,
                            text: &mut text,
                            static_text: &mut fixture,
                        },
                        &mut original_scene,
                    );
                    let mut legacy_scene = Scene::new();
                    let colors = tekes_ui_tokens::ColorRole::ALL.map(|role| role.resolve(theme));
                    let current_radius = tekes_ui_tokens::CornerRadius::concentric(
                        tekes_ui_tokens::CornerRadius::Row.value(),
                        tekes_ui_tokens::Spacing::Tight.value(),
                    );
                    let legacy_radius = tekes_ui_tokens::CornerRadius::Badge.value();
                    assert_eq!(current_radius, 2.0, "corrected concentric radius");
                    assert_eq!(legacy_radius, 4.0, "historical badge-floor radius");
                    let mut concentric_swatches = 0;
                    let mut concentric_swatch_quads = Vec::new();
                    for primitive in original_scene.primitives() {
                        match primitive {
                            tekes_ui_scene::Primitive::Quad(quad)
                                if quad.rect.size.width == 80.0
                                    && quad.rect.size.height == 48.0
                                    && quad.border_width > 0.0
                                    && quad.corner_radii
                                        == tekes_ui_scene::CornerRadii::uniform(current_radius)
                                    && colors.contains(&quad.fill) =>
                            {
                                concentric_swatches += 1;
                                concentric_swatch_quads.push(*quad);
                                legacy_scene.push_quad((*quad).with_corner_radius(legacy_radius));
                            }
                            other => legacy_scene.push(other.clone()),
                        }
                    }
                    if page == Page::Tokens {
                        assert_eq!(
                            concentric_swatches,
                            tekes_ui_tokens::ColorRole::ALL.len(),
                            "only the complete set of token swatch inner quads is exempt"
                        );
                    } else {
                        assert_eq!(concentric_swatches, 0, "non-token page swatch match");
                    }
                    let size = (FRAME_W as u32 * scale, FRAME_H as u32 * scale);
                    let after = renderer
                        .render_to_rgba8(
                            &original_scene,
                            text.atlas_mut(),
                            size,
                            scale as f32,
                            Gallery::background(theme),
                        )
                        .unwrap();
                    let legacy = renderer
                        .render_to_rgba8(
                            &legacy_scene,
                            text.atlas_mut(),
                            size,
                            scale as f32,
                            Gallery::background(theme),
                        )
                        .unwrap();
                    let new_width = size.0;
                    assert_eq!(width, new_width);
                    assert_eq!(before.len(), after.len(), "{name} image dimensions");
                    assert_eq!(before.len(), legacy.len(), "{name} legacy image dimensions");
                    // Existing goldens stay unchanged. The sole intentional pixel change
                    // is the deliberate concentric-radius correction on the 27 Tokens swatches,
                    // plus overlapping inner/outer AA on hairline borders. The legacy render
                    // changes only those 27 inner radii back to the historical badge floor.
                    // A radius-only delta is exact only where golden == legacy. Any residual
                    // historical AA variance still has to satisfy the original ring-only,
                    // 32-channel bound below.
                    let in_overlap = |scene: &Scene, x: f32, y: f32| {
                        scene.primitives().iter().any(|primitive| {
                            let tekes_ui_scene::Primitive::Quad(q) = primitive else {
                                return false;
                            };
                            if q.border_width <= 0.0 {
                                return false;
                            }
                            let scale = scale as f32;
                            let px = x - q.rect.center().x * scale;
                            let py = y - q.rect.center().y * scale;
                            let radius = if py > 0.0 {
                                if px > 0.0 {
                                    q.corner_radii.bottom_right
                                } else {
                                    q.corner_radii.bottom_left
                                }
                            } else if px > 0.0 {
                                q.corner_radii.top_right
                            } else {
                                q.corner_radii.top_left
                            } * scale;
                            let radius = radius.clamp(
                                0.0,
                                q.rect.size.width.min(q.rect.size.height) * scale * 0.5,
                            );
                            let distance = |inset: f32, r: f32| {
                                let dx = px.abs() - (q.rect.size.width * scale * 0.5 - inset) + r;
                                let dy = py.abs() - (q.rect.size.height * scale * 0.5 - inset) + r;
                                dx.max(dy).min(0.0) + dx.max(0.0).hypot(dy.max(0.0)) - r
                            };
                            let outer = (0.5 - distance(0.0, radius)).clamp(0.0, 1.0);
                            let inner = (0.5
                                - distance(
                                    q.border_width * scale,
                                    (radius - q.border_width * scale).max(0.0),
                                ))
                            .clamp(0.0, 1.0);
                            outer > 0.0 && outer < 1.0 && inner > 0.0 && inner < 1.0
                        })
                    };
                    let ring_coverage = |q: &tekes_ui_scene::Quad, radius: f32, x: f32, y: f32| {
                        let scale = scale as f32;
                        let px = x - q.rect.center().x * scale;
                        let py = y - q.rect.center().y * scale;
                        let radius = (radius * scale)
                            .clamp(0.0, q.rect.size.width.min(q.rect.size.height) * scale * 0.5);
                        let distance = |inset: f32, r: f32| {
                            let dx = px.abs() - (q.rect.size.width * scale * 0.5 - inset) + r;
                            let dy = py.abs() - (q.rect.size.height * scale * 0.5 - inset) + r;
                            dx.max(dy).min(0.0) + dx.max(0.0).hypot(dy.max(0.0)) - r
                        };
                        let outer = (0.5 - distance(0.0, radius)).clamp(0.0, 1.0);
                        let inner = (0.5
                            - distance(
                                q.border_width * scale,
                                (radius - q.border_width * scale).max(0.0),
                            ))
                        .clamp(0.0, 1.0);
                        (outer, inner)
                    };
                    let changed_swatch_coverage = |x: f32, y: f32| {
                        concentric_swatch_quads.iter().find_map(|quad| {
                            let current = ring_coverage(quad, current_radius, x, y);
                            let legacy = ring_coverage(quad, legacy_radius, x, y);
                            (current.0.to_bits() != legacy.0.to_bits()
                                || current.1.to_bits() != legacy.1.to_bits())
                            .then_some((quad.rect, current, legacy))
                        })
                    };
                    let mut corrected = 0;
                    let mut concentric_radius_pixels = 0;
                    let mut concentric_radius_aa_pixels = 0;
                    let mut concentric_quantized_aa_pixels = 0;
                    // New directory entries are intentional; all other content remains exact.
                    let start = super::super::layout::CATALOG_W as usize * scale as usize * 4;
                    let stride = width as usize * 4;
                    for (row, (a, b)) in before
                        .chunks_exact(stride)
                        .zip(after.chunks_exact(stride))
                        .enumerate()
                    {
                        for (column, (old, new)) in a
                            .chunks_exact(4)
                            .zip(b.chunks_exact(4))
                            .enumerate()
                            .skip(start / 4)
                        {
                            let offset = row * stride + column * 4;
                            let prior = &before[offset..offset + 4];
                            let historic = &legacy[offset..offset + 4];
                            if old != new {
                                if new != historic {
                                    if old == historic {
                                        concentric_radius_pixels += 1;
                                    } else {
                                        assert!(
                                            in_overlap(
                                                &legacy_scene,
                                                column as f32 + 0.5,
                                                row as f32 + 0.5,
                                            ),
                                            "{name} radius/AA pixel outside legacy overlap ({column},{row})"
                                        );
                                        assert!(
                                            prior
                                                .iter()
                                                .zip(historic)
                                                .all(|(a, b)| a.abs_diff(*b) <= 32),
                                            "{name} historical-to-legacy AA delta exceeds 32"
                                        );
                                        concentric_radius_aa_pixels += 1;
                                    }
                                } else {
                                    assert!(
                                        prior.iter().zip(new).all(|(a, b)| a.abs_diff(*b) <= 32),
                                        "{name} excessive AA correction"
                                    );
                                    let x = column as f32 + 0.5;
                                    let y = row as f32 + 0.5;
                                    if in_overlap(&original_scene, x, y) {
                                        corrected += 1;
                                    } else {
                                        let legacy_overlap = in_overlap(&legacy_scene, x, y);
                                        let coverage = changed_swatch_coverage(x, y);
                                        assert!(
                                            legacy_overlap && coverage.is_some(),
                                            "{name} unexpected pixel ({column},{row}) {old:?} -> {new:?}; legacy_overlap={legacy_overlap}, changed_swatch_coverage={coverage:?}"
                                        );
                                        if column == 819 && row == 92 {
                                            let (frame, current_cov, legacy_cov) =
                                                coverage.unwrap();
                                            eprintln!(
                                                "{name} pixel (819,92) matched swatch {frame:?} SDF: outer={:?}->{:?}, inner={:?}->{:?}, historical AA delta={:?}",
                                                current_cov.0,
                                                legacy_cov.0,
                                                current_cov.1,
                                                legacy_cov.1,
                                                prior
                                                    .iter()
                                                    .zip(historic)
                                                    .map(|(a, b)| a.abs_diff(*b))
                                                    .collect::<Vec<_>>()
                                            );
                                        }
                                        concentric_quantized_aa_pixels += 1;
                                    }
                                }
                            }
                        }
                    }
                    assert!(
                        corrected < 2000,
                        "{name}: correction affects too many pixels: {corrected}"
                    );
                    eprintln!(
                        "{name}: {concentric_radius_pixels} exact concentric-radius pixels, {concentric_radius_aa_pixels} radius/legacy-AA pixels, {concentric_quantized_aa_pixels} quantized-radius/legacy-AA pixels, {corrected} current-AA overlap pixels; other original content exact"
                    );
                }
            }
        }
    }

    #[test]
    fn png_has_expected_size() {
        let Ok(mut renderer) = Renderer::new_headless() else {
            return;
        };
        let bytes = render_png(
            &mut renderer,
            &mut TextEngine::new(),
            &mut StaticText::new(None),
            Page::Tokens,
            Theme::LIGHT,
            1,
        )
        .unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let reader = decoder.read_info().unwrap();
        let info = reader.info();
        assert_eq!((info.width, info.height), (FRAME_W as u32, FRAME_H as u32));
    }
}
