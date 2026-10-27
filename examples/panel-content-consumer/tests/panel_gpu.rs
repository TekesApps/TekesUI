//! Actual Metal/CoreText consumer and a one-shape coverage oracle over the exact arranged glyphs.
//! Current-main reception of the preserved R19 consumer; GPU output stays in this SSD intake epoch.
use tekes_ui_controls::{
    ButtonTextRenderer, Insets, ScrollArea,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_scene::{
    AtlasGlyph, FontKey, GlyphKey, GlyphRun, ImageId, ImageQuad, Point, PositionedGlyph, Primitive,
    Quad, RasterFormat, RasterGlyph, Rect, Scene, ShadowQuad, Size, TransformedImageQuad,
};
use tekes_ui_tokens::{Rgba, Theme};
#[path = "support/coretext.rs"]
mod coretext;
use coretext::Text;

#[derive(Clone, Copy, Debug)]
enum Pipeline {
    Quad,
    Image,
    TransformedImage,
    Glyph,
    Shadow,
}
impl Pipeline {
    const ALL: [Self; 5] = [
        Self::Quad,
        Self::Image,
        Self::TransformedImage,
        Self::Glyph,
        Self::Shadow,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::Quad => "quad",
            Self::Image => "image",
            Self::TransformedImage => "transformed-image",
            Self::Glyph => "coretext-glyph",
            Self::Shadow => "shadow",
        }
    }
}

/// A QA leaf that drives one actual pipeline using the frame assigned by the original Panel/Stack.
#[derive(Debug)]
struct PipelineInk {
    pipeline: Pipeline,
    image: ImageId,
}
impl Component for PipelineInk {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(72., 72.))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        frame: Rect,
        _: Rect,
        _: ContentTheme,
    ) {
        let red = Rgba::rgb8(230, 51, 13);
        let image = ImageQuad {
            rect: frame,
            image: self.image,
            opacity: 1.,
        };
        let primitive = match self.pipeline {
            Pipeline::Quad => Primitive::Quad(Quad::new(frame, red)),
            Pipeline::Image => Primitive::Image(image),
            Pipeline::TransformedImage => {
                Primitive::TransformedImage(TransformedImageQuad::new(image, 0.37))
            }
            Pipeline::Glyph => return, // actual CoreText TextSurface below owns this pipeline
            Pipeline::Shadow => Primitive::Shadow(ShadowQuad::new(frame, red).with_blur_radius(2.)),
        };
        scene.push(primitive);
    }
}

fn out_dir() -> std::path::PathBuf {
    let root = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_PANEL82_GPU_ARTIFACT_DIR")
            .expect("set fresh main7ab8e32 Issue #82 SSD output directory"),
    );
    assert!(root.starts_with(
        "/Volumes/HP P900/DEV/TekesUI/artifacts/issue-82/"
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn build_host(pipeline: Pipeline, nested: bool, image: ImageId) -> ContentHost {
    let body = TextFieldContent::read_only_content(
        "正文",
        &"MMMMmmWW中文 — actual CoreText crosses the rounded edge — ".repeat(36),
        tekes_ui_tokens::proposal::TextRole::UiBody,
    );
    let child = if matches!(pipeline, Pipeline::Glyph) {
        Content::new(ComponentId(8303), body)
    } else {
        Content::new(ComponentId(8302), StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            children: vec![
                Content::new(ComponentId(8301), PipelineInk { pipeline, image }),
                Content::new(ComponentId(8303), body),
            ],
            flexible_child: None,
        })
    };
    let mut panel = PanelContent::new(child).unwrap();
    panel.set_padding(Some(Insets::default())).unwrap();
    let inner = Content::new(ComponentId(8304), panel);
    let root = if nested {
        Content::new(
            ComponentId(8305),
            PanelContent::new(Content::new(
                ComponentId(8306),
                ScrollContent::new(ScrollArea::default(), inner),
            ))
            .unwrap(),
        )
    } else {
        inner
    };
    // `root` is already the original mounted Content node in both branches.
    ContentHost::new(root).unwrap()
}

fn render(host: &mut ContentHost, text: &mut Text, frame: Rect, clip: Rect, theme: Theme) -> Scene {
    let mut scene = Scene::new();
    host.render(&mut scene, text, frame, clip, theme.into())
        .unwrap();
    scene
}

/// A fresh Scene clone keeps the original immutable clip registry. All original non-glyph
/// operations and their order survive; only non-target Glyphs runs are omitted.
fn isolated_glyph_runs(
    source: &Scene,
    targets: &[usize],
    sentinel: Option<PositionedGlyph>,
) -> Scene {
    let mut scene = source.clone();
    scene.clear();
    let mut ordinal = 0usize;
    let mut found = 0usize;
    for primitive in source.primitives() {
        if let Primitive::Glyphs(run) = primitive {
            if targets.contains(&ordinal) {
                let mut selected = run.clone();
                if let Some(glyph) = sentinel {
                    selected.glyphs.push(glyph);
                }
                scene.push_glyphs(selected);
                found += 1;
            }
            ordinal += 1;
        } else {
            scene.push(primitive.clone());
        }
    }
    assert_eq!(
        found,
        targets.len(),
        "requested actual CoreText GlyphRun ordinals exist"
    );
    assert_eq!(scene.clip_shape_count(), source.clip_shape_count());
    assert_eq!(
        scene.required_clip_geometry(),
        source.required_clip_geometry()
    );
    scene
}

fn isolated_glyph_run(source: &Scene, target: usize, sentinel: Option<PositionedGlyph>) -> Scene {
    isolated_glyph_runs(source, &[target], sentinel)
}

fn insert_offscreen_transparent_sentinel(
    atlas: &mut tekes_ui_scene::GlyphAtlas,
    backing: f32,
    size_px: (u32, u32),
) -> PositionedGlyph {
    // Distinct reserved key; insert before CoreText renders any real text, so both sides use
    // exactly the same immutable atlas placement and texture contents.
    let glyph: AtlasGlyph = atlas
        .get_or_insert_with(GlyphKey::new(FontKey(u32::MAX), u16::MAX, 2000), || {
            Some(RasterGlyph {
                left: 0,
                top: 0,
                width: 1,
                height: 1,
                format: RasterFormat::Color,
                pixels: vec![0, 0, 0, 0],
            })
        })
        .expect("preinsert transparent Color-format sentinel into shared atlas");
    assert_eq!(glyph.width, 1);
    assert_eq!(glyph.height, 1);
    assert_eq!(glyph.format, RasterFormat::Color);
    let page = atlas.page_pixels(glyph.page).unwrap();
    let pixel_offset = ((glyph.y * atlas.page_size() + glyph.x) * 4) as usize;
    assert_eq!(&page[pixel_offset..pixel_offset + 4], &[0, 0, 0, 0]);

    // Keep the actual device x destination outside both the target and the R2 2^14 domain.
    // 16392 is exactly representable and divisible by the backing scales used here.
    let device_x = 16_392.0_f32.max(size_px.0 as f32 + 32.0);
    let origin = Point::new(device_x / backing, 64.0 / backing);
    let dest_x = (origin.x * backing).round() + glyph.left as f32;
    let dest_y = (origin.y * backing).round() + glyph.top as f32;
    assert!(
        dest_x > size_px.0 as f32,
        "sentinel bitmap must be wholly right of target"
    );
    assert!(
        dest_x > 16_384.0,
        "sentinel device destination must force whole-run fallback"
    );
    assert_eq!(dest_x.fract(), 0.0);
    assert_eq!(dest_y.fract(), 0.0);
    PositionedGlyph { origin, glyph }
}

fn actual_run_geometry(run: &GlyphRun, backing: f32) -> (f32, f32, f32, f32) {
    let mut x0 = f32::INFINITY;
    let mut y0 = f32::INFINITY;
    let mut x1 = f32::NEG_INFINITY;
    let mut y1 = f32::NEG_INFINITY;
    for positioned in &run.glyphs {
        let x = (positioned.origin.x * backing).round() + positioned.glyph.left as f32;
        let y = (positioned.origin.y * backing).round() + positioned.glyph.top as f32;
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x + positioned.glyph.width as f32);
        y1 = y1.max(y + positioned.glyph.height as f32);
    }
    (x0, y0, x1, y1)
}

