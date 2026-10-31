//! Issue86: actual production DataTableView and Content, no QA ink substitution.
use anyhow::{Result, ensure};
use serde_json::json;
use std::path::Path;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, collections_view::DataTableView, composition::*,
    extended::*,
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
    font_scale: f32,
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
            font_scale: 1.,
            draws: 0,
            packets: vec![],
        }
    }
    fn paragraph(&mut self, s: &str, r: proposal::TextRole, w: f32) -> tekes_ui_text::Paragraph {
        let r = r.style();
        self.shaper.shape(
            s,
            &TextStyle::system(r.size * self.font_scale).with_weight(r.weight),
            &ShapeOptions::with_width(w.max(1.) as f64),
        )
    }
}
impl ButtonTextRenderer for Labels {
    fn resolved_font_size(&mut self, r: proposal::TextRole) -> f32 {
        r.style().size * self.font_scale
    }
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
fn snapshot() -> KeyedTableSnapshot {
    KeyedTableSnapshot {
        dataset: TableDatasetKey(1),
        columns: (1..=2)
            .map(|k| KeyedTableColumn {
                key: TableColumnKey(k),
                name: format!("column{k}"),
            })
            .collect(),
        rows: (1..=3)
            .map(|k| KeyedTableRow {
                key: TableRowKey(k),
                name: format!("row{k}"),
                cells: vec![format!("value{k}"), "cell".into()],
            })
            .collect(),
        display_rows: (1..=3).map(TableRowKey).collect(),
    }
}
fn displays(s: &KeyedTableSnapshot) -> TableDisplays {
    TableDisplays {
        headers: s
            .columns
            .iter()
            .map(|c| TableHeaderDisplay {
                key: c.key,
                display: TableDisplayUpdate::Keep,
            })
            .collect(),
        rows: s
            .rows
            .iter()
            .map(|r| TableRowDisplay {
                key: r.key,
                id: ComponentId(100 + r.key.0),
                replace_wrapper: false,
                cells: s
                    .columns
                    .iter()
                    .map(|c| TableCellDisplay {
                        column: c.key,
                        display: TableDisplayUpdate::Keep,
                        accessory: TableDisplayUpdate::Keep,
                    })
                    .collect(),
            })
            .collect(),
    }
}

const ID: ComponentId = ComponentId(10);
fn content(custom: bool) -> DataTableContent {
    let mut s = snapshot();
    if custom {
        s.columns[0].name = "Long header 表头".into();
        s.rows[0].cells[0] = "Passive text\nsecond line\nthird line".into();
    }
    let mut d = displays(&s);
    if custom {
        d.rows[0].cells[0].display = TableDisplayUpdate::Replace(Content::new(
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
                        TextContent::new("Passive text\nsecond line\nthird line"),
                    ),
                ],
            },
        ));
        d.rows[1].cells[1].accessory = TableDisplayUpdate::Replace(Content::new(
            ComponentId(204),
            IconContent::new("tekes-settings").unwrap(),
        ));
    }
    DataTableContent::new(
        "Table",
        &s,
        &mut d,
        TableBudget {
            max_units: 100_000,
            max_utf8_bytes: 1_000_000,
        },
    )
    .unwrap()
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

