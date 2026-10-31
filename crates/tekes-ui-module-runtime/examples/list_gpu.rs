//! Actual GPU readback, same public TreeView/TreeContent and services.
use anyhow::{Result, ensure};
use serde_json::json;
use std::path::Path;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, collections_view::ListView, composition::*, extended::*,
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, Point, Primitive, Quad, Rect, Scene, Size};
use tekes_ui_text::{
    ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
    push_paragraph,
};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
struct TraceIcons<'a> {
    inner: &'a mut dyn tekes_ui_controls::layout_view::IconRenderer,
    packets: &'a mut Vec<serde_json::Value>,
}
impl tekes_ui_controls::layout_view::IconRenderer for TraceIcons<'_> {
    fn draw_icon(&mut self, s: &mut Scene, n: &str, f: Rect, c: Rgba) {
        self.packets.push(json!({"call":"draw_icon","name":n,"frame":[f.min_x(),f.min_y(),f.size.width,f.size.height],"tint":[c.r,c.g,c.b,c.a]}));
        self.inner.draw_icon(s, n, f, c)
    }
    fn prepare_icon(
        &mut self,
        n: &str,
        z: f32,
        b: f32,
        c: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        self.packets.push(
            json!({"call":"prepare_icon","name":n,"size":z,"backing":b,"tint":[c.r,c.g,c.b,c.a]}),
        );
        self.inner.prepare_icon(n, z, b, c)
    }
}
struct Labels {
    shaper: CoreTextShaper,
    raster: CoreTextRasterizer,
    atlas: GlyphAtlas,
    scale: f32,
    draws: usize,
    packets: Vec<serde_json::Value>,
}
impl Labels {
    fn new(scale: f32) -> Self {
        let f = CoreTextFonts::new();
        Self {
            shaper: CoreTextShaper::new(f.clone()),
            raster: CoreTextRasterizer::new(f),
            atlas: GlyphAtlas::default(),
            scale,
            draws: 0,
            packets: vec![],
        }
    }
    fn paragraph(&mut self, s: &str, r: proposal::TextRole, w: f32) -> tekes_ui_text::Paragraph {
        let r = r.style();
        self.shaper.shape(
            s,
            &TextStyle::system(r.size).with_weight(r.weight),
            &ShapeOptions::with_width(w.max(1.) as f64),
        )
    }
}
impl ButtonTextRenderer for Labels {
    fn backing_scale(&self) -> f32 {
        self.scale
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        if w <= 0. {
            return Size::default();
        }
        let p = self.paragraph(s, r, w);
        Size::new((p.width as f32).min(w), p.height as f32)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        s: &str,
        r: proposal::TextRole,
        f: Rect,
        a: ButtonTextAlign,
        ink: Rgba,
    ) {
        if f.is_empty() {
            return;
        }
        self.packets.push(json!({"text":s,"role":format!("{r:?}"),"frame":[f.min_x(),f.min_y(),f.size.width,f.size.height],"color":[ink.r,ink.g,ink.b,ink.a]}));
        let p = self.paragraph(s, r, f.size.width);
        let x = if a == ButtonTextAlign::Center {
            f.min_x() + (f.size.width - p.width as f32) * 0.5
        } else {
            f.min_x()
        };
        let y = f.min_y() + (f.size.height - p.height as f32) * 0.5;
        scene.push_clip(f);
        push_paragraph(
            scene,
            &mut self.atlas,
            &mut self.raster,
            &p,
            Point::new(x.round(), y.round()),
            self.scale,
            ink,
        );
        scene.pop_clip();
        self.draws += 1;
    }
}
fn snapshot() -> KeyedListSnapshot {
    KeyedListSnapshot {
        dataset: ListDatasetKey(1),
        items: (1..=3)
            .map(|k| KeyedListItem {
                key: ListItemKey(k),
                name: format!("row{k}"),
            })
            .collect(),
        display_items: (1..=3).map(ListItemKey).collect(),
    }
}
fn rows(s: &KeyedListSnapshot) -> Vec<ListRowDisplay> {
    s.items
        .iter()
        .map(|r| ListRowDisplay {
            key: r.key,
            row_id: ComponentId(100 + r.key.0),
            display: ListDisplayUpdate::Keep,
        })
        .collect()
}
fn budget() -> ListBudget {
    ListBudget {
        max_items: 100,
        max_display_items: 100,
        max_utf8_bytes: 1000000,
        max_staging_units: 100000,
    }
}
fn list(s: &KeyedListSnapshot) -> KeyedSelectableListContent {
    SelectableListContent::keyed("L", s, &mut rows(s), budget()).unwrap()
}
fn save(dir: &Path, name: &str, pixels: &[u8], size: (u32, u32)) -> Result<()> {
    std::fs::write(dir.join(format!("{name}.rgba")), pixels)?;
    let mut e = png::Encoder::new(
        std::fs::File::create(dir.join(format!("{name}.png")))?,
        size.0,
        size.1,
    );
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header()?.write_image_data(pixels)?;
    Ok(())
}
fn ink(p: &[u8], empty: &[u8], size: (u32, u32), clip: Rect, scale: f32) -> (usize, usize) {
    let mut inside = 0;
    let mut outside = 0;
    for (i, (a, b)) in p.chunks_exact(4).zip(empty.chunks_exact(4)).enumerate() {
        if a == b {
            continue;
        }
        let x = (i % size.0 as usize) as f32;
        let y = (i / size.0 as usize) as f32;
        let within = x >= (clip.min_x() * scale).floor()
            && x < (clip.max_x() * scale).ceil()
            && y >= (clip.min_y() * scale).floor()
            && y < (clip.max_y() * scale).ceil();
        if within { inside += 1 } else { outside += 1 }
    }
    (inside, outside)
}
fn layer(scene: &Scene, glyph: bool) -> Scene {
    let mut result = Scene::new();
    for p in scene.primitives() {
        if matches!(p, Primitive::PushClip(_) | Primitive::PopClip)
            || if glyph {
                matches!(p, Primitive::Glyphs(_))
            } else {
                matches!(p, Primitive::Image(_) | Primitive::TransformedImage(_))
            }
        {
            result.push(p.clone());
        }
    }
    result
}