fn glyph_run_scope_depths(scene: &Scene) -> Vec<usize> {
    let mut depth = 0usize;
    let mut result = Vec::new();
    for primitive in scene.primitives() {
        match primitive {
            Primitive::PushClip(_) | Primitive::PushShapeClip(_) => depth += 1,
            Primitive::PopClip => depth = depth.saturating_sub(1),
            Primitive::Glyphs(run) if !run.glyphs.is_empty() => result.push(depth),
            _ => {}
        }
    }
    result
}

fn build_glyph_host_with_text(body_text: &str) -> ContentHost {
    let body = TextFieldContent::read_only_content(
        "正文",
        body_text,
        tekes_ui_tokens::proposal::TextRole::UiBody,
    );
    let mut panel = PanelContent::new(Content::new(ComponentId(8403), body)).unwrap();
    panel.set_padding(Some(Insets::default())).unwrap();
    ContentHost::new(Content::new(ComponentId(8404), panel)).unwrap()
}

fn recovery_rects(scene: &Scene, backing: f32, size_px: (u32, u32)) -> [Rect; 3] {
    let mut bounds = scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Glyphs(run) if !run.glyphs.is_empty() => {
                Some(actual_run_geometry(run, backing))
            }
            _ => None,
        });
    let first = bounds
        .next()
        .expect("recovery scene retains its original GlyphRun");
    let (mut gx0, mut gy0, mut gx1, mut gy1) = first;
    for (x0, y0, x1, y1) in bounds {
        gx0 = gx0.min(x0);
        gy0 = gy0.min(y0);
        gx1 = gx1.max(x1);
        gy1 = gy1.max(y1);
    }
    let logical_width = size_px.0 as f32 / backing;
    let logical_height = size_px.1 as f32 / backing;
    let mut found: Vec<Rect> = Vec::new();
    'search: for y in (4..(logical_height as i32 - 10).max(4)).step_by(8) {
        for x in (4..(logical_width as i32 - 10).max(4)).step_by(8) {
            let candidate = Rect::new(x as f32, y as f32, 6., 6.);
            let device = [
                candidate.min_x() * backing,
                candidate.min_y() * backing,
                candidate.max_x() * backing,
                candidate.max_y() * backing,
            ];
            if device[0] < gx1 && device[2] > gx0 && device[1] < gy1 && device[3] > gy0 {
                continue;
            }
            if found.iter().any(|old| {
                candidate.min_x() < old.max_x() + 1.
                    && candidate.max_x() + 1. > old.min_x()
                    && candidate.min_y() < old.max_y() + 1.
                    && candidate.max_y() + 1. > old.min_y()
            }) {
                continue;
            }
            found.push(candidate);
            if found.len() == 3 {
                break 'search;
            }
        }
    }
    assert_eq!(
        found.len(),
        3,
        "reserve three output regions outside the selected run bitmap bounds"
    );
    [found[0], found[1], found[2]]
}

fn append_clip_recovery(
    scene: &mut Scene,
    blocked: Rect,
    quad: Rect,
    image_rect: Rect,
    image: ImageId,
) {
    scene.push_clip(Rect::new(0., 0., 0., 0.));
    scene.push_quad(Quad::new(blocked, Rgba::rgb8(240, 20, 20)));
    scene.pop_clip();
    scene.push_quad(Quad::new(quad, Rgba::rgb8(20, 80, 230)));
    scene.push_image(ImageQuad {
        rect: image_rect,
        image,
        opacity: 1.,
    });
}

#[allow(clippy::too_many_arguments)] // Keep render inputs, frame metadata, and output event archive together in this QA helper.
fn render_qa_frame(
    renderer: &mut tekes_ui_render::Renderer,
    scene: &Scene,
    atlas: &mut tekes_ui_scene::GlyphAtlas,
    size_px: (u32, u32),
    backing: f32,
    clear: Rgba,
    output: &std::path::Path,
    ordinal: usize,
    pair: &str,
    role: &str,
    glyph_count: usize,
    scene_scope_depth: usize,
    events: &mut Vec<serde_json::Value>,
) -> Vec<u8> {
    let scene_glyph_runs: Vec<_> = scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Glyphs(run) if !run.glyphs.is_empty() => {
                let (x0, y0, x1, y1) = actual_run_geometry(run, backing);
                Some(serde_json::json!({
                    "glyph_count": run.glyphs.len(),
                    "device_destination_union_recomputed_from_public_origins_bearings_and_sizes": [x0, y0, x1, y1]
                }))
            }
            _ => None,
        })
        .collect();
    println!(
        "R19_RENDER_EVENT ordinal={ordinal} pair={pair} role={role} glyph_count={glyph_count} scene_scope_depth={scene_scope_depth}"
    );
    events.push(serde_json::json!({
        "render_call_ordinal": ordinal,
        "pair": pair,
        "role": role,
        "glyph_count": glyph_count,
        "scene_scope_depth": scene_scope_depth,
        "scene_glyph_runs_recomputed_from_public_atlas_data": scene_glyph_runs,
        "target_pixels": size_px,
        "backing": backing,
        "prepared_state": "not observed by current production consumer API; historical diagnostic SDK results are separate evidence"
    }));
    let pixels = renderer
        .render_to_rgba8(scene, atlas, size_px, backing, clear)
        .unwrap();
    std::fs::write(
        output.join(format!("r19-frame-{ordinal:03}-{pair}-{role}.rgba")),
        &pixels,
    )
    .unwrap();
    pixels
}