fn chrome_layer(scene: &Scene) -> Scene {
    let mut result = Scene::new();
    for p in scene.primitives() {
        if !matches!(
            p,
            Primitive::Glyphs(_) | Primitive::Image(_) | Primitive::TransformedImage(_)
        ) {
            result.push(p.clone());
        }
    }
    result
}
fn role_part(h: &ContentHost, role: ContentRole) -> Option<usize> {
    h.semantics()
        .into_iter()
        .find(|n| n.id == ID)?
        .parts
        .into_iter()
        .find(|p| p.semantics.role == role)
        .map(|p| p.index)
}
fn main() -> Result<()> {
    let out =
        std::path::PathBuf::from(std::env::var_os("TABLE_QA_OUTPUT").expect("explicit output"));
    std::fs::create_dir_all(&out)?;
    let mut gpu = Renderer::new_headless()?;
    let mut cases = vec![];
    let mut failures = vec![];
    for (mode, t) in Theme::ALL.into_iter().enumerate() {
        let p = match std::env::var("TABLE_QA_THEME").ok() {
            Some(id) => Palette::for_theme_definition(t, &id).expect("formal external Theme"),
            None => t.into(),
        };
        let bg = p.color("surface-1").unwrap();
        for scale in [1., 2.] {
            for font in [1., 1.35] {
                let size = ((360. * scale) as u32, (260. * scale) as u32);
                let empty = gpu.render_to_rgba8(
                    &Scene::new(),
                    &mut GlyphAtlas::default(),
                    size,
                    scale,
                    bg,
                )?;
                for case in ["normal", "narrow", "ancestorclip", "zero"] {
                    let f = match case {
                        "zero" => Rect::new(16., 16., 0., 220.),
                        "narrow" => Rect::new(16., 16., 90., 220.),
                        _ => Rect::new(16., 16., 320., 220.),
                    };
                    let clip = if case == "ancestorclip" {
                        Rect::new(39., 62., 160., 75.)
                    } else {
                        f
                    };
                    let name = format!("m{mode}-{scale}x-font{font}-{case}");
                    let mut model = KeyedDataTable::new(
                        snapshot(),
                        TableBudget {
                            max_units: 100_000,
                            max_utf8_bytes: 1_000_000,
                        },
                    )
                    .unwrap();
                    model.sort_key(TableColumnKey(1));
                    let mut view = DataTableView::new();
                    let mut text = Labels::new(scale);
                    text.font_scale = font;
                    let mut scene = Scene::new();
                    view.render_keyed(&mut scene, &mut text, &model, f, Some(clip), p);
                    let view_rest =
                        gpu.render_to_rgba8(&scene, &mut text.atlas, size, scale, bg)?;
                    view.focus_header_keyed(TableColumnKey(1), true, &model);
                    scene.clear();
                    text.atlas.begin_frame();
                    let layout = view.render_keyed(&mut scene, &mut text, &model, f, Some(clip), p);
                    let original = gpu.render_to_rgba8(&scene, &mut text.atlas, size, scale, bg)?;
                    let view_focus_channels = view_rest
                        .iter()
                        .zip(&original)
                        .filter(|(a, b)| a != b)
                        .count();
                    if case == "normal" && view_focus_channels == 0 {
                        failures.push(format!("{name}: no actual production View focus ink"));
                    }
                    save(&out, &format!("{name}-view-rest"), &view_rest, size)?;
                    let view_focus_mask = view_rest
                        .chunks_exact(4)
                        .zip(original.chunks_exact(4))
                        .map(|(a, b)| u8::from(a != b))
                        .collect::<Vec<_>>();
                    std::fs::write(
                        out.join(format!("{name}-view-focus.mask")),
                        &view_focus_mask,
                    )?;

                    save(&out, &format!("{name}-view"), &original, size)?;
                    let reference_packets = text.packets.clone();
                    let view_glyphs = gpu.render_to_rgba8(
                        &layer(&scene, true),
                        &mut text.atlas,
                        size,
                        scale,
                        bg,
                    )?;
                    let view_chrome = gpu.render_to_rgba8(
                        &chrome_layer(&scene),
                        &mut text.atlas,
                        size,
                        scale,
                        bg,
                    )?;
                    save(&out, &format!("{name}-view-glyphs"), &view_glyphs, size)?;
                    save(&out, &format!("{name}-view-chrome"), &view_chrome, size)?;
                    let mut h = ContentHost::new(Content::new(ID, content(false)))?;
                    let mut ht = Labels::new(scale);
                    ht.font_scale = font;
                    let mut hs = Scene::new();
                    h.render(&mut hs, &mut ht, f, clip, p)?;
                    if case == "normal" || case == "narrow" {
                        let i = role_part(
                            &h,
                            ContentRole::TableColumnHeader {
                                index: 0,
                                sort: None,
                            },
                        )
                        .unwrap();
                        ensure!(
                            h.accessibility_action(ID, Some(i), AccessibilityAction::Press)
                                .consumed
                        );
                        h.focus(Some(ID), true);
                    } else {
                        // The owner remains authoritative even when the header is clipped.
                        let full = Rect::new(16., 16., 320., 220.);
                        h.render(&mut hs, &mut ht, full, full, p)?;
                        let i = role_part(
                            &h,
                            ContentRole::TableColumnHeader {
                                index: 0,
                                sort: None,
                            },
                        )
                        .unwrap();
                        ensure!(
                            h.accessibility_action(ID, Some(i), AccessibilityAction::Press)
                                .consumed
                        );
                        h.focus(Some(ID), true);
                    }
                    hs.clear();
                    ht.atlas.begin_frame();
                    h.render(&mut hs, &mut ht, f, clip, p)?;
                    let mounted = gpu.render_to_rgba8(&hs, &mut ht.atlas, size, scale, bg)?;
                    save(&out, &format!("{name}-host"), &mounted, size)?;
                    let delta = original
                        .iter()
                        .zip(&mounted)
                        .filter(|(a, b)| a != b)
                        .count();
                    let parts = h
                        .semantics()
                        .into_iter()
                        .find(|n| n.id == ID)
                        .map(|n| n.parts)
                        .unwrap_or_default();
                    for part in &parts {
                        let expected = match part.semantics.role {
                            ContentRole::TableRow { index } => layout.rows.get(index).map(|r| {
                                r.visible.unwrap_or(Rect::new(
                                    r.frame.min_x(),
                                    r.frame.min_y(),
                                    0.,
                                    0.,
                                ))
                            }),
                            ContentRole::TableColumnHeader { index, .. } => {
                                layout.headers.get(index).map(|r| {
                                    r.visible.unwrap_or(Rect::new(
                                        r.frame.min_x(),
                                        r.frame.min_y(),
                                        0.,
                                        0.,
                                    ))
                                })
                            }
                            _ => None,
                        };
                        if let Some(expected) = expected
                            && part.frame != expected
                        {
                            failures.push(format!(
                                "{name}: geometry mismatch {:?} {:?}/{expected:?}",
                                part.semantics.role, part.frame
                            ));
                        }
                    }
                    let (positive, outside) = ink(&mounted, &empty, size, clip, scale);
                    let (_, reference_outside) = ink(&original, &empty, size, clip, scale);
                    if delta != 0
                        || outside != 0
                        || reference_outside != 0
                        || (case != "zero" && positive == 0)
                    {
                        failures.push(format!("{name}: delta={delta} ink={positive} outside={outside}/{reference_outside}"));
                    }
                    let mut custom = ContentHost::new(Content::new(ID, content(true)))?;
                    let mut ct = Labels::new(scale);
                    ct.font_scale = font;
                    let mut cs = Scene::new();
                    let mut cache = SvgIconCache::default();
                    let mut packets = vec![];
                    custom.render(
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
                    let rest = gpu.render_to_rgba8(&cs, &mut ct.atlas, size, scale, bg)?;
                    custom.focus(Some(ID), true);
                    cs.clear();
                    ct.atlas.begin_frame();
                    custom.render(
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
                    let custom_pixels = gpu.render_to_rgba8(&cs, &mut ct.atlas, size, scale, bg)?;
                    save(&out, &format!("{name}-custom"), &custom_pixels, size)?;
                    let (cp, co) = ink(&custom_pixels, &empty, size, clip, scale);
                    let glyph =
                        gpu.render_to_rgba8(&layer(&cs, true), &mut ct.atlas, size, scale, bg)?;
                    let icon =
                        gpu.render_to_rgba8(&layer(&cs, false), &mut ct.atlas, size, scale, bg)?;
                    let gi = ink(&glyph, &empty, size, clip, scale).0;
                    let ii = ink(&icon, &empty, size, clip, scale).0;
                    let focus = rest
                        .iter()
                        .zip(&custom_pixels)
                        .filter(|(a, b)| a != b)
                        .count();
                    if co != 0
                        || (case != "zero" && cp == 0)
                        || (case == "normal" && (gi == 0 || ii == 0 || focus == 0))
                    {
                        failures.push(format!("{name}: custom ink={cp} outside={co} glyph={gi} icon={ii} focus={focus}"));
                    }
                    if case == "normal" {
                        save(&out, &format!("{name}-glyph"), &glyph, size)?;
                        save(&out, &format!("{name}-icon"), &icon, size)?;
                        save(&out, &format!("{name}-rest"), &rest, size)?;
                    }
                    let mut leaked = Scene::new();
                    leaked.push_quad(Quad::new(
                        Rect::new(1., 1., 4., 4.),
                        p.color("primary-text").unwrap(),
                    ));
                    let leak =
                        gpu.render_to_rgba8(&leaked, &mut GlyphAtlas::default(), size, scale, bg)?;
                    ensure!(ink(&leak, &empty, size, clip, scale).1 > 0);
                    ensure!(ink(&empty, &empty, size, clip, scale).0 == 0);
                    if mode == 0 && font == 1. && scale == 1. && case == "normal" {
                        save(&out, "negative-overflow", &leak, size)?;
                        save(&out, "negative-blank", &empty, size)?;
                    }
                    cases.push(json!({"case":name,"theme":format!("{t:?}"),"palette":format!("{:?}",p.source()),"default_channel_delta":delta,"production_view_text_packets":reference_packets,"production_view_focus_channels":view_focus_channels,"ink":positive,"outside":outside,"custom_ink":cp,"custom_outside":co,"glyph_ink":gi,"icon_ink":ii,"focus_channels":focus,"reference_layout":format!("{layout:?}"),"header_sort_state":format!("{:?}",h.get(ID).unwrap().component::<DataTableContent>().unwrap().sort_key_state()),"custom_frames":([201,202,203,204].map(|i|custom.get(ComponentId(i)).map(|n|format!("{:?}",n.frame())))),"default_geometry":format!("{:?}",h.semantics()),"custom_geometry":format!("{:?}",custom.semantics()),"default_text_packets":ht.packets,"custom_text_packets":ct.packets,"icon_packets":packets}));
                }
            }
        }
    }
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"cases":cases,"failures":failures,"raw_reference_parity_required_for_this_runner":true,"candidate_scope":"Approved legacy foreground change only: raw production View versus Content is the strict acceptance oracle; no QA ink substitution."}),
        )?,
    )?;
    ensure!(failures.is_empty(), "actual GPU failures: {failures:?}");
    println!("TABLE_GPU_PASS cases={}", cases.len());
    Ok(())
}
