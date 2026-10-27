#![recursion_limit = "256"]
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;

use tekes_ui_controls::{
    BarChart, ChartContent, ChartDatum, ControlKey, RequiredFocusResult, ScrollArea,
    composition::{
        ComponentId, Content, ContentEvent, ContentHost, HostRevealBudget, ListContent,
        ScrollContent, TextContent,
    },
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{
    Palette, Rgba, Theme,
    surface_palette::{resolve_data_plot_surface, resolve_surface},
};

const ROOT: ComponentId = ComponentId(104_900);
const CHART: ComponentId = ComponentId(104_901);
const LONG_LABEL: &str = "Long duplicate category label with original complete text";
fn clear() -> Rgba {
    Rgba::new(0., 0., 0., 0.)
}
fn quad_role_color_collisions(
    colors: tekes_ui_tokens::surface_palette::DataPlotSurfaceColors,
) -> Vec<String> {
    let roles = [
        ("F", colors.plot.focus.unwrap()),
        ("M", colors.plot.marker.unwrap()),
        ("S", colors.selection.background),
        ("B", colors.plot.boundary.unwrap()),
    ];
    let mut collisions = Vec::new();
    for i in 0..roles.len() {
        for j in (i + 1)..roles.len() {
            if roles[i].1 == roles[j].1 {
                collisions.push(format!("{}={}", roles[i].0, roles[j].0));
            }
        }
    }
    collisions
}

#[derive(Debug)]
struct DrawRecord {
    role: String,
    text: String,
    frame: Rect,
    start: usize,
    end: usize,
    rgba: [u8; 4],
    channel: &'static str,
}
struct RecordedText {
    inner: coretext::Text,
    draws: Vec<DrawRecord>,
}
impl tekes_ui_controls::ButtonTextRenderer for RecordedText {
    fn backing_scale(&self) -> f32 {
        self.inner.backing
    }
    fn measure(
        &mut self,
        s: &str,
        role: tekes_ui_tokens::proposal::TextRole,
        w: f32,
    ) -> tekes_ui_scene::Size {
        self.inner.measure(s, role, w)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        s: &str,
        role: tekes_ui_tokens::proposal::TextRole,
        frame: Rect,
        align: tekes_ui_controls::ButtonTextAlign,
        ink: Rgba,
    ) {
        let start = scene.primitives().len();
        self.inner.draw(scene, s, role, frame, align, ink);
        self.draws.push(DrawRecord {
            role: format!("{role:?}"),
            text: s.to_owned(),
            frame,
            start,
            end: scene.primitives().len(),
            rgba: rgba8(ink),
            channel: text_channel(s),
        });
    }
    fn resolved_font_size(&mut self, role: tekes_ui_tokens::proposal::TextRole) -> f32 {
        self.inner.resolved_font_size(role)
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        self.inner.text_surface_host()
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        self.inner.motion_context()
    }
}

fn rgba8(c: Rgba) -> [u8; 4] {
    [
        (c.r * 255.).round() as u8,
        (c.g * 255.).round() as u8,
        (c.b * 255.).round() as u8,
        (c.a * 255.).round() as u8,
    ]
}
fn rect4(r: Rect) -> [f32; 4] {
    [r.min_x(), r.min_y(), r.size.width, r.size.height]
}
fn data(value: f64, label: &str) -> Vec<ChartDatum> {
    (0..64).map(|_| ChartDatum::new(label, value)).collect()
}
fn chart(value: f64, label: &str, preferred_height: f32) -> ChartContent {
    ChartContent::try_new("Series", data(value, label))
        .unwrap()
        .preferred_height(preferred_height)
}
fn seed_original_selected_slot(host: &mut ContentHost) {
    // ChartContent intentionally has no model_mut/selected setter. Establish the
    // frozen index-2 fixture through the original keyboard owner, then blur while
    // retaining the business selection before the case's tested admission.
    assert!(host.focus(Some(CHART), true));
    for _ in 0..2 {
        host.event(&ContentEvent::Key {
            key: ControlKey::Right,
            down: true,
        });
    }
    assert_eq!(
        host.get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .selected_index(),
        Some(2)
    );
    assert!(host.focus(None, false));
    assert_eq!(host.focused(), None);
}
fn selected_theme_run_kind() -> String {
    let kind = std::env::var("CHART_THEME_RUN_KIND")
        .expect("CHART_THEME_RUN_KIND must identify the separately compiled palette artifact");
    assert!(
        kind == "default" || kind == "external",
        "unknown theme run kind: {kind}"
    );
    kind
}
fn read_plan() -> serde_json::Value {
    let path =
        std::env::var_os("CHART_FOUR_CASE_PLAN").expect("frozen owner-focus pairs-plan.json path");
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(value["max_input_pairs"], 4);
    assert_eq!(value["cases"].as_array().unwrap().len(), 4);
    value
}
fn output_dir() -> std::path::PathBuf {
    let p = std::path::PathBuf::from(
        std::env::var_os("CHART_FOUR_GPU_OUT").expect("unique SSD evidence output"),
    );
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn text_channel(text: &str) -> &'static str {
    if text.is_empty() {
        "NA"
    } else if text == LONG_LABEL || text == "ordinary category" {
        "C"
    } else if text.parse::<f64>().is_ok() {
        "V"
    } else if text.starts_with("following content line ") {
        "T"
    } else {
        "H"
    }
}
fn push_channel(
    scene: &Scene,
    draws: &[DrawRecord],
    channel: &str,
    colors: tekes_ui_tokens::surface_palette::DataPlotSurfaceColors,
) -> Scene {
    let mut out = Scene::new();
    for (i, prim) in scene.primitives().iter().enumerate() {
        let keep = match prim {
            Primitive::PushClip(_) | Primitive::PopClip => true,
            Primitive::Glyphs(_) => draws
                .iter()
                .any(|d| d.channel == channel && i >= d.start && i < d.end),
            Primitive::Quad(q) => match channel {
                "F" if q.border_width > 0. && Some(q.border_color) == colors.plot.focus => true,
                "F" if Some(q.fill) == colors.plot.focus && q.border_width == 0. => true,
                "M" if Some(q.fill) == colors.plot.marker => true,
                "S" if q.fill == colors.selection.background => true,
                "B" if Some(q.fill) == colors.plot.boundary => true,
                _ => false,
            },
            _ => false,
        };
        if keep {
            if let Primitive::Quad(q) = prim {
                if channel == "F"
                    && q.border_width > 0.
                    && Some(q.border_color) == colors.plot.focus
                {
                    let mut only = *q;
                    only.fill = clear();
                    out.push(Primitive::Quad(only));
                    continue;
                }
                if channel == "M" && Some(q.fill) == colors.plot.marker {
                    let mut only = *q;
                    only.border_width = 0.;
                    out.push(Primitive::Quad(only));
                    continue;
                }
            }
            out.push(prim.clone());
        }
    }
    out
}
fn primitive_kind(primitive: &Primitive) -> &'static str {
    match primitive {
        Primitive::Glyphs(_) => "Glyphs",
        Primitive::PushClip(_) => "PushClip",
        Primitive::PushShapeClip(_) => "PushShapeClip",
        Primitive::PopClip => "PopClip",
        Primitive::Quad(_) => "Quad",
        Primitive::KeyedQuad { .. } => "KeyedQuad",
        Primitive::Shadow(_) => "Shadow",
        Primitive::Image(_) => "Image",
        Primitive::TransformedImage(_) => "TransformedImage",
        _ => "Other",
    }
}