fn pipeline_present(scene: &Scene, pipeline: Pipeline) -> bool {
    scene.primitives().iter().any(|p| match pipeline {
        Pipeline::Quad => matches!(p, Primitive::Quad(_)),
        Pipeline::Image => matches!(p, Primitive::Image(_)),
        Pipeline::TransformedImage => matches!(p, Primitive::TransformedImage(_)),
        Pipeline::Glyph => matches!(p, Primitive::Glyphs(run) if !run.glyphs.is_empty()),
        Pipeline::Shadow => matches!(p, Primitive::Shadow(_)),
    })
}

fn panel_curve(
    host: &ContentHost,
    scene: &Scene,
    panel_id: ComponentId,
) -> tekes_ui_scene::ClipShape {
    let geometry = host
        .get(panel_id)
        .unwrap()
        .component::<PanelContent>()
        .unwrap()
        .geometry()
        .expect("original Panel must publish arranged geometry");
    let shape = geometry.shape;
    assert!(
        !shape.is_rect(),
        "oracle target must be the rounded Panel curve"
    );
    assert!(
        scene.primitives().iter().any(|primitive| match primitive {
            Primitive::PushShapeClip(key) => scene.clip_shape(key).ok() == Some(shape),
            _ => false,
        }),
        "actual Scene registry must contain the exact arranged Panel shape"
    );
    shape
}

fn is_target_curve(scene: &Scene, primitive: &Primitive, curve: tekes_ui_scene::ClipShape) -> bool {
    matches!(primitive, Primitive::PushShapeClip(key) if registered_shape(scene, key) == curve)
}

fn registered_shape(scene: &Scene, key: &tekes_ui_scene::ShapeKey) -> tekes_ui_scene::ClipShape {
    scene
        .clip_shape(key)
        .expect("original Scene registry must resolve every referenced shape key")
}

fn same_scene_operation(
    left_scene: &Scene,
    left: &Primitive,
    right_scene: &Scene,
    right: &Primitive,
) -> bool {
    match (left, right) {
        (Primitive::PushShapeClip(left_key), Primitive::PushShapeClip(right_key)) => {
            registered_shape(left_scene, left_key) == registered_shape(right_scene, right_key)
        }
        (
            Primitive::KeyedQuad {
                quad: left_quad,
                outer: left_key,
            },
            Primitive::KeyedQuad {
                quad: right_quad,
                outer: right_key,
            },
        ) => {
            left_quad == right_quad
                && registered_shape(left_scene, left_key)
                    == registered_shape(right_scene, right_key)
        }
        _ => left == right,
    }
}

/// Rebuild from the muted baseline clone, preserving its clip registry and every non-glyph
/// primitive/scope. Only the target Panel curve's occurrence count around actual glyphs changes.
fn glyph_reference_scene(
    baseline: &Scene,
    glyph_source: &Scene,
    curve: tekes_ui_scene::ClipShape,
    target_occurrences: usize,
) -> Scene {
    let baseline_non_glyph: Vec<_> = baseline
        .primitives()
        .iter()
        .filter(|p| !matches!(p, Primitive::Glyphs(_)))
        .collect();
    let source_non_glyph: Vec<_> = glyph_source
        .primitives()
        .iter()
        .filter(|p| !matches!(p, Primitive::Glyphs(_)))
        .collect();
    assert_eq!(baseline.clip_shape_count(), glyph_source.clip_shape_count());
    assert_eq!(
        baseline.required_clip_geometry(),
        glyph_source.required_clip_geometry()
    );
    assert_eq!(baseline_non_glyph.len(), source_non_glyph.len());
    for (background, source) in baseline_non_glyph.iter().zip(&source_non_glyph) {
        assert!(
            same_scene_operation(baseline, background, glyph_source, source),
            "muting text rasterization must leave every original Panel background, clip scope, and registry-backed operation geometrically unchanged"
        );
    }

    let mut result = baseline.clone();
    result.clear(); // Scene::clear keeps the cloned immutable registry and required clip version.
    let mut active_scopes: Vec<Primitive> = Vec::new();
    let mut baseline_index = 0usize;
    for primitive in glyph_source.primitives() {
        if let Primitive::Glyphs(run) = primitive {
            if run.glyphs.is_empty() {
                continue;
            }
            let available = active_scopes
                .iter()
                .filter(|scope| is_target_curve(baseline, scope, curve))
                .count();
            assert!(
                available >= target_occurrences,
                "actual glyph scope must contain enough occurrences of the exact Panel curve"
            );
            for _ in 0..active_scopes.len() {
                result.pop_clip();
            }
            let mut kept_curve = 0usize;
            let mut replayed = 0usize;
            for scope in &active_scopes {
                if is_target_curve(baseline, scope, curve) {
                    if kept_curve >= target_occurrences {
                        continue;
                    }
                    kept_curve += 1;
                }
                result.push(scope.clone());
                replayed += 1;
            }
            result.push(Primitive::Glyphs(run.clone()));
            for _ in 0..replayed {
                result.pop_clip();
            }
            for scope in &active_scopes {
                result.push(scope.clone());
            }
            continue;
        }

        let background_primitive = baseline_non_glyph[baseline_index];
        assert!(same_scene_operation(
            glyph_source,
            primitive,
            baseline,
            background_primitive
        ));
        baseline_index += 1;
        match background_primitive {
            Primitive::PushClip(_) | Primitive::PushShapeClip(_) => {
                active_scopes.push((*background_primitive).clone());
            }
            Primitive::PopClip => {
                assert!(
                    active_scopes.pop().is_some(),
                    "baseline clip scopes balance"
                );
            }
            _ => {}
        }
        result.push((*background_primitive).clone());
    }
    assert_eq!(baseline_index, baseline_non_glyph.len());
    assert!(active_scopes.is_empty(), "baseline clip scopes balance");
    result
}

