#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;

use tekes_ui_controls::composition::*;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    extended::{Axis, SplitView},
    layout_view::SplitViewView,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal, surface_palette::surface_names};

const ROOT: ComponentId = ComponentId(103_001);
const LEADING: ComponentId = ComponentId(103_002);
const TRAILING: ComponentId = ComponentId(103_003);
const DIVIDER: ComponentId = ComponentId(103_004);
const BUDGET: usize = 64 * 1024 * 1024;
const LEADING_TEXT: &str = "Leading editor pane";
const TRAILING_TEXT: &str = "Trailing reader pane";
const WIDTH: f32 = 160.0;
const HEIGHT: f32 = 96.0;

#[derive(Clone, Copy)]
struct DrawText {
    text: &'static str,
    frame: Rect,
}
struct RecordedText {
    inner: coretext::Text,
    draws: Vec<DrawText>,
}
impl ButtonTextRenderer for RecordedText {
    fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
        self.inner.measure(text, role, max_width)
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.inner.resolved_font_size(role)
    }
    fn backing_scale(&self) -> f32 {
        self.inner.backing
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        frame: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        self.inner.draw(scene, text, role, frame, align, ink);
        let known = match text {
            LEADING_TEXT => Some(LEADING_TEXT),
            TRAILING_TEXT => Some(TRAILING_TEXT),
            _ => None,
        };
        if let Some(text) = known {
            self.draws.push(DrawText { text, frame });
        }
    }
}

fn palette_runs() -> Vec<(String, Palette)> {
    let run = std::env::var("SPLITTER_THEME_RUN_KIND")
        .expect("locked runner must identify default or external Theme build");
    match run.as_str() {
        "default" => {
            assert!(std::env::var_os("TEKESUI_DESIGN_ARTIFACT").is_none());
            assert!(std::env::var_os("TEKESUI_THEME_LOCK").is_none());
            Theme::ALL
                .into_iter()
                .enumerate()
                .map(|(mode, theme)| (format!("default-mode{mode}"), Palette::from(theme)))
                .collect()
        }
        "external" => {
            let artifact = absolute_env_path("TEKESUI_DESIGN_ARTIFACT");
            assert!(
                artifact.is_dir(),
                "external Theme artifact must be a directory"
            );
            let lock = absolute_env_path("TEKESUI_THEME_LOCK");
            assert!(lock.is_file(), "external Theme source lock must exist");
            let instances_path = artifact.join("design/theme/generated/instances.json");
            assert!(
                instances_path.is_file(),
                "generated Theme instances must exist"
            );
            let instances: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&instances_path).unwrap()).unwrap();
            let ids: Vec<_> = instances["instances"]
                .as_array()
                .expect("generated external instances array")
                .iter()
                .filter_map(|i| i["id"].as_str())
                .collect();
            assert!(ids.contains(&"external-default"));
            assert!(ids.contains(&"external-inputs"));
            let mut result = Vec::new();
            for (mode, theme) in Theme::ALL.into_iter().enumerate() {
                for id in ["external-default", "external-inputs"] {
                    result.push((
                        format!("{id}-mode{mode}"),
                        Palette::for_theme_definition(theme, id)
                            .expect("compiled palette must match the checked artifact IDs"),
                    ));
                }
            }
            result
        }
        other => panic!("unknown SPLITTER_THEME_RUN_KIND={other}"),
    }
}

fn absolute_env_path(key: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(std::env::var_os(key).expect(key));
    assert!(
        path.is_absolute(),
        "{key} must identify an absolute locked input"
    );
    path.canonicalize().expect("Theme input must exist")
}

fn host(surface: &str, config: SplitViewConfig) -> ContentHost {
    let leading = Content::new(LEADING, Button::new(LEADING_TEXT));
    let trailing = Content::new(TRAILING, Button::new(TRAILING_TEXT));
    let split = SplitViewContent::try_new(SplitViewConstruction {
        config,
        slots: SplitViewSlotIds {
            leading: ComponentId(103_005),
            trailing: ComponentId(103_006),
            divider: DIVIDER,
        },
        leading,
        trailing,
        budget: BUDGET,
    })
    .unwrap();
    ContentHost::try_new_prepared(
        Content::new(ROOT, split).with_surface(surface).unwrap(),
        BUDGET,
    )
    .unwrap()
}

fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let x0 = a.min_x().max(b.min_x());
    let y0 = a.min_y().max(b.min_y());
    let x1 = a.max_x().min(b.max_x());
    let y1 = a.max_y().min(b.max_y());
    (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
}

fn pane_interior(pane: Rect, divider: Rect, clip: Rect, leading: bool) -> Option<Rect> {
    let clipped = intersect(pane, clip)?;
    let no_divider = if leading {
        Rect::new(
            clipped.min_x(),
            clipped.min_y(),
            (clipped.max_x().min(divider.min_x()) - clipped.min_x()).max(0.0),
            clipped.size.height,
        )
    } else {
        let x = clipped.min_x().max(divider.max_x());
        Rect::new(
            x,
            clipped.min_y(),
            (clipped.max_x() - x).max(0.0),
            clipped.size.height,
        )
    };
    intersect(no_divider, clip)
}

fn changed_in(
    pixels: &[u8],
    empty: &[u8],
    size: (u32, u32),
    scale: f32,
    area: Option<Rect>,
) -> usize {
    let Some(area) = area else { return 0 };
    pixels
        .chunks_exact(4)
        .zip(empty.chunks_exact(4))
        .enumerate()
        .filter(|(i, (pixel, ground))| {
            if pixel == ground {
                return false;
            }
            let x = (*i as u32 % size.0) as f32 / scale;
            let y = (*i as u32 / size.0) as f32 / scale;
            x + 1.0 / scale > area.min_x()
                && x < area.max_x()
                && y + 1.0 / scale > area.min_y()
                && y < area.max_y()
        })
        .count()
}

fn outside_clip(pixels: &[u8], empty: &[u8], size: (u32, u32), scale: f32, clip: Rect) -> usize {
    let left = (clip.min_x() * scale).ceil().max(0.0) as u32;
    let top = (clip.min_y() * scale).ceil().max(0.0) as u32;
    let right = (clip.max_x() * scale).floor().max(0.0) as u32;
    let bottom = (clip.max_y() * scale).floor().max(0.0) as u32;
    pixels
        .chunks_exact(4)
        .zip(empty.chunks_exact(4))
        .enumerate()
        .filter(|(i, (pixel, ground))| {
            if pixel == ground {
                return false;
            }
            let px = *i as u32 % size.0;
            let py = *i as u32 / size.0;
            px < left || px >= right || py < top || py >= bottom
        })
        .count()
}

fn divider_pixels_equal(
    actual: &[u8],
    original_view: &[u8],
    size: (u32, u32),
    scale: f32,
    divider: Rect,
) -> bool {
    (0..size.1).all(|py| {
        (0..size.0).all(|px| {
            let x = px as f32 / scale;
            let y = py as f32 / scale;
            if x + 1.0 / scale <= divider.min_x()
                || x >= divider.max_x()
                || y + 1.0 / scale <= divider.min_y()
                || y >= divider.max_y()
            {
                return true;
            }
            let offset = ((py * size.0 + px) * 4) as usize;
            actual[offset..offset + 4] == original_view[offset..offset + 4]
        })
    })
}