fn scene_glyph_clip_evidence(
    scene: &Scene,
    backing: f32,
    size: (u32, u32),
    atlas_page_count: u32,
) -> serde_json::Value {
    let mut active_clip_stack: Vec<serde_json::Value> = Vec::new();
    let mut underflow_count = 0usize;
    let mut glyph_clip_evidence = Vec::new();
    let primitive_kinds: Vec<_> = scene.primitives().iter().map(primitive_kind).collect();
    for (primitive_index, primitive) in scene.primitives().iter().enumerate() {
        match primitive {
            Primitive::PushClip(rect) => active_clip_stack.push(serde_json::json!({
                "kind":"rect", "rect_pt":rect4(*rect)
            })),
            Primitive::PushShapeClip(key) => {
                let shape = scene
                    .clip_shape(key)
                    .expect("captured ShapeKey must resolve in its Scene");
                let rect = shape.rect();
                let radii = shape.radii();
                active_clip_stack.push(serde_json::json!({
                    "kind":"shape", "rect_pt":rect4(rect),
                    "radii_pt":[radii.top_left,radii.top_right,radii.bottom_right,radii.bottom_left],
                    "shape_key":format!("{key:?}")
                }));
            }
            Primitive::PopClip => {
                underflow_count += usize::from(active_clip_stack.pop().is_none());
            }
            Primitive::Glyphs(run) => {
                let mut glyph_support_rects_px = Vec::new();
                let mut emitted_glyph_count = 0usize;
                let glyphs: Vec<_> = run.glyphs.iter().map(|g| {
                    let ox = (g.origin.x * backing).round();
                    let oy = (g.origin.y * backing).round();
                    let page_valid = g.glyph.page < atlas_page_count;
                    let finite_origin = ox.is_finite() && oy.is_finite();
                    let emitted = page_valid && finite_origin;
                    if emitted {
                        emitted_glyph_count += 1;
                        let rect = [
                            ox + g.glyph.left as f32,
                            oy + g.glyph.top as f32,
                            g.glyph.width as f32,
                            g.glyph.height as f32,
                        ];
                        glyph_support_rects_px.push(rect);
                        serde_json::json!({
                            "page":g.glyph.page,"page_valid":page_valid,
                            "origin_pt":[g.origin.x,g.origin.y],"bitmap_offset_px":[g.glyph.left,g.glyph.top],
                            "bitmap_size_px":[g.glyph.width,g.glyph.height],"destination_px":rect,
                            "finite_origin":finite_origin,"renderer_emits":emitted
                        })
                    } else {
                        serde_json::json!({
                            "page":g.glyph.page,"page_valid":page_valid,
                            "origin_pt":[g.origin.x,g.origin.y],"bitmap_offset_px":[g.glyph.left,g.glyph.top],
                            "bitmap_size_px":[g.glyph.width,g.glyph.height],"finite_origin":finite_origin,"renderer_emits":false
                        })
                    }
                }).collect();
                glyph_clip_evidence.push(serde_json::json!({
                    "primitive_index":primitive_index,
                    "source_glyph_count":run.glyphs.len(),
                    "emitted_glyph_count":emitted_glyph_count,
                    "glyphs":glyphs,
                    "glyph_support_rects_px":glyph_support_rects_px,
                    "active_clip_stack":active_clip_stack.clone()
                }));
            }
            _ => {}
        }
    }
    serde_json::json!({
        "viewport_px":[0,0,size.0,size.1],
        "primitive_count":scene.primitives().len(),
        "atlas_page_count":atlas_page_count,
        "scene_primitive_kinds":primitive_kinds,
        "glyph_clip_evidence":glyph_clip_evidence,
        "scene_clip_stack_underflow_count":underflow_count,
        "scene_clip_stack_final_depth":active_clip_stack.len()
    })
}