fn pixel(bytes: &[u8], width: usize, x: usize, y: usize) -> &[u8] {
    let i = (y * width + x) * 4;
    &bytes[i..i + 4]
}

#[derive(Clone)]
struct DiagnosticClipScope {
    shapes: Vec<tekes_ui_scene::ClipShape>,
    analytic: bool,
}

/// Mirrors the frozen renderer's source-bound work terms for diagnostics only:
/// target-pixel area times active clip-record depth times drawable instances.
/// The renderer's own preflight/error remains the only authority on acceptance.
fn renderer_work_diagnostic(scene: &Scene, size_px: (u32, u32)) -> String {
    use tekes_ui_scene::Primitive;

    let renderer_uses_analytic_clips = scene.required_clip_geometry() != 0
        || scene.primitives().iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::PushShapeClip(_) | Primitive::KeyedQuad { .. }
            )
        });
    let mut scopes = vec![DiagnosticClipScope {
        shapes: Vec::new(),
        analytic: false,
    }];
    let mut counts = [0usize; 8];
    let mut shape_scopes = Vec::new();
    let mut glyph_runs = Vec::new();
    let mut work_factor = 0u64;

    for (index, primitive) in scene.primitives().iter().enumerate() {
        match primitive {
            Primitive::PushClip(rect) => {
                let current = scopes.last().unwrap();
                let mut next = current.clone();
                if renderer_uses_analytic_clips
                    && let Ok(shape) = tekes_ui_scene::ClipShape::normalize(
                        *rect,
                        tekes_ui_scene::CornerRadii::ZERO,
                        Point::new(0., 0.),
                    )
                    && !next.shapes.contains(&shape)
                {
                    next.shapes.push(shape);
                }
                shape_scopes.push(format!(
                    "primitive={index} push_rect={rect:?} active_analytic={} record_depth={}",
                    next.analytic,
                    next.shapes.len(),
                ));
                scopes.push(next);
                counts[0] += 1;
            }
            Primitive::PushShapeClip(key) => {
                let current = scopes.last().unwrap();
                let mut next = current.clone();
                if let Ok(shape) = scene.clip_shape(key) {
                    if !next.shapes.contains(&shape) {
                        next.shapes.push(shape);
                    }
                    shape_scopes.push(format!(
                        "primitive={index} push_shape={shape:?} active_analytic=true record_depth={}",
                        next.shapes.len(),
                    ));
                } else {
                    shape_scopes.push(format!(
                        "primitive={index} push_shape_key={key:?} registry_lookup=ERROR"
                    ));
                }
                next.analytic = true;
                scopes.push(next);
                counts[1] += 1;
            }
            Primitive::PopClip => {
                if scopes.len() > 1 {
                    scopes.pop();
                }
                counts[2] += 1;
            }
            Primitive::Glyphs(run) => {
                let current = scopes.last().unwrap();
                let count = run.glyphs.len() as u64;
                let depth = if current.analytic {
                    current.shapes.len() as u64
                } else {
                    0
                };
                let term = depth.saturating_mul(count);
                work_factor = work_factor.saturating_add(term);
                glyph_runs.push(format!(
                    "primitive={index} glyph_count={} active_analytic={} record_depth={depth} depth_x_glyphs={term}",
                    run.glyphs.len(),
                    current.analytic,
                ));
                counts[3] += 1;
            }
            Primitive::Quad(_) => {
                let current = scopes.last().unwrap();
                if current.analytic {
                    work_factor = work_factor.saturating_add(current.shapes.len() as u64);
                }
                counts[4] += 1;
            }
            Primitive::KeyedQuad { .. } => {
                let current = scopes.last().unwrap();
                if renderer_uses_analytic_clips {
                    work_factor = work_factor.saturating_add(current.shapes.len() as u64);
                }
                counts[4] += 1;
            }
            Primitive::Image(_) => {
                let current = scopes.last().unwrap();
                if current.analytic {
                    work_factor = work_factor.saturating_add(current.shapes.len() as u64);
                }
                counts[5] += 1;
            }
            Primitive::TransformedImage(_) => {
                let current = scopes.last().unwrap();
                if current.analytic {
                    work_factor = work_factor.saturating_add(current.shapes.len() as u64);
                }
                counts[6] += 1;
            }
            Primitive::Shadow(_) => {
                let current = scopes.last().unwrap();
                if current.analytic {
                    work_factor = work_factor.saturating_add(current.shapes.len() as u64);
                }
                counts[7] += 1;
            }
            _ => {}
        }
    }
    let pixel_area = u64::from(size_px.0).saturating_mul(u64::from(size_px.1));
    let estimated_shader_work = pixel_area.saturating_mul(work_factor);
    format!(
        "scene_primitives={} clip_shapes={} required_clip_geometry={} renderer_analytic_path={}\nprimitive_counts=[push_rect:{}, push_shape:{}, pop:{}, glyph_runs:{}, quad_or_keyed:{}, image:{}, transformed_image:{}, shadow:{}]\noutput_pixels={size_px:?} pixel_area={pixel_area} depth_x_instances={work_factor} old_fulltarget_baseline_work_estimate={estimated_shader_work}\nshape_scopes=[{}]\nglyph_runs=[{}]",
        scene.len(),
        scene.clip_shape_count(),
        scene.required_clip_geometry(),
        renderer_uses_analytic_clips,
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        counts[4],
        counts[5],
        counts[6],
        counts[7],
        shape_scopes.join("; "),
        glyph_runs.join("; "),
    )
}

struct RenderFailureContext<'a> {
    clip_mode: &'a str,
    frame: Rect,
    clip: Rect,
    size_px: (u32, u32),
    backing: f32,
    text_scale: f32,
    theme: Theme,
    pipeline: Pipeline,
    nested: bool,
}