const ID: ComponentId = ComponentId(10);
fn custom() -> KeyedSelectableListContent {
    let s = snapshot();
    let mut d = rows(&s);
    d[0].display = ListDisplayUpdate::Replace(Content::new(
        ComponentId(201),
        StackContent {
            model: Stack {
                axis: Axis::Horizontal,
                gap: 6.,
            },
            flexible_child: Some(ComponentId(203)),
            children: vec![
                Content::new(
                    ComponentId(202),
                    IconContent::new("tekes-settings").unwrap(),
                ),
                Content::new(
                    ComponentId(203),
                    TextContent::new("Passive text 文本\nsecond long line\nthird line"),
                ),
            ],
        },
    ));
    SelectableListContent::keyed("List", &s, &mut d, budget()).unwrap()
}
fn main() -> Result<()> {
    let out =
        std::path::PathBuf::from(std::env::var_os("LIST_QA_OUTPUT").expect("explicit output"));
    std::fs::create_dir_all(&out)?;
    let mut gpu = Renderer::new_headless()?;
    let mut cases = vec![];
    let mut failures = vec![];
    for (mode, t) in Theme::ALL.into_iter().enumerate() {
        let p = match std::env::var("LIST_QA_THEME").ok() {
            Some(id) => Palette::for_theme_definition(t, &id).expect("formal external Theme"),
            None => t.into(),
        };
        let bg = p.color("surface-1").unwrap();
        for scale in [1., 2.] {
            let size = ((360. * scale) as u32, (260. * scale) as u32);
            let empty =
                gpu.render_to_rgba8(&Scene::new(), &mut GlyphAtlas::default(), size, scale, bg)?;
            for case in ["normal", "narrow", "ancestorclip", "zero"] {
                let f = match case {
                    "zero" => Rect::new(16., 16., 0., 220.),
                    "narrow" => Rect::new(16., 16., 90., 220.),
                    _ => Rect::new(16., 16., 320., 220.),
                };
                let clip = if case == "ancestorclip" {
                    Rect::new(39., 42., 160., 75.)
                } else {
                    f
                };
                for state in ["rest", "selected", "focus"] {
                    let name = format!("m{mode}-{scale}x-{case}-{state}");
                    let labels = snapshot()
                        .items
                        .into_iter()
                        .map(|i| i.name)
                        .collect::<Vec<_>>();
                    let selected = (state != "rest").then_some(0);
                    let model = List { count: 3, selected };
                    let mut view = ListView::new();
                    let mut text = Labels::new(scale);
                    let mut scene = Scene::new();
                    view.render(&mut scene, &mut text, &model, &labels, f, Some(clip), p);
                    if state == "focus" {
                        view.focus(0, true);
                    }
                    scene.clear();
                    text.atlas.begin_frame();
                    let layout =
                        view.render(&mut scene, &mut text, &model, &labels, f, Some(clip), p);
                    let original = gpu.render_to_rgba8(&scene, &mut text.atlas, size, scale, bg)?;
                    let mut legacy = SelectableListContent::new("List", labels.clone());
                    legacy.model.selected = selected;
                    let mut lh = ContentHost::new(Content::new(ID, legacy))?;
                    let mut lt = Labels::new(scale);
                    let mut ls = Scene::new();
                    lh.render(&mut ls, &mut lt, f, clip, p)?;
                    if state == "focus" {
                        lh.focus(Some(ID), true);
                    }
                    ls.clear();
                    lt.atlas.begin_frame();
                    lh.render(&mut ls, &mut lt, f, clip, p)?;
                    let legacy_pixels = gpu.render_to_rgba8(&ls, &mut lt.atlas, size, scale, bg)?;
                    let mut c = list(&snapshot());
                    if selected.is_some() {
                        c.select_key(ListItemKey(1)).unwrap();
                    }
                    let mut h = ContentHost::new(Content::new(ID, c))?;
                    let mut ht = Labels::new(scale);
                    let mut hs = Scene::new();
                    h.render(&mut hs, &mut ht, f, clip, p)?;
                    if state == "focus" {
                        h.focus(Some(ID), true);
                    }
                    hs.clear();
                    ht.atlas.begin_frame();
                    h.render(&mut hs, &mut ht, f, clip, p)?;
                    let mounted = gpu.render_to_rgba8(&hs, &mut ht.atlas, size, scale, bg)?;
                    save(&out, &format!("{name}-view"), &original, size)?;
                    save(&out, &format!("{name}-legacy"), &legacy_pixels, size)?;
                    save(&out, &format!("{name}-keyed"), &mounted, size)?;
                    let delta = original
                        .iter()
                        .zip(&mounted)
                        .filter(|(a, b)| a != b)
                        .count();
                    let legacy_delta = original
                        .iter()
                        .zip(&legacy_pixels)
                        .filter(|(a, b)| a != b)
                        .count();
                    let (positive, outside) = ink(&mounted, &empty, size, clip, scale);
                    let (_, vo) = ink(&original, &empty, size, clip, scale);
                    if delta != 0
                        || legacy_delta != 0
                        || outside != 0
                        || vo != 0
                        || (case != "zero" && positive == 0)
                    {
                        failures.push(format!("{name}: delta={delta}/{legacy_delta} ink={positive} outside={outside}/{vo}"));
                    }
                    let mut c = custom();
                    if selected.is_some() {
                        c.select_key(ListItemKey(1)).unwrap();
                    }
                    let mut ch = ContentHost::new(Content::new(ID, c))?;
                    let mut ct = Labels::new(scale);
                    let mut cs = Scene::new();
                    let mut cache = SvgIconCache::default();
                    let mut packets = vec![];
                    ch.render(
                        &mut cs,
                        &mut WithIconRenderer::new(
                            &mut ct,
                            &mut TraceIcons {
                                inner: &mut cache.renderer(&mut gpu, scale),
                                packets: &mut packets,
                            },
                        ),
                        f,
                        clip,
                        p,
                    )?;
                    let unfocused = gpu.render_to_rgba8(&cs, &mut ct.atlas, size, scale, bg)?;
                    if state == "focus" {
                        ch.focus(Some(ID), true);
                    }
                    cs.clear();
                    ct.atlas.begin_frame();
                    ch.render(
                        &mut cs,
                        &mut WithIconRenderer::new(
                            &mut ct,
                            &mut TraceIcons {
                                inner: &mut cache.renderer(&mut gpu, scale),
                                packets: &mut packets,
                            },
                        ),
                        f,
                        clip,
                        p,
                    )?;
                    let pixels = gpu.render_to_rgba8(&cs, &mut ct.atlas, size, scale, bg)?;
                    save(&out, &format!("{name}-custom"), &pixels, size)?;
                    let (cp, co) = ink(&pixels, &empty, size, clip, scale);
                    let glyph =
                        gpu.render_to_rgba8(&layer(&cs, true), &mut ct.atlas, size, scale, bg)?;
                    let icon =
                        gpu.render_to_rgba8(&layer(&cs, false), &mut ct.atlas, size, scale, bg)?;
                    let gi = ink(&glyph, &empty, size, clip, scale).0;
                    let ii = ink(&icon, &empty, size, clip, scale).0;
                    let focus = unfocused
                        .iter()
                        .zip(&pixels)
                        .filter(|(a, b)| a != b)
                        .count();
                    if co != 0
                        || (case != "zero" && cp == 0)
                        || (case == "normal"
                            && (gi == 0 || ii == 0 || (state == "focus" && focus == 0)))
                    {
                        failures.push(format!("{name}: custom ink={cp} outside={co} glyph={gi} icon={ii} focus={focus}"));
                    }
                    if case == "normal" {
                        save(&out, &format!("{name}-glyph"), &glyph, size)?;
                        save(&out, &format!("{name}-icon"), &icon, size)?;
                    }
                    cases.push(json!({"case":name,"theme":format!("{t:?}"),"palette":format!("{:?}",p.source()),"default_channel_delta":delta,"legacy_channel_delta":legacy_delta,"ink":positive,"outside":outside,"custom_ink":cp,"custom_outside":co,"glyph_ink":gi,"icon_ink":ii,"focus_channels":focus,"layout":format!("{layout:?}"),"keyed_semantics":format!("{:?}",h.semantics()),"custom_semantics":format!("{:?}",ch.semantics()),"text_packets":ht.packets,"custom_text_packets":ct.packets,"icon_packets":packets}));
                }
            }
        }
    }
    // Negative controls for non-blank and overflow detectors.
    let size = (360, 260);
    let p: Palette = Theme::LIGHT.into();
    let bg = p.color("surface-1").unwrap();
    let blank = gpu.render_to_rgba8(&Scene::new(), &mut GlyphAtlas::default(), size, 1., bg)?;
    let mut leak = Scene::new();
    leak.push_quad(Quad::new(
        Rect::new(1., 1., 4., 4.),
        p.color("primary-text").unwrap(),
    ));
    let overflow = gpu.render_to_rgba8(&leak, &mut GlyphAtlas::default(), size, 1., bg)?;
    ensure!(ink(&blank, &blank, size, Rect::new(16., 16., 320., 220.), 1.).0 == 0);
    ensure!(ink(&overflow, &blank, size, Rect::new(16., 16., 320., 220.), 1.).1 > 0);
    save(&out, "negative-blank", &blank, size)?;
    save(&out, "negative-overflow", &overflow, size)?;
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&json!({"cases":cases,"failures":failures}))?,
    )?;
    ensure!(failures.is_empty(), "actual GPU failures {failures:?}");
    println!("LIST_GPU_PASS cases={}", cases.len());
    Ok(())
}