#[expect(clippy::too_many_arguments, reason = "Frozen GPU receipt axes remain explicit; this is a test capture helper, not a public API.")]
fn record_frame(
    out: &std::path::Path,
    gpu: &mut tekes_ui_render::Renderer,
    scene: &Scene,
    text: &mut RecordedText,
    palette: Palette,
    surface: &str,
    key: &str,
    frame: Rect,
    clip: Rect,
    backing: f32,
    focus_result: Option<RequiredFocusResult>,
    focused: Option<ComponentId>,
    selected: Option<usize>,
    data: &[ChartDatum],
    offset: Option<Point>,
) -> serde_json::Value {
    let colors = resolve_data_plot_surface(palette, surface).unwrap();
    let quad_role_collisions = quad_role_color_collisions(colors);
    let size = ((512. * backing) as u32, (280. * backing) as u32);
    let glyph_clip_metadata =
        scene_glyph_clip_evidence(scene, backing, size, text.inner.atlas.page_count());
    let ground = resolve_surface(palette, surface, "content", "rest")
        .unwrap()
        .background;
    let final_pixels = gpu
        .render_to_rgba8(scene, &mut text.inner.atlas, size, backing, ground)
        .unwrap();
    std::fs::write(out.join(format!("{key}.rgba8")), &final_pixels).unwrap();
    for channel in ["F", "V", "C", "T", "H", "M", "S", "B"] {
        let oracle = push_channel(scene, &text.draws, channel, colors);
        let bytes = gpu
            .render_to_rgba8(&oracle, &mut text.inner.atlas, size, backing, clear())
            .unwrap();
        std::fs::write(out.join(format!("{key}-{channel}.rgba8")), bytes).unwrap();
    }
    serde_json::json!({
        "key":key,"size_px":[size.0,size.1],"backing":backing,"frame":rect4(frame),"clip":rect4(clip),
        "required_focus_result":format!("{focus_result:?}"),"focused":focused.map(|v|v.0),"selected":selected,
        "data":data.iter().map(|d|serde_json::json!({"label":d.label,"value":d.value})).collect::<Vec<_>>(),
        "scroll_offset":offset.map(|p|[p.x,p.y]),
        "quad_role_color_collisions_diagnostic":quad_role_collisions,
        "quad_owner":"only ChartContent emits Quad primitives in these frozen trees; TextContent contributes Glyphs",
        "colors":{"ground":rgba8(ground),"plot_bg":rgba8(colors.plot.background),"plot_text":rgba8(colors.plot.text),"plot_focus":colors.plot.focus.map(rgba8),"plot_marker":colors.plot.marker.map(rgba8),"category_text":rgba8(colors.category.text),"selection_bg":rgba8(colors.selection.background),"plot_boundary":colors.plot.boundary.map(rgba8)},
        "focus_width":tekes_ui_tokens::proposal::metric("stroke", "c-focus-width").unwrap(),
        "focus_antialias_guard_px":tekes_ui_tokens::proposal::FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS,
        "draws":text.draws.iter().map(|d|serde_json::json!({"role":d.role,"text":d.text,"text_channel":d.channel,"frame":rect4(d.frame),"rgba":d.rgba,"start":d.start,"end":d.end})).collect::<Vec<_>>(),
        "quads":scene.primitives().iter().enumerate().filter_map(|(index,p)| if let Primitive::Quad(q)=p {Some(serde_json::json!({"primitive_index":index,"rect":rect4(q.rect),"fill":rgba8(q.fill),"border":rgba8(q.border_color),"border_width":q.border_width}))} else {None}).collect::<Vec<_>>(),
        "primitive_count":glyph_clip_metadata["primitive_count"],"atlas_page_count":glyph_clip_metadata["atlas_page_count"],
        "viewport_px":glyph_clip_metadata["viewport_px"],
        "scene_primitive_kinds":glyph_clip_metadata["scene_primitive_kinds"],
        "glyph_clip_evidence":glyph_clip_metadata["glyph_clip_evidence"],
        "scene_clip_stack_underflow_count":glyph_clip_metadata["scene_clip_stack_underflow_count"],
        "scene_clip_stack_final_depth":glyph_clip_metadata["scene_clip_stack_final_depth"],
        "glyph_destination_rule":"round(origin*backing)+(AtlasGlyph.left,top); size=AtlasGlyph.width,height, matching tekes-ui-render::GlyphInstance::new",
        "renderer":"tekes_ui_render::Renderer::new_headless/render_to_rgba8"
    })
}
fn assert_focus_only_quad_delta(
    unfocused: &serde_json::Value,
    focused: &serde_json::Value,
    focus: [u8; 4],
) -> Vec<serde_json::Value> {
    for field in [
        "frame",
        "clip",
        "size_px",
        "backing",
        "data",
        "selected",
        "scroll_offset",
        "colors",
        "required_focus_result",
    ] {
        assert_eq!(
            unfocused[field], focused[field],
            "focus/blur changed {field}"
        );
    }
    let text_records = |row: &serde_json::Value| {
        row["draws"].as_array().unwrap().iter().map(|d| {
            serde_json::json!({"role":d["role"],"text":d["text"],"text_channel":d["text_channel"],"frame":d["frame"],"rgba":d["rgba"]})
        }).collect::<Vec<_>>()
    };
    assert_eq!(
        text_records(unfocused),
        text_records(focused),
        "focus/blur changed text content, ink, or draw frames"
    );
    let geometry = |row: &serde_json::Value| {
        let mut values = row["quads"].as_array().unwrap().iter().map(|q| {
            serde_json::json!({"rect":q["rect"],"fill":q["fill"],"border":q["border"],"border_width":q["border_width"]}).to_string()
        }).collect::<Vec<_>>();
        values.sort();
        values
    };
    let before = geometry(unfocused);
    let after = geometry(focused);
    let mut remaining = before.clone();
    let mut added = Vec::new();
    for q in focused["quads"].as_array().unwrap() {
        let normalized = serde_json::json!({"rect":q["rect"],"fill":q["fill"],"border":q["border"],"border_width":q["border_width"]}).to_string();
        if let Some(i) = remaining.iter().position(|v| *v == normalized) {
            remaining.remove(i);
        } else {
            added.push(q.clone());
        }
    }
    assert!(
        remaining.is_empty(),
        "focus render removed or altered non-focus plot/bar/baseline geometry"
    );
    assert!(
        !added.is_empty(),
        "focus render added no geometry attributed to focus"
    );
    assert!(
        added
            .iter()
            .all(|q| q["fill"] == serde_json::json!(focus)
                || q["border"] == serde_json::json!(focus)),
        "added quad is not focus-colored by resolved semantic token"
    );
    let _ = after;
    added
}

fn assert_focus_ink_in_witness(out: &std::path::Path, key: &str, witness: Rect, scale: f32) {
    // This is a minimum positive-ink guard. The emitted F/T/M/S/B channels are
    // also consumed by the original offline analyzer for actual ground ratios,
    // AA support, continuous full-thickness area and occlusion.
    let bytes = std::fs::read(out.join(format!("{key}-F.rgba8"))).unwrap();
    let width = (512. * scale) as usize;
    let x0 = (witness.min_x() * scale).floor().max(0.) as usize;
    let x1 = (witness.max_x() * scale).ceil().max(0.) as usize;
    let y0 = (witness.min_y() * scale).floor().max(0.) as usize;
    let y1 = (witness.max_y() * scale).ceil().max(0.) as usize;
    let mut ink = 0usize;
    let mut points = std::collections::HashSet::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let i = (y * width + x) * 4;
            if bytes.get(i + 3).copied().unwrap_or(0) > 0 {
                ink += 1;
                points.insert((x, y));
            }
        }
    }
    let required = (8.0 * scale * scale).ceil() as usize;
    assert!(
        ink >= required,
        "focus witness {key} has only {ink} positive pixels in {witness:?}; requires at least {required} before detailed AA/contrast analysis"
    );
    let mut max_span = 0usize;
    let mut pending = points.clone();
    while let Some(start) = pending.iter().next().copied() {
        pending.remove(&start);
        let mut todo = vec![start];
        let mut minx = start.0;
        let mut maxx = start.0;
        let mut miny = start.1;
        let mut maxy = start.1;
        while let Some((x, y)) = todo.pop() {
            for n in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if pending.remove(&n) {
                    minx = minx.min(n.0);
                    maxx = maxx.max(n.0);
                    miny = miny.min(n.1);
                    maxy = maxy.max(n.1);
                    todo.push(n);
                }
            }
        }
        max_span = max_span.max((maxx - minx + 1).max(maxy - miny + 1));
    }
    assert!(
        max_span >= (4.0 * scale).ceil() as usize,
        "focus witness {key} lacks a continuous 2w positive component"
    );
}
fn visible_witness(host: &ContentHost, id: ComponentId) -> Rect {
    match host.required_focus_result(id) {
        Some(RequiredFocusResult::Visible { witness }) => witness,
        other => panic!("expected visible required owner witness, got {other:?}"),
    }
}
#[expect(clippy::too_many_arguments, reason = "Frozen GPU receipt axes remain explicit; this is a test capture helper, not a public API.")]
fn render(
    host: &mut ContentHost,
    text: &mut RecordedText,
    gpu: &mut tekes_ui_render::Renderer,
    out: &std::path::Path,
    key: &str,
    frame: Rect,
    clip: Rect,
    palette: Palette,
) -> serde_json::Value {
    text.draws.clear();
    let mut scene = Scene::new();
    host.render(&mut scene, text, frame, clip, palette).unwrap();
    let (selected, data) = host
        .get(CHART)
        .and_then(|n| n.component::<ChartContent>())
        .map(|c| (c.selected_index(), c.data().to_vec()))
        .unwrap_or((None, vec![]));
    let offset = host
        .get(ROOT)
        .and_then(|n| n.component::<ScrollContent>())
        .map(|s| s.scroll.offset);
    let focus_result = host.required_focus_result(CHART);
    let row = record_frame(
        out,
        gpu,
        &scene,
        text,
        palette,
        "canvas",
        key,
        frame,
        clip,
        text.inner.backing,
        focus_result.clone(),
        host.focused(),
        selected,
        &data,
        offset,
    );
    if let Some(RequiredFocusResult::Visible { witness }) = focus_result {
        assert!(witness.min_x() >= clip.min_x() && witness.max_x() <= clip.max_x());
        assert!(witness.min_y() >= clip.min_y() && witness.max_y() <= clip.max_y());
    }
    assert_clip_exterior_clear(
        &scene,
        &mut text.inner.atlas,
        gpu,
        clip,
        text.inner.backing,
        resolve_surface(palette, "canvas", "content", "rest")
            .unwrap()
            .background,
    );
    row
}
fn assert_clip_exterior_clear(
    scene: &Scene,
    atlas: &mut tekes_ui_scene::GlyphAtlas,
    gpu: &mut tekes_ui_render::Renderer,
    clip: Rect,
    scale: f32,
    ground: Rgba,
) {
    let size = ((512. * scale) as u32, (280. * scale) as u32);
    let pixels = gpu
        .render_to_rgba8(scene, atlas, size, scale, ground)
        .unwrap();
    let (w, h) = (size.0 as usize, size.1 as usize);
    for y in 0..h {
        for x in 0..w {
            let p = Point::new((x as f32 + 0.5) / scale, (y as f32 + 0.5) / scale);
            if !clip.contains(p) {
                let i = (y * w + x) * 4;
                assert_eq!(
                    &pixels[i..i + 4],
                    &rgba8(ground),
                    "pixel outside parent clip at {x},{y}"
                );
            }
        }
    }
}