fn render_failure_diagnostic(
    host: &ContentHost,
    scene: &Scene,
    context: &RenderFailureContext<'_>,
    error: &impl std::fmt::Debug,
) -> String {
    let mut nodes = Vec::new();
    for id in [
        ComponentId(8301),
        ComponentId(8303),
        ComponentId(8304),
        ComponentId(8305),
        ComponentId(8306),
    ] {
        if let Some(node) = host.get(id) {
            let panel_geometry = node
                .component::<PanelContent>()
                .and_then(PanelContent::geometry)
                .map(|geometry| (geometry.outer, geometry.allocation, geometry.shape));
            let surface = host.text_surface(id).map(|surface| {
                (
                    surface.outer,
                    surface.inner,
                    surface.input.text.len(),
                    surface
                        .paragraph
                        .as_ref()
                        .map(|paragraph| paragraph.lines.len()),
                )
            });
            nodes.push(format!(
                "id={id:?} mount={} frame={:?} visible={:?} panel_geometry={panel_geometry:?} text_surface={surface:?}",
                node.mount_identity(), node.frame(), node.visible(),
            ));
        }
    }
    format!(
        "renderer error={error:?}; case clip_mode={} frame={:?} ancestor_clip={:?} output_px={:?} backing={} text_scale={} theme={:?} pipeline={:?} nested={}\n{}\noriginal_host_nodes=[{}]",
        context.clip_mode,
        context.frame,
        context.clip,
        context.size_px,
        context.backing,
        context.text_scale,
        context.theme,
        context.pipeline,
        context.nested,
        renderer_work_diagnostic(scene, context.size_px),
        nodes.join("; "),
    )
}

