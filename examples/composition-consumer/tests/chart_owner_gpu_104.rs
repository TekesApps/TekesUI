#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;

use tekes_ui_controls::{BarChart, ChartDatum, collections_view::ChartView};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, surface_palette::{resolve_data_plot_surface, resolve_surface, surface_names}};

const LONG_LABEL: &str = "Long duplicate category label with original complete text";
const OUTPUT_ENV: &str = "CHART_104_GPU_OUT";

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
    fn backing_scale(&self) -> f32 { self.inner.backing }
    fn measure(&mut self, s: &str, role: tekes_ui_tokens::proposal::TextRole, w: f32) -> tekes_ui_scene::Size {
        self.inner.measure(s, role, w)
    }
    fn draw(&mut self, scene: &mut Scene, s: &str, role: tekes_ui_tokens::proposal::TextRole, frame: Rect, align: tekes_ui_controls::ButtonTextAlign, ink: Rgba) {
        let start = scene.primitives().len();
        self.inner.draw(scene, s, role, frame, align, ink);
        self.draws.push(DrawRecord { role: format!("{role:?}"), text: s.to_owned(), frame, start, end: scene.primitives().len(), rgba: rgba8(ink), channel: text_channel(s) });
    }
    fn resolved_font_size(&mut self, role: tekes_ui_tokens::proposal::TextRole) -> f32 { self.inner.resolved_font_size(role) }
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> { self.inner.text_surface_host() }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> { self.inner.motion_context() }
}
fn text_channel(s: &str) -> &'static str {
    if s.is_empty() { "NA" }
    else if s == LONG_LABEL { "C" }
    else if s.parse::<f64>().is_ok() { "V" }
    else { "C" }
}
fn clear() -> Rgba { Rgba::new(0., 0., 0., 0.) }
fn rgba8(c: Rgba) -> [u8; 4] { [(c.r*255.).round() as u8, (c.g*255.).round() as u8, (c.b*255.).round() as u8, (c.a*255.).round() as u8] }
fn rect4(r: Rect) -> [f32; 4] { [r.min_x(), r.min_y(), r.size.width, r.size.height] }
fn scene_quads(scene: &Scene) -> Vec<serde_json::Value> {
    scene.primitives().iter().enumerate().filter_map(|(index,p)| if let Primitive::Quad(q)=p { Some(serde_json::json!({"primitive_index":index,"rect":rect4(q.rect),"fill":rgba8(q.fill),"border":rgba8(q.border_color),"border_width":q.border_width})) } else { None }).collect()
}
fn quad_role_color_collisions(colors: tekes_ui_tokens::surface_palette::DataPlotSurfaceColors) -> Vec<String> {
    let roles=[("F",colors.plot.focus.unwrap()),("M",colors.plot.marker.unwrap()),("S",colors.selection.background),("B",colors.plot.boundary.unwrap())];
    let mut out=vec![]; for i in 0..roles.len() { for j in (i+1)..roles.len() { if roles[i].1==roles[j].1 { out.push(format!("{}={}",roles[i].0,roles[j].0)); } } } out
}
fn text_run_kind() -> String {
    let kind = std::env::var("CHART_THEME_RUN_KIND").expect("CHART_THEME_RUN_KIND=default|external required");
    assert!(kind == "default" || kind == "external");
    kind
}
fn push_channel(scene: &Scene, draws: &[DrawRecord], channel: &str, colors: tekes_ui_tokens::surface_palette::DataPlotSurfaceColors) -> Scene {
    let mut out = Scene::new();
    for (i, prim) in scene.primitives().iter().enumerate() {
        let keep = match prim {
            Primitive::PushClip(_) | Primitive::PopClip => true,
            Primitive::Glyphs(_) => draws.iter().any(|d| d.channel == channel && i >= d.start && i < d.end),
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
                if channel == "F" && q.border_width > 0. && Some(q.border_color) == colors.plot.focus {
                    let mut only = *q; only.fill = clear(); out.push(Primitive::Quad(only)); continue;
                }
                if channel == "M" && Some(q.fill) == colors.plot.marker {
                    let mut only = *q; only.border_width = 0.; out.push(Primitive::Quad(only)); continue;
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
        Primitive::PopClip => "PopClip",
        Primitive::Quad(_) => "Quad",
        Primitive::Shadow(_) => "Shadow",
        Primitive::Image(_) => "Image",
        Primitive::TransformedImage(_) => "TransformedImage",
        _ => "Other",
    }
}

fn glyph_clip_evidence(
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fixture { Positive, Negative, Zero, Narrow, DenseLongLabel, InteriorParentClip, ParentTranslatedVisible }
impl Fixture {
    fn name(self) -> &'static str { match self { Self::Positive=>"positive",Self::Negative=>"negative",Self::Zero=>"zero",Self::Narrow=>"narrow",Self::DenseLongLabel=>"dense-long-label",Self::InteriorParentClip=>"interior-parent-clip-negative",Self::ParentTranslatedVisible=>"parent-translated-visible" } }
}

/// Direct original ChartView GPU evidence. It deliberately has no ContentHost dependency.
/// Run once per separately compiled default/external theme artifact under the shared GPU lock.
#[test]
#[ignore = "run separately for default and external selected theme artifacts under the shared GPU lock"]
fn chart_owner_gpu_104_view_only_matrix() {
    let kind = text_run_kind();
    assert_eq!(surface_names().len(), 7, "#104 frozen surface domain");
    assert_eq!(Theme::ALL.len(), 4, "#104 frozen mode domain");
    let out = std::path::PathBuf::from(std::env::var_os(OUTPUT_ENV).expect("SSD output directory"));
    std::fs::create_dir_all(&out).unwrap();
    let mut renderer = tekes_ui_render::Renderer::new_headless().expect("headless GPU renderer");
    let fixtures = [Fixture::Positive,Fixture::Negative,Fixture::Zero,Fixture::Narrow,Fixture::DenseLongLabel,Fixture::InteriorParentClip,Fixture::ParentTranslatedVisible];
    let mut rows = vec![];
    for fixture in fixtures {
        if fixture != Fixture::Positive && kind == "external" { continue; }
        for (mode_index, mode) in Theme::ALL.into_iter().enumerate() {
            if fixture != Fixture::Positive && mode_index != 0 { continue; }
            let mode_name = ["Light","Dark","LightIncreasedContrast","DarkIncreasedContrast"][mode_index];
            let palette: Palette = if kind == "default" { mode.into() } else { Palette::for_theme_definition(mode,"chart-external").expect("external artifact contains chart-external") };
            for &surface in surface_names() {
                if fixture != Fixture::Positive && surface != "canvas" { continue; }
                let packet = resolve_data_plot_surface(palette,surface).unwrap();
                for backing in [1.0f32,2.0] {
                    let (value,label,count,width,height,clip,translate) = match fixture {
                        Fixture::Positive => (1.0,"ordinary category",2,480.,240.,None,false),
                        Fixture::Negative => (-1.0,"",64,480.,240.,None,false),
                        Fixture::Zero => (0.0,"same",4,480.,240.,None,false),
                        Fixture::Narrow => (1.0,"",64,120.,240.,None,false),
                        Fixture::DenseLongLabel => (1.0,LONG_LABEL,64,480.,240.,None,false),
                        Fixture::InteriorParentClip => (-1.0,"",64,470.,240.,Some(Rect::new(80.,80.,320.,120.)),false),
                        Fixture::ParentTranslatedVisible => (-1.0,"",64,470.,240.,Some(Rect::new(80.,80.,320.,120.)),true),
                    };
                    let mut model = if fixture == Fixture::Positive {
                        BarChart::new(vec![
                            ChartDatum::new("ordinary category",1.0),
                            ChartDatum::new("ordinary category",0.0),
                        ])
                    } else {
                        BarChart::new((0..count).map(|_| ChartDatum::new(label,value)).collect())
                    };
                    if fixture == Fixture::Positive { model.select(0); } else if count > 2 { model.select(2); }
                    let mut text = RecordedText { inner: coretext::Text::new(1.), draws: vec![] };
                    text.inner.backing = backing;
                    let mut view = ChartView::new();
                    let mut frame = if fixture == Fixture::InteriorParentClip { Rect::new(21.,21.,width,height) } else if translate { Rect::new(21.,-40.,width,height) } else { Rect::new(16.,16.,width,height) };
                    if fixture == Fixture::DenseLongLabel {
                        let measured = view.measure_owner(&mut text,&model,width,height);
                        let needed = measured.measured_label_extent.expect("long label measurement").height + 240.;
                        frame = Rect::new(16.,16.,width,needed);
                    }
                    let clip_rect = clip;
                    let capacity = view.arrange_owner(&mut text,&mut model,frame,clip_rect);
                    let initial_space = view.owner_focus_space(&model);
                    let visible = matches!(initial_space,tekes_ui_controls::collections_view::ChartOwnerFocusSpace::Visible{..});
                    if fixture == Fixture::InteriorParentClip {
                        assert_eq!(initial_space,tekes_ui_controls::collections_view::ChartOwnerFocusSpace::NoVisibleWitness,"interior clip remains a negative owner witness");
                        assert!(view.try_focus_owner(true,&mut model).is_err(),"interior clip must not manufacture focus");
                    } else {
                        assert!(visible,"original visible owner fixture lost its witness: {} / {mode_name} / {surface} / {backing}",fixture.name());
                        view.try_focus_owner(true,&mut model).expect("visible original owner focuses");
                    }
                    let focus_space = view.owner_focus_space(&model);
                    let mut focus_scene = Scene::new();
                    let focus_painted = view.paint_owner_on_surface(&mut focus_scene,&mut text,&mut model,palette,surface).unwrap();
                    let size = ((600. * backing) as u32,(700. * backing) as u32);
                    let ground = resolve_surface(palette,surface,"content","rest").unwrap().background;
                    let focus_pixels = renderer.render_to_rgba8(&focus_scene,&mut text.inner.atlas,size,backing,ground).unwrap();
                    let key = format!("{}-{mode_name}-{surface}-{}x",fixture.name(),backing as u32);
                    std::fs::write(out.join(format!("{key}-view.rgba8")),&focus_pixels).unwrap();
                    let focus_clip_ev = glyph_clip_evidence(&focus_scene,backing,size,text.inner.atlas.page_count());
                    let focus_draws = std::mem::take(&mut text.draws);
                    let focus_quads = scene_quads(&focus_scene);
                    for channel in ["F","V","C","T","H","M","S","B"] {
                        let isolated = push_channel(&focus_scene,&focus_draws,channel,packet);
                        let pixels = renderer.render_to_rgba8(&isolated,&mut text.inner.atlas,size,backing,clear()).unwrap();
                        std::fs::write(out.join(format!("{key}-view-{channel}.rgba8")),pixels).unwrap();
                    }
                    let focused_before_blur = model.is_focused();
                    let selection_before_blur = model.selected();
                    let data_before_blur = model.data().to_vec();
                    view.blur(&mut model);
                    assert!(!model.is_focused(), "real ChartView::blur must clear focus");
                    assert_eq!(model.selected(), selection_before_blur);
                    assert_eq!(model.data(), data_before_blur);
                    let mut blur_scene = Scene::new();
                    let blur_painted = view.paint_owner_on_surface(&mut blur_scene,&mut text,&mut model,palette,surface).unwrap();
                    let blur_pixels = renderer.render_to_rgba8(&blur_scene,&mut text.inner.atlas,size,backing,ground).unwrap();
                    std::fs::write(out.join(format!("{key}-view-blur.rgba8")),&blur_pixels).unwrap();
                    let blur_clip_ev = glyph_clip_evidence(&blur_scene,backing,size,text.inner.atlas.page_count());
                    let blur_draws = std::mem::take(&mut text.draws);
                    let blur_quads = scene_quads(&blur_scene);
                    for channel in ["V","C","T","H"] {
                        let isolated = push_channel(&blur_scene,&blur_draws,channel,packet);
                        let pixels = renderer.render_to_rgba8(&isolated,&mut text.inner.atlas,size,backing,clear()).unwrap();
                        std::fs::write(out.join(format!("{key}-view-blur-{channel}.rgba8")),pixels).unwrap();
                    }
                    let effective_clip = clip_rect.unwrap_or(frame);
                    let records = |draws: &[DrawRecord]| draws.iter().map(|d|serde_json::json!({"role":d.role,"text":d.text,"channel":d.channel,"rgba":d.rgba,"frame":rect4(d.frame),"start":d.start,"end":d.end})).collect::<Vec<_>>();
                    let colors = serde_json::json!({"ground":rgba8(ground),"plot_focus":packet.plot.focus.map(rgba8),"plot_marker":packet.plot.marker.map(rgba8),"selection_bg":rgba8(packet.selection.background),"plot_boundary":packet.plot.boundary.map(rgba8),"plot_text":rgba8(packet.plot.text),"category_text":rgba8(packet.category.text)});
                    let role_color_collisions = quad_role_color_collisions(packet);
                    rows.push(serde_json::json!({
                        "key":key,"run_kind":kind,"fixture":fixture.name(),"mode":mode_name,"theme_debug":format!("{mode:?}"),"theme_id":if kind=="default"{"default"}else{"chart-external"},"surface":surface,"backing":backing,"size_px":[size.0,size.1],"frame":rect4(frame),"clip":rect4(effective_clip),"capacity":format!("{capacity:?}"),"required_focus_result":format!("{focus_space:?}"),"view_blur_focus_result":format!("{:?}",view.owner_focus_space(&model)),"focus_painted":format!("{focus_painted:?}"),"blur_painted":format!("{blur_painted:?}"),"focused_before_blur":focused_before_blur,"focused_after_blur":model.is_focused(),"blur_method":"ChartView::blur","view_blur_selected":model.selected(),"view_blur_data_count":model.data().len(),"selected":model.selected(),"data_count":model.data().len(),"focus_width":tekes_ui_tokens::proposal::metric("stroke","c-focus-width").unwrap(),"colors":colors,"ground":rgba8(ground),"plot":{"background":rgba8(packet.plot.background),"text":rgba8(packet.plot.text),"focus":packet.plot.focus.map(rgba8),"marker":packet.plot.marker.map(rgba8),"boundary":packet.plot.boundary.map(rgba8)},"category":{"background":rgba8(packet.category.background),"text":rgba8(packet.category.text)},"selection_background":rgba8(packet.selection.background),"quad_role_color_collisions_diagnostic":role_color_collisions,"view_focus_clip_evidence":focus_clip_ev,"view_blur_clip_evidence":blur_clip_ev,"view_text_draws":records(&focus_draws),"view_blur_text_draws":records(&blur_draws),"view_quads":focus_quads,"view_blur_quads":blur_quads
                    }));
                }
            }
        }
    }
    let expected_rows = if kind == "default" { 56 + 6*2 } else { 56 };
    assert_eq!(rows.len(),expected_rows,"preserve full #104 matrix plus default-only original structural probes");
    std::fs::write(out.join("lane.json"),serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
}