struct CaseGeometry {
    frame: Rect,
    clip: Rect,
    config: SplitViewConfig,
    update_ratio: bool,
}
fn render_case(
    gpu: &mut tekes_ui_render::Renderer,
    output: &std::path::Path,
    name: &str,
    surface: &str,
    palette: Palette,
    backing: f32,
    geometry: CaseGeometry,
) -> serde_json::Value {
    let CaseGeometry {
        frame,
        clip,
        config,
        update_ratio,
    } = geometry;
    let size = ((WIDTH * backing) as u32, (HEIGHT * backing) as u32);
    let clear = palette.color("surface-1").unwrap();
    let mut text = RecordedText {
        inner: coretext::Text::new(1.0),
        draws: Vec::new(),
    };
    text.inner.backing = backing;
    let mut host = host(surface, config);
    let mut scene = Scene::new();
    host.render_checked(&mut scene, &mut text, frame, clip, palette)
        .unwrap();
    let layout = host
        .get(ROOT)
        .unwrap()
        .component::<SplitViewContent>()
        .unwrap()
        .layout()
        .unwrap();
    let mut shared_view = SplitViewView::new(Axis::Horizontal);
    let mut view_model = SplitView::new(config.ratio);
    view_model.divider = config.divider;
    view_model.min_pane = config.min_pane;
    view_model.collapsed = config.collapsed;
    let mut view_scene = Scene::new();
    let view_layout = shared_view.render_at_scale(
        &mut view_scene,
        &mut view_model,
        frame,
        Some(clip),
        palette,
        backing,
    );
    assert_eq!(layout, view_layout, "original View geometry {name}");
    let empty = gpu
        .render_to_rgba8(&Scene::new(), &mut text.inner.atlas, size, backing, clear)
        .unwrap();
    let pixels = gpu
        .render_to_rgba8(&scene, &mut text.inner.atlas, size, backing, clear)
        .unwrap();
    let view_pixels = gpu
        .render_to_rgba8(&view_scene, &mut text.inner.atlas, size, backing, clear)
        .unwrap();
    let lead = pane_interior(layout.leading.frame(), layout.divider.frame(), clip, true);
    let trail = pane_interior(layout.trailing.frame(), layout.divider.frame(), clip, false);
    // Quiet Button may exactly match the original opaque clear color. Keep
    // that rendering for glyph/clip/View assertions, and measure background
    // coverage from the identical Scene over transparent clear.
    let transparent = Rgba {
        r: 0.,
        g: 0.,
        b: 0.,
        a: 0.,
    };
    let coverage_empty = gpu
        .render_to_rgba8(
            &Scene::new(),
            &mut text.inner.atlas,
            size,
            backing,
            transparent,
        )
        .unwrap();
    let coverage_pixels = gpu
        .render_to_rgba8(&scene, &mut text.inner.atlas, size, backing, transparent)
        .unwrap();
    std::fs::write(
        output.join(format!("{name}.coverage.rgba8")),
        &coverage_pixels,
    )
    .unwrap();
    let lead_ink = changed_in(&coverage_pixels, &coverage_empty, size, backing, lead);
    let trail_ink = changed_in(&coverage_pixels, &coverage_empty, size, backing, trail);
    assert_eq!(
        outside_clip(&pixels, &empty, size, backing, clip),
        0,
        "host parent clip {name}"
    );
    assert_eq!(
        outside_clip(&view_pixels, &empty, size, backing, clip),
        0,
        "View parent clip {name}"
    );

    let lead_text = text
        .draws
        .iter()
        .find(|d| d.text == LEADING_TEXT)
        .map(|d| d.frame);
    let trail_text = text
        .draws
        .iter()
        .find(|d| d.text == TRAILING_TEXT)
        .map(|d| d.frame);
    let lead_text_pixels = changed_in(
        &pixels,
        &empty,
        size,
        backing,
        lead_text.and_then(|r| intersect(r, lead.unwrap_or_default())),
    );
    let trail_text_pixels = changed_in(
        &pixels,
        &empty,
        size,
        backing,
        trail_text.and_then(|r| intersect(r, trail.unwrap_or_default())),
    );
    let lead_visible = lead.is_some_and(|r| r.size.width > 0.0 && r.size.height > 0.0);
    let trail_visible = trail.is_some_and(|r| r.size.width > 0.0 && r.size.height > 0.0);
    if (lead_visible && !(lead_ink > lead_text_pixels && lead_text_pixels > 0))
        || (trail_visible && !(trail_ink > trail_text_pixels && trail_text_pixels > 0))
    {
        eprintln!(
            "PIXEL_FAILURE {name} size={size:?} lead={lead:?} trail={trail:?} lead_text={lead_text:?} trail_text={trail_text:?} ink={lead_ink}/{trail_ink} text={lead_text_pixels}/{trail_text_pixels}"
        );
        std::fs::write(output.join(format!("{name}.failed.rgba8")), &pixels).unwrap();
        std::fs::write(output.join(format!("{name}.failed.empty.rgba8")), &empty).unwrap();
        std::fs::write(
            output.join(format!("{name}.failed.scene.txt")),
            format!("{scene:?}"),
        )
        .unwrap();
    }
    if lead_visible {
        assert!(
            lead_ink > lead_text_pixels && lead_text_pixels > 0,
            "leading Button background and CoreText glyphs must paint {name}"
        );
    } else {
        assert_eq!(
            lead_ink, 0,
            "hidden leading pane has no ghost pixels {name}"
        );
    }
    if trail_visible {
        assert!(
            trail_ink > trail_text_pixels && trail_text_pixels > 0,
            "trailing Button background and CoreText glyphs must paint {name}"
        );
    } else {
        assert_eq!(
            trail_ink, 0,
            "hidden trailing pane has no ghost pixels {name}"
        );
    }
    let view_divider_ink = changed_in(
        &view_pixels,
        &empty,
        size,
        backing,
        Some(layout.divider.frame()),
    );
    if layout
        .divider
        .visible_frame()
        .is_some_and(|r| r.size.width > 0.0 && r.size.height > 0.0)
    {
        assert!(
            view_divider_ink > 0,
            "original View must paint divider pixels {name}"
        );
        assert!(
            divider_pixels_equal(&pixels, &view_pixels, size, backing, layout.divider.frame()),
            "ContentHost divider pixels match original View within divider ROI: {name}"
        );
    }

    let mut after_geometry = None;
    if update_ratio {
        text.draws.clear();
        let result =
            host.accessibility_action(DIVIDER, None, AccessibilityAction::SetValue("0.625".into()));
        assert!(
            result.consumed,
            "existing ContentHost ratio action route {name}"
        );
        let mut after_scene = Scene::new();
        host.render_checked(&mut after_scene, &mut text, frame, clip, palette)
            .unwrap();
        let after_layout = host
            .get(ROOT)
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .layout()
            .unwrap();
        assert_ne!(
            layout.leading.frame(),
            after_layout.leading.frame(),
            "ratio moves original geometry {name}"
        );
        let after_pixels = gpu
            .render_to_rgba8(&after_scene, &mut text.inner.atlas, size, backing, clear)
            .unwrap();
        assert_ne!(
            pixels, after_pixels,
            "ratio update changes real rendered pane pixels {name}"
        );
        assert_eq!(
            outside_clip(&after_pixels, &empty, size, backing, clip),
            0,
            "updated host parent clip {name}"
        );
        let after_lead = pane_interior(
            after_layout.leading.frame(),
            after_layout.divider.frame(),
            clip,
            true,
        );
        let after_trail = pane_interior(
            after_layout.trailing.frame(),
            after_layout.divider.frame(),
            clip,
            false,
        );
        let after_coverage = gpu
            .render_to_rgba8(
                &after_scene,
                &mut text.inner.atlas,
                size,
                backing,
                transparent,
            )
            .unwrap();
        std::fs::write(
            output.join(format!("{name}.after.coverage.rgba8")),
            &after_coverage,
        )
        .unwrap();
        let after_lead_ink =
            changed_in(&after_coverage, &coverage_empty, size, backing, after_lead);
        let after_trail_ink =
            changed_in(&after_coverage, &coverage_empty, size, backing, after_trail);
        let after_lead_text = text
            .draws
            .iter()
            .find(|d| d.text == LEADING_TEXT)
            .map(|d| d.frame);
        let after_trail_text = text
            .draws
            .iter()
            .find(|d| d.text == TRAILING_TEXT)
            .map(|d| d.frame);
        let after_lead_glyph = changed_in(
            &after_pixels,
            &empty,
            size,
            backing,
            after_lead_text.and_then(|r| intersect(r, after_lead.unwrap_or_default())),
        );
        let after_trail_glyph = changed_in(
            &after_pixels,
            &empty,
            size,
            backing,
            after_trail_text.and_then(|r| intersect(r, after_trail.unwrap_or_default())),
        );
        assert!(
            after_lead_ink > after_lead_glyph && after_lead_glyph > 0,
            "updated leading Button background and glyphs {name}"
        );
        assert!(
            after_trail_ink > after_trail_glyph && after_trail_glyph > 0,
            "updated trailing Button background and glyphs {name}"
        );
        let mut after_view = SplitViewView::new(Axis::Horizontal);
        let mut after_model = SplitView::new(0.625);
        after_model.divider = config.divider;
        after_model.min_pane = config.min_pane;
        after_model.collapsed = config.collapsed;
        let mut after_view_scene = Scene::new();
        let after_view_layout = after_view.render_at_scale(
            &mut after_view_scene,
            &mut after_model,
            frame,
            Some(clip),
            palette,
            backing,
        );
        assert_eq!(
            after_layout, after_view_layout,
            "updated original View geometry {name}"
        );
        let after_view_pixels = gpu
            .render_to_rgba8(
                &after_view_scene,
                &mut text.inner.atlas,
                size,
                backing,
                clear,
            )
            .unwrap();
        let after_view_divider_ink = changed_in(
            &after_view_pixels,
            &empty,
            size,
            backing,
            Some(after_layout.divider.frame()),
        );
        assert!(
            after_view_divider_ink > 0,
            "updated original View divider paints {name}"
        );
        assert!(
            divider_pixels_equal(
                &after_pixels,
                &after_view_pixels,
                size,
                backing,
                after_layout.divider.frame(),
            ),
            "updated divider ROI matches original View {name}"
        );
        assert_eq!(
            outside_clip(&after_view_pixels, &empty, size, backing, clip),
            0,
            "updated original View clip {name}"
        );
        after_geometry = Some(serde_json::json!({
            "leading": rect_json(after_layout.leading.frame()),
            "divider": rect_json(after_layout.divider.frame()),
            "trailing": rect_json(after_layout.trailing.frame()),
        }));
        let key = format!("{name}.after");
        std::fs::write(output.join(format!("{key}.rgba8")), &after_pixels).unwrap();
        std::fs::write(
            output.join(format!("{key}.original-view.rgba8")),
            &after_view_pixels,
        )
        .unwrap();
        std::fs::write(
            output.join(format!("{key}.scene-primitives.txt")),
            format!("{:#?}", after_scene.primitives()),
        )
        .unwrap();
        std::fs::write(
            output.join(format!("{key}.original-view-primitives.txt")),
            format!("{:#?}", after_view_scene.primitives()),
        )
        .unwrap();
    }
    let key = name.to_owned();
    std::fs::write(output.join(format!("{key}.rgba8")), &pixels).unwrap();
    std::fs::write(
        output.join(format!("{key}.original-view.rgba8")),
        &view_pixels,
    )
    .unwrap();
    std::fs::write(
        output.join(format!("{key}.scene-primitives.txt")),
        format!("{:#?}", scene.primitives()),
    )
    .unwrap();
    std::fs::write(
        output.join(format!("{key}.original-view-primitives.txt")),
        format!("{:#?}", view_scene.primitives()),
    )
    .unwrap();
    serde_json::json!({
        "case": key,
        "surface": surface,
        "backing": backing,
        "frame": rect_json(frame),
        "clip": rect_json(clip),
        "leading": rect_json(layout.leading.frame()),
        "divider": rect_json(layout.divider.frame()),
        "trailing": rect_json(layout.trailing.frame()),
        "background_measurement": "same full Scene over transparent clear; glyph/clip/View comparisons retain original opaque clear",
        "leading_interior_ink_pixels": lead_ink,
        "trailing_interior_ink_pixels": trail_ink,
        "leading_glyph_pixels": lead_text_pixels,
        "trailing_glyph_pixels": trail_text_pixels,
        "outside_clip_pixels": 0,
        "divider_roi_matches_original_view": true,
        "ratio_update_geometry": after_geometry,
        "primitive_and_clip_order_saved": true,
    })
}