#[test]
fn actual_coretext_each_glyph_run_scissor_matches_parent_fallback_and_restores_state() {
    let mut renderer =
        tekes_ui_render::Renderer::new_headless().expect("actual Metal renderer required");
    let image = renderer.register_image(&[230, 51, 13, 255], 1, 1).unwrap();
    let output = out_dir();
    let clear = Rgba::rgb8(11, 177, 61);
    let backing = 2.0;
    let size_px = (640, 288);
    let frame = Rect::new(0., 0., 320., 144.);
    let mut text = Text::new(0.9);
    text.backing = backing;
    let sentinel = insert_offscreen_transparent_sentinel(&mut text.atlas, backing, size_px);
    let mut host = build_host(Pipeline::Glyph, false, image);
    let actual_scene = render(&mut host, &mut text, frame, frame, Theme::LIGHT);
    let runs: Vec<&GlyphRun> = actual_scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run),
            _ => None,
        })
        .collect();
    let actual_glyph_count: usize = runs.iter().map(|run| run.glyphs.len()).sum();
    assert_eq!(
        actual_glyph_count, 448,
        "R14 ordinary full-viewport CoreText case retains its 448 actual glyphs"
    );
    assert!(
        runs.iter().all(|run| (35..=40).contains(&run.glyphs.len())),
        "each original shaped row is isolated at its actual run granularity"
    );
    assert!(
        runs.iter()
            .flat_map(|run| &run.glyphs)
            .any(|glyph| glyph.glyph.left < 0 || glyph.glyph.top < 0),
        "finite CoreText corpus includes a negative bearing"
    );
    assert!(
        text.atlas.stats().glyphs > 0,
        "sentinel and CoreText glyphs were inserted before either render side"
    );

    let mut events = Vec::new();
    let mut ordinal = 0usize;
    let mut paired_runs = Vec::new();
    let run_depths = glyph_run_scope_depths(&actual_scene);
    assert_eq!(run_depths.len(), runs.len());
    for (run_index, run) in runs.iter().enumerate() {
        let normal_scene = isolated_glyph_run(&actual_scene, run_index, None);
        let fallback_scene = isolated_glyph_run(&actual_scene, run_index, Some(sentinel));
        let count = run.glyphs.len();
        let normal = render_qa_frame(
            &mut renderer,
            &normal_scene,
            &mut text.atlas,
            size_px,
            backing,
            clear,
            &output,
            ordinal,
            &format!("ordinary-run-{run_index:02}"),
            "run-scissor",
            count,
            run_depths[run_index],
            &mut events,
        );
        ordinal += 1;
        let fallback = render_qa_frame(
            &mut renderer,
            &fallback_scene,
            &mut text.atlas,
            size_px,
            backing,
            clear,
            &output,
            ordinal,
            &format!("ordinary-run-{run_index:02}"),
            "parent-scissor-sentinel-fallback",
            count + 1,
            run_depths[run_index],
            &mut events,
        );
        ordinal += 1;
        assert_eq!(
            normal, fallback,
            "per-run exact RGBA: ordinary CoreText run {run_index} new scissor equals same run forced to parent-scissor fallback"
        );
        let (x0, y0, x1, y1) = actual_run_geometry(run, backing);
        paired_runs.push(serde_json::json!({"run_index":run_index,"glyph_count":count,"actual_device_bounds":[x0,y0,x1,y1],"normal_render_ordinal":ordinal-2,"fallback_render_ordinal":ordinal-1,"exact_rgba_equal":true}));
    }
    assert_eq!(
        paired_runs.len(),
        12,
        "R14 full viewport case contains twelve actual CoreText row runs"
    );

    // Verify the next primitive after a Glyph run sees its own prepared scissor, and Pop after
    // an empty clip restores later Quad and Image draws. The target rectangles are chosen from
    // free space outside the selected run's actual bitmap bounds, without moving any glyph.
    let recovery_runs = [0usize, 1usize];
    let baseline_scene = isolated_glyph_runs(&actual_scene, &recovery_runs, None);
    let recovery_rect = recovery_rects(&baseline_scene, backing, size_px);
    let baseline = render_qa_frame(
        &mut renderer,
        &baseline_scene,
        &mut text.atlas,
        size_px,
        backing,
        clear,
        &output,
        ordinal,
        "restore-base",
        "run-scissor",
        runs[0].glyphs.len() + runs[1].glyphs.len(),
        run_depths[0].max(run_depths[1]),
        &mut events,
    );
    ordinal += 1;
    let mut normal_recovery_scene = isolated_glyph_runs(&actual_scene, &recovery_runs, None);
    append_clip_recovery(
        &mut normal_recovery_scene,
        recovery_rect[0],
        recovery_rect[1],
        recovery_rect[2],
        image,
    );
    let mut fallback_recovery_scene =
        isolated_glyph_runs(&actual_scene, &recovery_runs, Some(sentinel));
    append_clip_recovery(
        &mut fallback_recovery_scene,
        recovery_rect[0],
        recovery_rect[1],
        recovery_rect[2],
        image,
    );
    let normal_recovery = render_qa_frame(
        &mut renderer,
        &normal_recovery_scene,
        &mut text.atlas,
        size_px,
        backing,
        clear,
        &output,
        ordinal,
        "restore-sequence",
        "run-scissor",
        runs[0].glyphs.len() + runs[1].glyphs.len(),
        run_depths[0].max(run_depths[1]),
        &mut events,
    );
    ordinal += 1;
    let fallback_recovery = render_qa_frame(
        &mut renderer,
        &fallback_recovery_scene,
        &mut text.atlas,
        size_px,
        backing,
        clear,
        &output,
        ordinal,
        "restore-sequence",
        "parent-scissor-sentinel-fallback",
        runs[0].glyphs.len() + runs[1].glyphs.len() + 2,
        run_depths[0].max(run_depths[1]),
        &mut events,
    );
    ordinal += 1;
    assert_eq!(
        normal_recovery, fallback_recovery,
        "empty clip, Pop, later Quad and Image must have identical exact output under both run scopes"
    );
    let sample = |pixels: &[u8], rect: Rect| {
        let x = ((rect.center().x * backing).floor() as u32).min(size_px.0 - 1);
        let y = ((rect.center().y * backing).floor() as u32).min(size_px.1 - 1);
        let sampled = pixel(pixels, size_px.0 as usize, x as usize, y as usize);
        [sampled[0], sampled[1], sampled[2], sampled[3]]
    };
    assert_eq!(
        sample(&normal_recovery, recovery_rect[0]),
        sample(&baseline, recovery_rect[0]),
        "Quad inside an empty clip draws no pixels"
    );
    assert_eq!(
        sample(&normal_recovery, recovery_rect[1]),
        [20, 80, 230, 255],
        "Quad after Pop is visible outside the glyph run bounds"
    );
    assert_eq!(
        sample(&normal_recovery, recovery_rect[2]),
        [230, 51, 13, 255],
        "Image after Glyph and Pop is visible outside the glyph run bounds"
    );

    // A narrow edge case checks real CoreText run geometry at a clipped target boundary.
    let edge_size = (176, 192);
    let edge_frame = Rect::new(0., 0., 88., 96.);
    let mut edge_text = Text::new(1.0);
    edge_text.backing = 2.0;
    let edge_sentinel = insert_offscreen_transparent_sentinel(&mut edge_text.atlas, 2.0, edge_size);
    let mut edge_host = build_host(Pipeline::Glyph, false, image);
    let edge_scene = render(
        &mut edge_host,
        &mut edge_text,
        edge_frame,
        edge_frame,
        Theme::LIGHT,
    );
    let (edge_index, edge_run) = edge_scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run),
            _ => None,
        })
        .enumerate()
        .max_by(|(_, left), (_, right)| {
            actual_run_geometry(left, 2.0)
                .2
                .total_cmp(&actual_run_geometry(right, 2.0).2)
        })
        .expect("narrow CoreText scene emits a nonempty GlyphRun");
    let edge_normal_scene = isolated_glyph_run(&edge_scene, edge_index, None);
    let edge_fallback_scene = isolated_glyph_run(&edge_scene, edge_index, Some(edge_sentinel));
    let edge_count = edge_run.glyphs.len();
    let edge_depth = glyph_run_scope_depths(&edge_scene)[edge_index];
    let edge_normal = render_qa_frame(
        &mut renderer,
        &edge_normal_scene,
        &mut edge_text.atlas,
        edge_size,
        2.0,
        clear,
        &output,
        ordinal,
        "target-edge-run",
        "run-scissor",
        edge_count,
        edge_depth,
        &mut events,
    );
    ordinal += 1;
    let edge_fallback = render_qa_frame(
        &mut renderer,
        &edge_fallback_scene,
        &mut edge_text.atlas,
        edge_size,
        2.0,
        clear,
        &output,
        ordinal,
        "target-edge-run",
        "parent-scissor-sentinel-fallback",
        edge_count + 1,
        edge_depth,
        &mut events,
    );
    ordinal += 1;
    assert_eq!(
        edge_normal, edge_fallback,
        "target-edge CoreText run must match parent-scissor reference"
    );
    let edge_bounds = actual_run_geometry(edge_run, 2.0);
    assert!(
        edge_bounds.2 >= edge_size.0 as f32 - 64.0 || edge_bounds.0 <= 16.0,
        "target-edge corpus must place an actual CoreText bitmap within 32 device pixels of a target edge"
    );

    // Separate true Color Glyph coverage: CoreText rasterizes a visible emoji into a Color atlas
    // run; the same preinserted transparent sentinel remains the only reference-side addition.
    let emoji_text = "Visible Apple Color Emoji 😀 outside the clip edge";
    let mut color_text = Text::new(1.0);
    color_text.backing = 2.0;
    let color_size = (640, 288);
    let color_sentinel =
        insert_offscreen_transparent_sentinel(&mut color_text.atlas, 2.0, color_size);
    let mut color_host = build_glyph_host_with_text(emoji_text);
    let color_scene = render(&mut color_host, &mut color_text, frame, frame, Theme::LIGHT);
    let mut run_ordinal = 0usize;
    let color_run_index = color_scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Glyphs(run) if !run.glyphs.is_empty() => {
                let current = run_ordinal;
                run_ordinal += 1;
                run.glyphs
                    .iter()
                    .any(|glyph| glyph.glyph.format == RasterFormat::Color)
                    .then_some(current)
            }
            _ => None,
        })
        .next()
        .expect("actual CoreText emoji must produce at least one Color-format GlyphRun");
    let color_run = color_scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run),
            _ => None,
        })
        .nth(color_run_index)
        .unwrap();
    assert!(
        color_run
            .glyphs
            .iter()
            .any(|glyph| glyph.glyph.format == RasterFormat::Color)
    );
    let color_normal_scene = isolated_glyph_run(&color_scene, color_run_index, None);
    let mut color_baseline_scene = color_scene.clone();
    color_baseline_scene.clear();
    for primitive in color_scene.primitives() {
        if !matches!(primitive, Primitive::Glyphs(_)) {
            color_baseline_scene.push(primitive.clone());
        }
    }
    let color_fallback_scene =
        isolated_glyph_run(&color_scene, color_run_index, Some(color_sentinel));
    let color_count = color_run.glyphs.len();
    let color_depth = glyph_run_scope_depths(&color_scene)[color_run_index];
    let color_baseline = render_qa_frame(
        &mut renderer,
        &color_baseline_scene,
        &mut color_text.atlas,
        color_size,
        2.0,
        clear,
        &output,
        ordinal,
        "color-glyph-baseline",
        "non-glyph-only",
        0,
        color_depth,
        &mut events,
    );
    ordinal += 1;
    let color_normal = render_qa_frame(
        &mut renderer,
        &color_normal_scene,
        &mut color_text.atlas,
        color_size,
        2.0,
        clear,
        &output,
        ordinal,
        "color-glyph-run",
        "run-scissor",
        color_count,
        color_depth,
        &mut events,
    );
    ordinal += 1;
    let color_fallback = render_qa_frame(
        &mut renderer,
        &color_fallback_scene,
        &mut color_text.atlas,
        color_size,
        2.0,
        clear,
        &output,
        ordinal,
        "color-glyph-run",
        "parent-scissor-sentinel-fallback",
        color_count + 1,
        color_depth,
        &mut events,
    );
    assert_eq!(
        color_normal, color_fallback,
        "actual CoreText Color Glyph run matches its parent-scissor fallback reference"
    );
    assert!(
        color_normal
            .chunks_exact(4)
            .zip(color_baseline.chunks_exact(4))
            .any(|(actual, baseline)| actual != baseline),
        "the real Color Glyph run contributes pixels beyond the original non-glyph content"
    );

    std::fs::write(
        output.join("r19-render-events.jsonl"),
        events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n")
            + "\n",
    )
    .unwrap();
    std::fs::write(output.join("r19-glyphrun-summary.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "epoch":"R19-glyphrun-scissor-qa",
        "source_frame":"R14 ordinary 320x144 backing2 text-scale0.9 Light Glyph case, plus narrow target-edge and actual CoreText emoji finite cases",
        "ordinary_actual_glyphs":actual_glyph_count,
        "ordinary_run_count":runs.len(),
        "ordinary_scene_clip_stack_depths":run_depths,
        "ordinary_run_comparisons":paired_runs,
        "ordinary_recovery_adjacent_glyph_runs_quad_empty_clip_pop_image_exact":true,
        "ordinary_recovery_samples":{"empty_clip_matches_base":true,"post_pop_quad_visible":true,"post_pop_image_visible":true},
        "target_edge_exact_rgba_match":true,
        "target_edge_device_bounds":edge_bounds,
        "negative_bearing_present":true,
        "actual_coretext_color_glyph_run_count":color_count,
        "actual_coretext_color_glyph_exact_rgba_match":true,
        "sentinel":{"atlas":"same prepopulated GlyphAtlas for both sides","format":"Color","pixel":[0,0,0,0],"device_x":16392,"whole_run_fallback":"not certified by this public production-API pixel comparison; historical prepared-state diagnostics remain separate"},
        "prepared_state":"not observed by current production consumer API; do not join current events to historical diagnostic SDK results",
        "exact_rgba_scope":"per isolated GlyphRun only; not a full-scene equivalence theorem",
        "existing_450_18_matrix":"unchanged and remains an independent acceptance test"
    })).unwrap()).unwrap();
    ordinal += 1;
    assert_eq!(ordinal, events.len());
}