/// Exact structural fixtures only. The theme/surface/backing lane is separate below.
#[test]
#[ignore = "default selected artifact only; original v1 structural runner uses Palette::from"]
fn preregistered_four_original_owner_cases() {
    assert_eq!(
        selected_theme_run_kind(),
        "default",
        "the original v1 structural runner uses Palette::from; these four fixtures remain default-only"
    );
    let plan = read_plan();
    let out = output_dir();
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let palette = Palette::from(Theme::LIGHT);
    let mut rows = vec![];

    // 1. Original visible narrow target: use original 64-bar, blank-label data.
    let spec = &plan["cases"][0];
    assert_eq!(spec["key"], "visible-narrow-static");
    let frame = Rect::new(16., 16., 120., 240.);
    assert_eq!(spec["data"]["count"], 64);
    assert_eq!(spec["data"]["selected"], 2);
    assert_eq!(spec["data"]["repeat_value"], 1.0);
    assert_eq!(spec["data"]["label"], "");
    assert_eq!(spec["parent_clip"], serde_json::json!([16, 16, 120, 240]));
    assert_eq!(spec["mode"], "light");
    assert_eq!(spec["backing"], 1);
    assert_eq!(spec["text_scale"], 1);
    assert_eq!(
        spec["owner_allocation"],
        serde_json::json!([16, 16, 120, 240])
    );
    let mut original_model = BarChart::new(data(1., ""));
    original_model.select(2);
    let mut original_view = tekes_ui_controls::collections_view::ChartView::new();
    let mut geom_text = coretext::Text::new(1.);
    geom_text.backing = 1.;
    assert_eq!(
        original_view.arrange_owner(&mut geom_text, &mut original_model, frame, None),
        tekes_ui_controls::collections_view::ChartOwnerCapacity::Sufficient
    );
    assert_eq!(original_view.layout().geometry.bars.len(), 64);
    assert_eq!(
        original_view.layout().geometry.bars[2].rect.size.width,
        0.,
        "original narrow fixture must retain its zero-width slot"
    );
    let mut text = RecordedText {
        inner: coretext::Text::new(1.),
        draws: vec![],
    };
    text.inner.backing = 1.;
    let mut host = ContentHost::new(
        Content::new(CHART, chart(1., "", 240.))
            .with_surface("canvas")
            .unwrap(),
    )
    .unwrap();
    let _setup = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "narrow-selection-setup",
        frame,
        frame,
        palette,
    );
    seed_original_selected_slot(&mut host);
    let before = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "narrow-unfocused",
        frame,
        frame,
        palette,
    );
    let before_state = format!("{host:?}");
    assert!(matches!(
        host.required_focus_result(CHART),
        Some(RequiredFocusResult::Visible { .. })
    ));
    let draws_before_focus = text.draws.len();
    assert!(host.focus(Some(CHART), true));
    let text_draw_calls_immediately_after_focus = text.draws.len();
    assert_eq!(
        text_draw_calls_immediately_after_focus, draws_before_focus,
        "focus admission must not invoke text drawing"
    );
    assert_eq!(host.focused(), Some(CHART));
    assert_eq!(
        host.get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .selected_index(),
        Some(2)
    );
    let after = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "narrow-focused",
        frame,
        frame,
        palette,
    );
    assert!(host.focus(None, false));
    assert_eq!(host.focused(), None);
    let unfocused_same_layout = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "narrow-postfocus-unfocused",
        frame,
        frame,
        palette,
    );
    let focus_geometry = assert_focus_only_quad_delta(
        &unfocused_same_layout,
        &after,
        rgba8(
            resolve_data_plot_surface(palette, "canvas")
                .unwrap()
                .plot
                .focus
                .unwrap(),
        ),
    );
    assert_focus_ink_in_witness(&out, "narrow-focused", visible_witness(&host, CHART), 1.);
    assert_eq!(
        host.get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .data()
            .len(),
        64
    );
    rows.push(serde_json::json!({"case":spec["key"],"before":before,"after":after,"host_before_focus":before_state,"text_draw_calls_before_focus":draws_before_focus,"text_draw_calls_frozen_immediately_after_focus":text_draw_calls_immediately_after_focus,"selection_after_focus":host.get(CHART).unwrap().component::<ChartContent>().unwrap().selected_index(),"unfocused_same_layout_control":unfocused_same_layout,"focus_geometry_delta":focus_geometry,"callback_observation":"text-renderer draw calls only; this fixture does not instrument action callbacks"}));

    // 2. Original long-label fixture with actual CoreText measurement and a real finite parent.
    let spec = &plan["cases"][1];
    assert_eq!(spec["key"], "longlabel-nonempty-original-scroll");
    let label = "Long duplicate category label with original complete text";
    assert_eq!(spec["data"]["count"], 64);
    assert_eq!(spec["data"]["selected"], 2);
    assert_eq!(spec["data"]["label"], label);
    assert_eq!(spec["data"]["repeat_value"], 1.0);
    assert_eq!(spec["parent_clip"], serde_json::json!([16, 16, 480, 240]));
    assert_eq!(spec["mode"], "light");
    assert_eq!(spec["backing"], 1);
    assert_eq!(spec["text_scale"], 1);
    assert_eq!(spec["host_frame"], serde_json::json!([16, 16, 480, 240]));
    let mut text = RecordedText {
        inner: coretext::Text::new(1.),
        draws: vec![],
    };
    text.inner.backing = 1.;
    let model = BarChart::new(data(1., label));
    let measure = tekes_ui_controls::collections_view::ChartView::new();
    let measured = measure.measure_owner(&mut text, &model, 480., 240.);
    let extent = measured
        .measured_label_extent
        .expect("real CoreText label extent");
    assert!(extent.height > 240.);
    let preferred = extent.height + 240.;
    let scroll = ScrollContent::new(
        ScrollArea::default(),
        Content::new(CHART, chart(1., label, preferred))
            .with_surface("canvas")
            .unwrap(),
    );
    let root_frame = Rect::new(16., 16., 480., 240.);
    let mut host = ContentHost::new(Content::new(ROOT, scroll)).unwrap();
    let setup_frame = Rect::new(16., 16., 480., preferred + 80.);
    let _setup = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "longlabel-selection-setup",
        setup_frame,
        setup_frame,
        palette,
    );
    let setup_offset = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    seed_original_selected_slot(&mut host);
    let setup_offset_after = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    assert_eq!(
        setup_offset_after, setup_offset,
        "selection setup must not pre-scroll the long-label case"
    );
    let before = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "longlabel-unfocused",
        root_frame,
        root_frame,
        palette,
    );
    let offset_before = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    let data_before = host
        .get(CHART)
        .unwrap()
        .component::<ChartContent>()
        .unwrap()
        .data()
        .to_vec();
    assert_eq!(data_before.len(), 64);
    assert!(
        data_before
            .iter()
            .all(|d| d.label == label && d.value == 1.)
    );
    let focus_result_before = host.required_focus_result(CHART);
    let initially_visible = matches!(
        &focus_result_before,
        Some(RequiredFocusResult::Visible { .. })
    );
    let initially_clipped = matches!(
        &focus_result_before,
        Some(RequiredFocusResult::Clipped { .. })
    );
    assert!(
        initially_visible || initially_clipped,
        "long-label fixture must have an actual finite Visible or Clipped owner result"
    );
    let draws_before_focus = text.draws.len();
    assert!(host.focus(Some(CHART), true));
    let text_draw_calls_immediately_after_focus = text.draws.len();
    assert_eq!(
        text_draw_calls_immediately_after_focus, draws_before_focus,
        "focus admission must not invoke text drawing"
    );
    assert_eq!(
        host.get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .selected_index(),
        Some(2)
    );
    let offset_after = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    if initially_clipped {
        assert!(
            offset_after.y > offset_before.y,
            "an initially clipped long-label owner must be revealed by original ScrollContent"
        );
    } else {
        assert_eq!(
            offset_after, offset_before,
            "an already visible long-label owner must not cause an unrelated scroll"
        );
    }
    let after = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "longlabel-focused",
        root_frame,
        root_frame,
        palette,
    );
    assert!(host.focus(None, false));
    assert_eq!(host.focused(), None);
    let unfocused_same_layout = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "longlabel-postfocus-unfocused",
        root_frame,
        root_frame,
        palette,
    );
    let offset_after_blur = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    assert_eq!(
        offset_after_blur, offset_after,
        "blur reference must preserve the post-focus offset"
    );
    let focus_geometry = assert_focus_only_quad_delta(
        &unfocused_same_layout,
        &after,
        rgba8(
            resolve_data_plot_surface(palette, "canvas")
                .unwrap()
                .plot
                .focus
                .unwrap(),
        ),
    );
    assert_focus_ink_in_witness(&out, "longlabel-focused", visible_witness(&host, CHART), 1.);
    rows.push(serde_json::json!({"case":spec["key"],"actual_coretext_label_extent":[extent.width,extent.height],"finite_chart_preferred_height":preferred,"focus_result_before":format!("{focus_result_before:?}"),"initially_visible":initially_visible,"initially_clipped":initially_clipped,"before":before,"after":after,"offset_before":[offset_before.x,offset_before.y],"offset_after":[offset_after.x,offset_after.y],"selection_setup_offset":[setup_offset.x,setup_offset.y],"text_draw_calls_before_focus":draws_before_focus,"text_draw_calls_frozen_immediately_after_focus":text_draw_calls_immediately_after_focus,"unfocused_same_layout_control":unfocused_same_layout,"focus_geometry_delta":focus_geometry,"callback_observation":"text-renderer draw calls only; this fixture does not instrument action callbacks"}));

    // 3. Exact 24-line original ScrollContent tree; private focus-witness plan is executed by Host focus admission.
    let spec = &plan["cases"][2];
    assert_eq!(spec["key"], "interior-only-original-reveal-before-after");
    assert_eq!(spec["data"]["count"], 64);
    assert_eq!(spec["data"]["selected"], 2);
    assert_eq!(spec["data"]["repeat_value"], -1.0);
    assert_eq!(spec["data"]["label"], "");
    assert_eq!(spec["mode"], "light");
    assert_eq!(spec["backing"], 1);
    assert_eq!(spec["host_frame"], serde_json::json!([16, 16, 480, 240]));
    assert_eq!(spec["parent_clip"], serde_json::json!([80, 80, 320, 120]));
    let lines = spec["following24lines"].as_str().unwrap();
    let text_content = Content::new(ComponentId(104_903), TextContent::new(lines));
    let children = ListContent::new("original chart and following TextContent", vec![
        Content::new(CHART, chart(-1., "", 240.))
            .with_surface("canvas")
            .unwrap(),
        text_content,
    ]);
    let children = Content::new(ComponentId(104_902), children);
    let scroll = ScrollContent::new(ScrollArea::default(), children);
    let frame = Rect::new(16., 16., 480., 240.);
    let clip = Rect::new(80., 80., 320., 120.);
    let mut host = ContentHost::new(Content::new(ROOT, scroll)).unwrap();
    let mut text = RecordedText {
        inner: coretext::Text::new(1.),
        draws: vec![],
    };
    text.inner.backing = 1.;
    let setup_frame = Rect::new(16., 16., 480., 600.);
    let _setup = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "interior-selection-setup",
        setup_frame,
        setup_frame,
        palette,
    );
    let setup_offset = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    seed_original_selected_slot(&mut host);
    let setup_offset_after = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    assert_eq!(
        setup_offset_after, setup_offset,
        "large-viewport selection setup must not pre-scroll the preregistered interior case"
    );
    let before = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "interior-before",
        frame,
        clip,
        palette,
    );
    assert!(matches!(
        host.required_focus_result(CHART),
        Some(RequiredFocusResult::Clipped { .. })
    ));
    let selected_before = host
        .get(CHART)
        .unwrap()
        .component::<ChartContent>()
        .unwrap()
        .selected_index();
    let data_before = host
        .get(CHART)
        .unwrap()
        .component::<ChartContent>()
        .unwrap()
        .data()
        .to_vec();
    let offset_before = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    assert_eq!(
        offset_before, setup_offset,
        "restoring the exact parent viewport/clip must preserve original scroll offset"
    );
    let readonly_plan_before = format!("{host:?}");
    let diagnostic_plan = host.plan_host_reveal(
        CHART,
        tekes_ui_controls::ScrollRevealOptions {
            animated: false,
            ..Default::default()
        },
        &[],
        HostRevealBudget::default(),
    );
    assert_eq!(
        format!("{host:?}"),
        readonly_plan_before,
        "public read-only frame plan mutated Host"
    );
    let diagnostic_plan_debug = format!("{diagnostic_plan:?}");
    // This invokes the original private required-witness plan/commit path exactly once.
    let draws_before_focus = text.draws.len();
    assert!(host.focus(Some(CHART), true));
    let text_draw_calls_immediately_after_focus = text.draws.len();
    assert_eq!(
        text_draw_calls_immediately_after_focus, draws_before_focus,
        "host reveal must not invoke text drawing"
    );
    let offset_after = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    assert!(
        offset_after.y > offset_before.y,
        "original reveal did not move the document: {offset_before:?} -> {offset_after:?}"
    );
    assert_eq!(
        host.get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .selected_index(),
        selected_before
    );
    assert_eq!(
        host.get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .data(),
        data_before
    );
    let after = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "interior-after",
        frame,
        clip,
        palette,
    );
    assert!(host.focus(None, false));
    assert_eq!(host.focused(), None);
    let unfocused_same_layout = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "interior-postreveal-unfocused",
        frame,
        clip,
        palette,
    );
    let offset_after_blur = host
        .get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    assert_eq!(
        offset_after_blur, offset_after,
        "blur reference must preserve the post-reveal offset"
    );
    let witness = match host.required_focus_result(CHART) {
        Some(RequiredFocusResult::Visible { witness }) => witness,
        other => panic!("post-reveal owner witness missing: {other:?}"),
    };
    assert!(witness.min_x() >= clip.min_x() && witness.max_x() <= clip.max_x());
    assert!(witness.min_y() >= clip.min_y() && witness.max_y() <= clip.max_y());
    let focus_geometry = assert_focus_only_quad_delta(
        &unfocused_same_layout,
        &after,
        rgba8(
            resolve_data_plot_surface(palette, "canvas")
                .unwrap()
                .plot
                .focus
                .unwrap(),
        ),
    );
    assert_focus_ink_in_witness(&out, "interior-after", witness, 1.);
    rows.push(serde_json::json!({"case":spec["key"],"before":before,"after":after,"offset_setup":[setup_offset.x,setup_offset.y],"offset_before":[offset_before.x,offset_before.y],"offset_after":[offset_after.x,offset_after.y],"visible_witness":rect4(witness),"read_only_generic_host_plan_debug_not_focus_plan":diagnostic_plan_debug,"text_draw_calls_before_focus":draws_before_focus,"text_draw_calls_frozen_immediately_after_focus":text_draw_calls_immediately_after_focus,"unfocused_same_layout_control":unfocused_same_layout,"focus_geometry_delta":focus_geometry,"callback_observation":"text-renderer draw calls only; this fixture does not instrument action callbacks"}));

    // 4. Tiny static negative: fresh admission refuses; existing focused owner is retained on reflow.
    let spec = &plan["cases"][3];
    assert_eq!(spec["key"], "unrevealable-tiny-negative");
    assert_eq!(spec["data"]["count"], 64);
    assert_eq!(spec["data"]["selected"], 2);
    assert_eq!(spec["data"]["repeat_value"], 1.0);
    assert_eq!(spec["data"]["label"], "");
    assert_eq!(spec["text_scale"], 1);
    assert_eq!(spec["parent_clip"], serde_json::json!([16, 16, 4, 4]));
    assert_eq!(spec["mode"], "dark-ic");
    assert_eq!(spec["backing"], 1);
    assert_eq!(spec["owner_allocation"], serde_json::json!([16, 16, 4, 4]));
    let tiny = Rect::new(16., 16., 4., 4.);
    let palette = Palette::from(Theme::DARK_INCREASED);
    let mut text = RecordedText {
        inner: coretext::Text::new(1.),
        draws: vec![],
    };
    text.inner.backing = 1.;
    let mut host = ContentHost::new(
        Content::new(CHART, chart(1., "", 240.))
            .with_surface("canvas")
            .unwrap(),
    )
    .unwrap();
    let enough_frame = Rect::new(16., 16., 480., 240.);
    let _setup = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "tiny-selection-setup",
        enough_frame,
        enough_frame,
        palette,
    );
    seed_original_selected_slot(&mut host);
    let _ = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "tiny-fresh-unfocused",
        tiny,
        tiny,
        palette,
    );
    let data_before = host
        .get(CHART)
        .unwrap()
        .component::<ChartContent>()
        .unwrap()
        .data()
        .to_vec();
    let selected_before = host
        .get(CHART)
        .unwrap()
        .component::<ChartContent>()
        .unwrap()
        .selected_index();
    let text_draw_calls_before_refusal = text.draws.len();
    assert!(!host.focus(Some(CHART), true));
    let text_draw_calls_after_fresh_focus_refusal = text.draws.len();
    assert_eq!(host.focused(), None);
    assert_eq!(
        text_draw_calls_after_fresh_focus_refusal, text_draw_calls_before_refusal,
        "tiny fresh admission rejection must not invoke text drawing"
    );
    assert_eq!(
        host.get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .data(),
        data_before
    );
    assert_eq!(
        host.get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .selected_index(),
        selected_before
    );
    let _ = render(
        &mut host,
        &mut text,
        &mut gpu,
        &out,
        "tiny-fresh-refused",
        tiny,
        tiny,
        palette,
    );
    let mut enough = ContentHost::new(
        Content::new(CHART, chart(1., "", 240.))
            .with_surface("canvas")
            .unwrap(),
    )
    .unwrap();
    let enough_frame = Rect::new(16., 16., 480., 240.);
    let _ = render(
        &mut enough,
        &mut text,
        &mut gpu,
        &out,
        "tiny-existing-owner-setup",
        enough_frame,
        enough_frame,
        palette,
    );
    seed_original_selected_slot(&mut enough);
    let _ = render(
        &mut enough,
        &mut text,
        &mut gpu,
        &out,
        "tiny-existing-owner-before",
        enough_frame,
        enough_frame,
        palette,
    );
    let text_draw_calls_before_existing_focus = text.draws.len();
    assert!(enough.focus(Some(CHART), true));
    let text_draw_calls_immediately_after_existing_focus = text.draws.len();
    assert_eq!(
        text_draw_calls_immediately_after_existing_focus, text_draw_calls_before_existing_focus,
        "existing-owner focus confirmation must not invoke text drawing"
    );
    let _ = render(
        &mut enough,
        &mut text,
        &mut gpu,
        &out,
        "tiny-existing-owner-focused",
        enough_frame,
        enough_frame,
        palette,
    );
    let retained_data = enough
        .get(CHART)
        .unwrap()
        .component::<ChartContent>()
        .unwrap()
        .data()
        .to_vec();
    let retained_selection = enough
        .get(CHART)
        .unwrap()
        .component::<ChartContent>()
        .unwrap()
        .selected_index();
    let tiny_result = render(
        &mut enough,
        &mut text,
        &mut gpu,
        &out,
        "tiny-existing-owner-after-reflow",
        tiny,
        tiny,
        palette,
    );
    assert_eq!(enough.focused(), Some(CHART));
    assert_eq!(
        enough
            .get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .data(),
        retained_data
    );
    assert_eq!(
        enough
            .get(CHART)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .selected_index(),
        retained_selection
    );
    assert_eq!(
        tiny_result["required_focus_result"],
        format!("{:?}", Some(RequiredFocusResult::Insufficient))
    );
    rows.push(serde_json::json!({"case":spec["key"],"positive_visibility_evidence":false,"text_draw_calls_before_fresh_focus":text_draw_calls_before_refusal,"text_draw_calls_after_fresh_focus_refusal":text_draw_calls_after_fresh_focus_refusal,"text_draw_calls_before_existing_focus":text_draw_calls_before_existing_focus,"text_draw_calls_immediately_after_existing_focus":text_draw_calls_immediately_after_existing_focus,"refused_fresh_state":{"focused":false,"selected":selected_before,"data_count":data_before.len()},"existing_owner_after_reflow":{"focused":enough.focused().map(|v|v.0),"selected":retained_selection,"data_count":retained_data.len(),"capture":tiny_result},"callback_observation":"text-renderer draw calls only; action callbacks are not instrumented"}));

    std::fs::write(
        out.join("structural-cases.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
}

/// Separate finite #104 shared packet lane; it does not multiply the four structural fixtures.
#[test]
#[ignore = "run separately against default and external selected-artifact builds"]
fn shared_data_plot_packet_gpu_lane() {
    use tekes_ui_tokens::surface_palette::{
        resolve_data_plot_surface, resolve_surface, surface_names,
    };
    let run_kind = selected_theme_run_kind();
    let theme_id = if run_kind == "default" {
        "default"
    } else {
        "chart-external"
    };
    let out = output_dir().join("shared-packet");
    std::fs::create_dir_all(&out).unwrap();
    assert_eq!(
        surface_names().len(),
        7,
        "original #104 applicable surface list changed"
    );
    assert_eq!(Theme::ALL.len(), 4);
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut rows = vec![];
    for (mode_index, mode) in Theme::ALL.into_iter().enumerate() {
        let mode_name = [
            "Light",
            "Dark",
            "LightIncreasedContrast",
            "DarkIncreasedContrast",
        ][mode_index];
        let palette = if run_kind == "default" {
            Palette::from(mode)
        } else {
            Palette::for_theme_definition(mode, "chart-external")
                .expect("selected external artifact must compile chart-external")
        };
        for &surface in surface_names() {
            let packet = resolve_data_plot_surface(palette, surface).unwrap();
            let quad_role_collisions = quad_role_color_collisions(packet);
            for backing in [1f32, 2.] {
                let mut text = RecordedText {
                    inner: coretext::Text::new(1.),
                    draws: vec![],
                };
                text.inner.backing = backing;
                let mut model = BarChart::new(vec![
                    ChartDatum::new("ordinary category", 1.),
                    ChartDatum::new("ordinary category", 0.),
                ]);
                model.select(0);
                let mut view = tekes_ui_controls::collections_view::ChartView::new();
                let frame = Rect::new(0., 0., 300., 200.);
                assert_eq!(
                    view.arrange_owner(&mut text, &mut model, frame, None),
                    tekes_ui_controls::collections_view::ChartOwnerCapacity::Sufficient
                );
                view.try_focus_owner(true, &mut model).unwrap();
                let expected = view.owner_focus_space(&model);
                let mut scene = Scene::new();
                assert_eq!(
                    view.paint_owner_on_surface(
                        &mut scene, &mut text, &mut model, palette, surface
                    )
                    .unwrap(),
                    expected
                );
                let size = ((300. * backing) as u32, (200. * backing) as u32);
                let ground = resolve_surface(palette, surface, "content", "rest")
                    .unwrap()
                    .background;
                let pixels = gpu
                    .render_to_rgba8(&scene, &mut text.inner.atlas, size, backing, ground)
                    .unwrap();
                let key = format!("{:?}-{theme_id}-{surface}-{}x", mode, backing as u32);
                let view_focus_clip_evidence =
                    scene_glyph_clip_evidence(&scene, backing, size, text.inner.atlas.page_count());
                let view_focus_draws = std::mem::take(&mut text.draws);
                std::fs::write(out.join(format!("{key}-view.rgba8")), &pixels).unwrap();
                for channel in ["F", "V", "C", "T", "H", "M", "S", "B"] {
                    let oracle = push_channel(&scene, &view_focus_draws, channel, packet);
                    let rgba = gpu
                        .render_to_rgba8(&oracle, &mut text.inner.atlas, size, backing, clear())
                        .unwrap();
                    std::fs::write(out.join(format!("{key}-view-{channel}.rgba8")), rgba).unwrap();
                }
                let view_quads = scene.primitives().iter().enumerate().filter_map(|(index, p)| {
                    if let Primitive::Quad(q) = p {
                        Some(serde_json::json!({"primitive_index":index,"rect":rect4(q.rect),"fill":rgba8(q.fill),"border":rgba8(q.border_color),"border_width":q.border_width}))
                    } else { None }
                }).collect::<Vec<_>>();
                view.blur(&mut model);
                let mut view_blur_scene = Scene::new();
                let view_blur_result = view
                    .paint_owner_on_surface(
                        &mut view_blur_scene,
                        &mut text,
                        &mut model,
                        palette,
                        surface,
                    )
                    .unwrap();
                let view_blur_pixels = gpu
                    .render_to_rgba8(
                        &view_blur_scene,
                        &mut text.inner.atlas,
                        size,
                        backing,
                        ground,
                    )
                    .unwrap();
                let view_blur_clip_evidence = scene_glyph_clip_evidence(
                    &view_blur_scene,
                    backing,
                    size,
                    text.inner.atlas.page_count(),
                );
                let view_blur_draws = std::mem::take(&mut text.draws);
                std::fs::write(
                    out.join(format!("{key}-view-blur.rgba8")),
                    &view_blur_pixels,
                )
                .unwrap();
                let view_blur_quads = view_blur_scene.primitives().iter().enumerate().filter_map(|(index, p)| {
                    if let Primitive::Quad(q) = p {
                        Some(serde_json::json!({"primitive_index":index,"rect":rect4(q.rect),"fill":rgba8(q.fill),"border":rgba8(q.border_color),"border_width":q.border_width}))
                    } else { None }
                }).collect::<Vec<_>>();

                let mut content_text = RecordedText {
                    inner: coretext::Text::new(1.),
                    draws: vec![],
                };
                content_text.inner.backing = backing;
                let content_component = ChartContent::try_new("Series", vec![
                    ChartDatum::new("ordinary category", 1.),
                    ChartDatum::new("ordinary category", 0.),
                ])
                .unwrap()
                .preferred_height(200.);
                let content = Content::new(CHART, content_component)
                    .with_surface(surface)
                    .unwrap();
                let mut content_host = ContentHost::new(content).unwrap();
                let mut content_arrange_scene = Scene::new();
                content_host
                    .render(
                        &mut content_arrange_scene,
                        &mut content_text,
                        frame,
                        frame,
                        palette,
                    )
                    .unwrap();
                content_text.draws.clear();
                let content_text_draw_calls_before_focus = content_text.draws.len();
                assert!(content_host.focus(Some(CHART), true));
                let content_text_draw_calls_immediately_after_focus = content_text.draws.len();
                assert_eq!(
                    content_text_draw_calls_immediately_after_focus,
                    content_text_draw_calls_before_focus,
                    "ContentHost focus admission after arrange must not draw text"
                );
                let mut content_scene = Scene::new();
                content_host
                    .render(&mut content_scene, &mut content_text, frame, frame, palette)
                    .unwrap();
                let content_pixels = gpu
                    .render_to_rgba8(
                        &content_scene,
                        &mut content_text.inner.atlas,
                        size,
                        backing,
                        ground,
                    )
                    .unwrap();
                let content_focus_clip_evidence = scene_glyph_clip_evidence(
                    &content_scene,
                    backing,
                    size,
                    content_text.inner.atlas.page_count(),
                );
                let content_focus_draws = std::mem::take(&mut content_text.draws);
                std::fs::write(out.join(format!("{key}-content.rgba8")), &content_pixels).unwrap();
                for channel in ["F", "V", "C", "T", "H", "M", "S", "B"] {
                    let oracle =
                        push_channel(&content_scene, &content_focus_draws, channel, packet);
                    let rgba = gpu
                        .render_to_rgba8(
                            &oracle,
                            &mut content_text.inner.atlas,
                            size,
                            backing,
                            clear(),
                        )
                        .unwrap();
                    std::fs::write(out.join(format!("{key}-content-{channel}.rgba8")), rgba)
                        .unwrap();
                }
                let content_quads = content_scene.primitives().iter().enumerate().filter_map(|(index, p)| {
                    if let Primitive::Quad(q) = p {
                        Some(serde_json::json!({"primitive_index":index,"rect":rect4(q.rect),"fill":rgba8(q.fill),"border":rgba8(q.border_color),"border_width":q.border_width}))
                    } else { None }
                }).collect::<Vec<_>>();
                let content_focus = format!("{:?}", content_host.required_focus_result(CHART));
                assert!(content_host.focus(None, false));
                let mut content_blur_scene = Scene::new();
                content_host
                    .render(
                        &mut content_blur_scene,
                        &mut content_text,
                        frame,
                        frame,
                        palette,
                    )
                    .unwrap();
                let content_blur_pixels = gpu
                    .render_to_rgba8(
                        &content_blur_scene,
                        &mut content_text.inner.atlas,
                        size,
                        backing,
                        ground,
                    )
                    .unwrap();
                let content_blur_clip_evidence = scene_glyph_clip_evidence(
                    &content_blur_scene,
                    backing,
                    size,
                    content_text.inner.atlas.page_count(),
                );
                let content_blur_draws = std::mem::take(&mut content_text.draws);
                std::fs::write(
                    out.join(format!("{key}-content-blur.rgba8")),
                    &content_blur_pixels,
                )
                .unwrap();
                let content_blur_quads = content_blur_scene.primitives().iter().enumerate().filter_map(|(index, p)| {
                    if let Primitive::Quad(q) = p {
                        Some(serde_json::json!({"primitive_index":index,"rect":rect4(q.rect),"fill":rgba8(q.fill),"border":rgba8(q.border_color),"border_width":q.border_width}))
                    } else { None }
                }).collect::<Vec<_>>();
                let content_node = content_host
                    .get(CHART)
                    .unwrap()
                    .component::<ChartContent>()
                    .unwrap();
                assert_eq!(
                    content_node.data(),
                    model.data(),
                    "paired View/Content lane must use identical chart data"
                );
                assert_eq!(
                    content_node.selected_index(),
                    model.selected(),
                    "paired View/Content lane must use identical selection"
                );
                let view_blur_selected = model.selected();
                let view_blur_data_count = model.data().len();
                let content_blur_selected = content_node.selected_index();
                let content_blur_data_count = content_node.data().len();
                assert_eq!(view_blur_selected, model.selected());
                assert_eq!(content_blur_selected, model.selected());
                assert_eq!(view_blur_data_count, model.data().len());
                assert_eq!(content_blur_data_count, model.data().len());
                rows.push(serde_json::json!({
                    "key":key,"run_kind":run_kind,"mode":mode_name,"theme_debug":format!("{mode:?}"),"mode_index":mode_index,
                    "theme_id":theme_id,"surface":surface,"backing":backing,"size_px":[size.0,size.1],
                    "frame":rect4(frame),"clip":rect4(frame),"required_focus_result":format!("{expected:?}"),
                    "view_blur_focus_result":format!("{view_blur_result:?}"),"content_focus":content_focus,
                    "content_text_draw_calls_before_focus_after_warmup_clear":content_text_draw_calls_before_focus,
                    "content_text_draw_calls_immediately_after_focus":content_text_draw_calls_immediately_after_focus,
                    "ground":rgba8(ground),
                    "plot":{"background":rgba8(packet.plot.background),"text":rgba8(packet.plot.text),"focus":packet.plot.focus.map(rgba8),"marker":packet.plot.marker.map(rgba8),"boundary":packet.plot.boundary.map(rgba8)},
                    "category":{"background":rgba8(packet.category.background),"text":rgba8(packet.category.text)},
                    "selection_background":rgba8(packet.selection.background),
                    "focus_width":tekes_ui_tokens::proposal::metric("stroke","c-focus-width").unwrap(),
                    "focus_antialias_guard_px":tekes_ui_tokens::proposal::FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS,
                    "data_count":model.data().len(),"selected":model.selected(),
                    "view_blur_data_count":view_blur_data_count,"view_blur_selected":view_blur_selected,
                    "content_blur_data_count":content_blur_data_count,"content_blur_selected":content_blur_selected,
                    "view_quads":view_quads,"view_blur_quads":view_blur_quads,"content_quads":content_quads,"content_blur_quads":content_blur_quads,
                    "view_focus_clip_evidence":view_focus_clip_evidence,"view_blur_clip_evidence":view_blur_clip_evidence,
                    "content_focus_clip_evidence":content_focus_clip_evidence,"content_blur_clip_evidence":content_blur_clip_evidence,
                    "view_text_draws":view_focus_draws.iter().map(|d|serde_json::json!({"role":d.role,"text":d.text,"channel":d.channel,"rgba":d.rgba,"frame":rect4(d.frame),"start":d.start,"end":d.end})).collect::<Vec<_>>(),
                    "view_blur_text_draws":view_blur_draws.iter().map(|d|serde_json::json!({"role":d.role,"text":d.text,"channel":d.channel,"rgba":d.rgba,"frame":rect4(d.frame),"start":d.start,"end":d.end})).collect::<Vec<_>>(),
                    "content_text_draws":content_focus_draws.iter().map(|d|serde_json::json!({"role":d.role,"text":d.text,"channel":d.channel,"rgba":d.rgba,"frame":rect4(d.frame),"start":d.start,"end":d.end})).collect::<Vec<_>>(),
                    "content_blur_text_draws":content_blur_draws.iter().map(|d|serde_json::json!({"role":d.role,"text":d.text,"channel":d.channel,"rgba":d.rgba,"frame":rect4(d.frame),"start":d.start,"end":d.end})).collect::<Vec<_>>(),
                    "current_original_view_path":"current_original_view","current_content_host_path":"current_content_host",
                    "same_input_content_capture":true,"quad_role_color_collisions_diagnostic":quad_role_collisions,
                    "quad_owner_exclusive":"this lane renders one ChartView or one ChartContent owner; every Quad belongs to that chart",
                    "view_content_final_rgba_equal":pixels==content_pixels,
                    "view_content_blur_rgba_equal":view_blur_pixels==content_blur_pixels
                }));
            }
        }
    }
    assert_eq!(
        rows.len(),
        56,
        "each separately compiled selected artifact contributes 4 modes x 7 surfaces x 1x/2x"
    );
    std::fs::write(
        out.join("lane.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
}