fn rect_json(rect: Rect) -> [f32; 4] {
    [
        rect.min_x(),
        rect.min_y(),
        rect.size.width,
        rect.size.height,
    ]
}

#[test]
#[ignore = "explicit headless Metal lane; run under original build lock with fresh evidence path"]
fn real_split_content_host_theme_surface_clip_and_view_matrix() {
    let output = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_SPLITTER_GPU_OUT").expect("fresh evidence directory"),
    );
    std::fs::create_dir(&output).expect("fresh non-existing output directory");
    let mut gpu = tekes_ui_render::Renderer::new_headless().expect("actual headless Metal");
    let mut cases = Vec::new();
    let themes = palette_runs();
    let external = std::env::var("SPLITTER_THEME_RUN_KIND").unwrap() == "external";
    for (theme_name, palette) in themes {
        for surface in surface_names() {
            for backing in [1.0_f32, 2.0] {
                let label = format!("normal-{theme_name}-{surface}-{backing}x");
                cases.push(render_case(
                    &mut gpu,
                    &output,
                    &label,
                    surface,
                    palette,
                    backing,
                    CaseGeometry {
                        frame: Rect::new(8.0, 8.0, 144.0, 72.0),
                        clip: Rect::new(20.25, 8.0, 124.0, 72.0),
                        config: SplitViewConfig::default(),
                        update_ratio: true,
                    },
                ));
            }
        }
    }

    // Fixed geometry probes, separate from the full theme/surface matrix.
    let representative = if external {
        let theme = Theme::LIGHT;
        Palette::for_theme_definition(theme, "external-default").unwrap()
    } else {
        Palette::from(Theme::LIGHT)
    };
    for backing in [1.0_f32, 2.0] {
        cases.push(render_case(
            &mut gpu,
            &output,
            &format!("fractional-canvas-{backing}x"),
            "canvas",
            representative,
            backing,
            CaseGeometry {
                frame: Rect::new(8.25, 8.5, 143.5, 72.25),
                clip: Rect::new(20.25, 8.0, 124.0, 72.0),
                config: SplitViewConfig::default(),
                update_ratio: false,
            },
        ));
        let collapsed = SplitViewConfig {
            min_pane: 20.0,
            collapsed: Some(tekes_ui_controls::extended::Pane::Leading),
            ..SplitViewConfig::default()
        };
        cases.push(render_case(
            &mut gpu,
            &output,
            &format!("narrow-one-pane-canvas-{backing}x"),
            "canvas",
            representative,
            backing,
            CaseGeometry {
                frame: Rect::new(8.0, 8.0, 30.0, 72.0),
                clip: Rect::new(0.0, 0.0, WIDTH, HEIGHT),
                config: collapsed,
                update_ratio: false,
            },
        ));
        cases.push(render_case(
            &mut gpu,
            &output,
            &format!("zero-allocation-canvas-{backing}x"),
            "canvas",
            representative,
            backing,
            CaseGeometry {
                frame: Rect::new(8.0, 8.0, 0.0, 0.0),
                clip: Rect::new(0.0, 0.0, WIDTH, HEIGHT),
                config: SplitViewConfig::default(),
                update_ratio: false,
            },
        ));
    }
    let expected = if external { 118 } else { 62 };
    assert_eq!(cases.len(), expected, "finite frozen case count");
    let receipt = serde_json::json!({
        "issue": 103,
        "lane": "explicit ignored real GPU consumer; never in default test run",
        "theme_run_kind": if external { "external" } else { "default" },
        "external_artifact_root": std::env::var_os("TEKESUI_DESIGN_ARTIFACT").map(|v| v.to_string_lossy().to_string()),
        "external_theme_lock": std::env::var_os("TEKESUI_THEME_LOCK").map(|v| v.to_string_lossy().to_string()),
        "external_instance_ids_checked": external,
        "theme_modes": 4,
        "surfaces_in_normal_matrix": 7,
        "normal_matrix_backings": [1,2],
        "fixed_geometry_probes": ["fractional", "narrow-one-pane", "zero-allocation"],
        "expected_case_count": expected,
        "actual_case_count": cases.len(),
        "content_surface_inherited_from_root": true,
        "draw_records_are_only_pixel_rois_not_visibility_proof": true,
        "physical_window": false,
        "voiceover": false,
        "cases": cases,
    });
    std::fs::write(
        output.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
}