#[test]
fn actual_coretext_all_gpu_pipelines_and_full_frame_single_coverage_oracle() {
    let mut renderer =
        tekes_ui_render::Renderer::new_headless().expect("actual Metal renderer required");
    let red_image = renderer.register_image(&[230, 51, 13, 255], 1, 1).unwrap();
    let output = out_dir();
    let clear = Rgba::rgb8(11, 177, 61);
    let clear_pixel = [11, 177, 61, 255];
    let mut cases = 0usize;
    let mut oracle_cases = 0usize;
    for (width, height, nested, clip_mode) in [
        (320., 144., false, "ordinary"),
        (88., 96., false, "narrow"),
        (0., 96., false, "zero-width"),
        (274., 138., true, "nested"),
        (302.5, 157.5, true, "fractional-clip"),
    ] {
        for backing in [1., 1.5, 2.] {
            for text_scale in [0.9, 1., 1.2] {
                for theme in [Theme::LIGHT, Theme::DARK] {
                    let frame = Rect::new(0., 0., width, height);
                    let clip = if clip_mode == "fractional-clip" {
                        Rect::new(9.25, 7.5, (width - 22.).max(0.), (height - 19.).max(0.))
                    } else {
                        frame
                    };
                    let px_w = ((width * backing).round() as usize).max(1);
                    let px_h = ((height * backing).round() as usize).max(1);
                    for pipeline in Pipeline::ALL {
                        let mut host = build_host(pipeline, nested, red_image);
                        let mut text = Text::new(text_scale);
                        text.backing = backing;
                        assert!(text.services, "CoreText text services enabled");
                        let actual_scene = render(&mut host, &mut text, frame, clip, theme);
                        let output_size = (px_w as u32, px_h as u32);
                        let actual = renderer
                            .render_to_rgba8(
                                &actual_scene,
                                &mut text.atlas,
                                output_size,
                                backing,
                                clear,
                            )
                            .unwrap_or_else(|error| {
                                let context = RenderFailureContext {
                                    clip_mode, frame, clip, size_px: output_size, backing,
                                    text_scale, theme, pipeline, nested,
                                };
                                panic!(
                                    "actual Metal frame preflight failed; diagnostic evidence follows:\n{}",
                                    render_failure_diagnostic(&host, &actual_scene, &context, &error),
                                )
                            });
                        if width == 0. {
                            assert!(
                                actual.chunks_exact(4).all(|p| p == clear_pixel),
                                "zero-width Panel must paint no pixels; invisible children are not required to emit primitives"
                            );
                            std::fs::write(
                                output.join(format!(
                                    "zero-width-clear-b{backing}-t{text_scale}-{:?}.rgba",
                                    theme
                                )),
                                &actual,
                            )
                            .unwrap();
                            cases += 1;
                            continue;
                        }
                        assert!(
                            pipeline_present(&actual_scene, pipeline),
                            "missing actual {:?} primitive: {clip_mode}/{backing}/{text_scale}",
                            pipeline
                        );
                        assert!(
                            actual_scene.primitives().iter().any(
                                |p| matches!(p, Primitive::Glyphs(run) if !run.glyphs.is_empty())
                            ),
                            "CoreText did not rasterize the original text consumer"
                        );
                        assert!(
                            actual.chunks_exact(4).filter(|p| *p != clear_pixel).count() > 12,
                            "positive ink missing: {}/{clip_mode}/{backing}/{text_scale}",
                            pipeline.name()
                        );
                        if clip_mode == "ordinary" {
                            assert_eq!(
                                pixel(&actual, px_w, 0, 0),
                                clear_pixel,
                                "full render should retain transparent rounded corner gap"
                            );
                        } else if clip_mode == "fractional-clip" {
                            assert_eq!(
                                pixel(&actual, px_w, 0, 0),
                                clear_pixel,
                                "fractional ancestor clip must leave pixels outside its viewport clear"
                            );
                        }
                        let repeat = renderer
                            .render_to_rgba8(
                                &actual_scene,
                                &mut text.atlas,
                                (px_w as u32, px_h as u32),
                                backing,
                                clear,
                            )
                            .unwrap();
                        assert_eq!(
                            actual, repeat,
                            "same-frame render should be deterministic, not the no-double-AA oracle"
                        );

                        if matches!(pipeline, Pipeline::Glyph) && !nested && clip_mode == "ordinary"
                        {
                            // Re-render the exact same Host/layout with only text raster output muted.
                            // Measurement and shaping remain actual CoreText; this is the transparent background baseline.
                            text.draw_text = false;
                            let baseline_scene = render(&mut host, &mut text, frame, clip, theme);
                            let baseline = renderer
                                .render_to_rgba8(
                                    &baseline_scene,
                                    &mut text.atlas,
                                    (px_w as u32, px_h as u32),
                                    backing,
                                    clear,
                                )
                                .unwrap();
                            text.draw_text = true;
                            let curve = panel_curve(&host, &actual_scene, ComponentId(8304));
                            assert_eq!(
                                curve,
                                panel_curve(&host, &baseline_scene, ComponentId(8304)),
                                "muted baseline retains the original arranged rounded Panel geometry"
                            );
                            let single_curve_scene =
                                glyph_reference_scene(&baseline_scene, &actual_scene, curve, 1);
                            let unclipped_scene =
                                glyph_reference_scene(&baseline_scene, &actual_scene, curve, 0);
                            let expected = renderer
                                .render_to_rgba8(
                                    &single_curve_scene,
                                    &mut text.atlas,
                                    (px_w as u32, px_h as u32),
                                    backing,
                                    clear,
                                )
                                .unwrap();
                            let unmasked = renderer
                                .render_to_rgba8(
                                    &unclipped_scene,
                                    &mut text.atlas,
                                    (px_w as u32, px_h as u32),
                                    backing,
                                    clear,
                                )
                                .unwrap();
                            let mut mismatch = [0usize; 2]; // every contained and outside pixel center in the Panel AABB
                            let mut compared = [0usize; 2];
                            let mut outside_real_ink = 0usize;
                            for y in 0..px_h {
                                for x in 0..px_w {
                                    let logical = [
                                        ((x as f64) + 0.5) / f64::from(backing),
                                        ((y as f64) + 0.5) / f64::from(backing),
                                    ];
                                    let inside = curve.contains(logical).unwrap();
                                    let category = usize::from(!inside);
                                    compared[category] += 1;
                                    let actual_pixel = pixel(&actual, px_w, x, y);
                                    let expected_pixel = pixel(&expected, px_w, x, y);
                                    let ref_pixel = pixel(&unmasked, px_w, x, y);
                                    let base_pixel = pixel(&baseline, px_w, x, y);
                                    if actual_pixel != expected_pixel {
                                        mismatch[category] += 1;
                                    }
                                    if !inside && ref_pixel != base_pixel {
                                        outside_real_ink += 1;
                                    }
                                }
                            }
                            assert_eq!(
                                mismatch,
                                [0, 0],
                                "full-frame inner/outer pixels differ from one actual rounded coverage pass; possible duplicate AA/clip scope"
                            );
                            assert!(
                                compared.iter().all(|count| *count > 0),
                                "oracle must compare both the curve interior and exterior"
                            );
                            assert!(
                                outside_real_ink > 0,
                                "unmasked actual CoreText glyph coverage must cross the outer curve into its transparent gap"
                            );
                            assert_eq!(
                                pixel(&baseline, px_w, 0, 0),
                                clear_pixel,
                                "transparent gap baseline must remain clear; opaque overpaint would hide leakage"
                            );
                            std::fs::write(output.join(format!("coverage-{clip_mode}-b{backing}-t{text_scale}-{:?}.json", theme)), serde_json::to_vec_pretty(&serde_json::json!({
                                "inside_pixels_compared":compared[0],
                                "outside_pixels_compared":compared[1],
                                "inside_outside_mismatches":mismatch,
                                "unmasked_real_coretext_ink_pixels_in_curve_gap":outside_real_ink,
                                "actual_equals_single_shape_reference":true,
                                "background_corner_is_clear":true,
                                "render":"actual Metal",
                                "font_provider":"CoreTextShaper + CoreTextRasterizer + original TextSurfaceTextRenderer"
                            })).unwrap()).unwrap();
                            oracle_cases += 1;
                        }
                        std::fs::write(
                            output.join(format!(
                                "{}-{clip_mode}-b{backing}-t{text_scale}-{:?}.rgba",
                                pipeline.name(),
                                theme
                            )),
                            &actual,
                        )
                        .unwrap();
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 450);
    assert_eq!(oracle_cases, 18);
    std::fs::write(output.join("matrix-summary.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "format":"tekesui-issue82-local-panel-gpu-consumer-r2/1",
        "actual_metal":true,
        "source_head":"7ab8e3290d162d58e963011e8a969d5ed6bc57a0 (local main intake baseline; current results require actual execution)",
        "pipeline_combinations":cases,
        "single_shape_coverage_oracle_combinations":oracle_cases,
        "pipelines":["Quad","Image","TransformedImage","CoreText Glyph","Shadow"],
        "width_modes":["ordinary","narrow","zero-width full-frame clear negative","nested","fractional ancestor clip"],
        "backing_scales":[1.0,1.5,2.0],
        "text_scales":[0.9,1.0,1.2],
        "themes":["Light","Dark"],
        "oracle":"same ContentHost and shaped glyphs; same actual Panel background; exact one-root-shape replay compared against every frame pixel, plus unclipped glyph contribution in the transparent curve gap",
        "scope":"local-only; remote B2/public revision/native window/physical IME/VoiceOver/performance remain not_run"
    })).unwrap()).unwrap();
}
