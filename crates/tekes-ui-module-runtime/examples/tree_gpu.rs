//! Actual GPU readback, same public TreeView/TreeContent and services.
use anyhow::{Result, ensure};
use serde_json::json;
use std::path::Path;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    collections_view::TreeView,
    composition::*,
    extended::{Axis, KeyedTreeNode, Stack, Tree, TreeDatasetKey as D, TreeNodeKey as K},
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
fn model_nodes() -> Vec<KeyedTreeNode> {
    vec![
        (1, None, "Root 分支"),
        (2, Some(1), "Child detail"),
        (3, Some(2), "Grandchild"),
        (4, None, "Leaf"),
    ]
    .into_iter()
    .map(|(k, p, s)| KeyedTreeNode {
        key: K(k),
        parent: p.map(K),
        label: s.into(),
        disabled: false,
        initially_expanded: true,
    })
    .collect()
}
fn content(custom: bool) -> TreeContent {
    let mut nodes: Vec<_> = model_nodes()
        .into_iter()
        .map(|n| TreeContentNode {
            row_id: ComponentId(100 + n.key.0),
            node: n,
            display: TreeDisplayUpdate::Keep,
        })
        .collect();
    if custom {
        nodes[0].display = TreeDisplayUpdate::Replace(Content::new(
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
                    Content::new(ComponentId(203), TextContent::new("Passive 文本")),
                ],
            },
        ));
    }
    let mut c = TreeContent::new("Tree", D(1), nodes).unwrap();
    c.select_key(K(1));
    c
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
fn main() -> Result<()> {
    let out =
        std::path::PathBuf::from(std::env::var_os("TREE_QA_OUTPUT").expect("explicit output"));
    std::fs::create_dir_all(&out)?;
    let theme_id = std::env::var("TREE_QA_THEME").ok();
    let mut gpu = Renderer::new_headless()?;
    let mut cases = vec![];
    let mut failures = vec![];
    let attribution = std::env::var_os("TREE_QA_ATTRIBUTION").is_some();
    for (mode, t) in Theme::ALL.into_iter().enumerate() {
        if attribution && mode != 0 {
            continue;
        }
        let p = match theme_id.as_deref() {
            Some(id) => {
                Palette::for_theme_definition(t, id).expect("compiled formal external Theme")
            }
            None => t.into(),
        };
        let bg = p.color("surface-1").unwrap();
        for scale in [1., 2.] {
            if attribution && scale != 1. {
                continue;
            }
            let size = ((360. * scale) as u32, (240. * scale) as u32);
            let empty =
                gpu.render_to_rgba8(&Scene::new(), &mut GlyphAtlas::default(), size, scale, bg)?;
            for case in ["normal", "zero", "narrow", "fractional", "ancestorclip"] {
                if attribution && case != "normal" {
                    continue;
                }
                let f = match case {
                    "zero" => Rect::new(16., 16., 0., 200.),
                    "narrow" => Rect::new(16., 16., 64., 200.),
                    "fractional" => Rect::new(16.35, 16.65, 300., 200.),
                    _ => Rect::new(16., 16., 300., 200.),
                };
                let c = if case == "ancestorclip" {
                    Rect::new(35., 57., 110., 53.)
                } else {
                    f
                };
                let prefix = format!("mode{mode}-{case}-{}x", scale as u32);
                let mut model = Tree::default();
                model.replace_keyed(D(1), model_nodes()).unwrap();
                model.select_key(K(1));
                let mut view = TreeView::new();
                let mut text = Labels::new(scale);
                let mut iconcache = SvgIconCache::default();
                let mut scene = Scene::new();
                view.render_model(&mut scene, &mut text, &mut model, f, Some(c), p);
                view.focus_model(0, true, &mut model);
                scene.clear();
                text.atlas.begin_frame();
                let layout = view.render_model(&mut scene, &mut text, &mut model, f, Some(c), p);
                let mut view_icon_packets = vec![];
                view.render_icons_model(
                    &mut scene,
                    &mut TraceIcons {
                        inner: &mut iconcache.renderer(&mut gpu, scale),
                        packets: &mut view_icon_packets,
                    },
                    &mut model,
                    Some(c),
                    p,
                    proposal::metric("size", "size-icon-standard").unwrap(),
                );
                let old = gpu.render_to_rgba8(&scene, &mut text.atlas, size, scale, bg)?;
                save(&out, &format!("{prefix}-view"), &old, size)?;
                let view_packets = text.packets.clone();
                let view_state_rows=model.nodes().iter().enumerate().map(|(i,n)|json!({"label":n.label,"selected":model.selected==Some(i),"disabled":n.disabled,"focused":view.focused_index(&model)==Some(i)})).collect::<Vec<_>>();
                let mut h = ContentHost::new(Content::new(ComponentId(10), content(false)))?;
                let mut text = Labels::new(scale);
                let mut iconcache = SvgIconCache::default();
                let mut scene = Scene::new();
                let mut host_icon_packets = vec![];
                h.render(
                    &mut scene,
                    &mut WithIconRenderer::new(
                        &mut text,
                        &mut TraceIcons {
                            inner: &mut iconcache.renderer(&mut gpu, scale),
                            packets: &mut host_icon_packets,
                        },
                    ),
                    f,
                    c,
                    p,
                )?;
                let rest = gpu.render_to_rgba8(&scene, &mut text.atlas, size, scale, bg)?;
                h.focus(Some(ComponentId(10)), true);
                scene.clear();
                text.atlas.begin_frame();
                h.render(
                    &mut scene,
                    &mut WithIconRenderer::new(
                        &mut text,
                        &mut TraceIcons {
                            inner: &mut iconcache.renderer(&mut gpu, scale),
                            packets: &mut host_icon_packets,
                        },
                    ),
                    f,
                    c,
                    p,
                )?;
                let new = gpu.render_to_rgba8(&scene, &mut text.atlas, size, scale, bg)?;
                save(&out, &format!("{prefix}-host"), &new, size)?;
                let focus_changed = rest.iter().zip(&new).filter(|(a, b)| a != b).count();
                if case == "normal" {
                    ensure!(focus_changed > 0, "focus-visible must leave actual GPU ink");
                    save(&out, &format!("{prefix}-host-rest"), &rest, size)?;
                }
                let reference_rows=layout.items.iter().map(|r|json!({"id":r.id,"depth":r.depth,"frame":[r.frame.min_x(),r.frame.min_y(),r.frame.size.width,r.frame.size.height]})).collect::<Vec<_>>();
                let glyph_layer =
                    gpu.render_to_rgba8(&layer(&scene, true), &mut text.atlas, size, scale, bg)?;
                let icon_layer =
                    gpu.render_to_rgba8(&layer(&scene, false), &mut text.atlas, size, scale, bg)?;
                save(&out, &format!("{prefix}-host-glyphs"), &glyph_layer, size)?;
                save(&out, &format!("{prefix}-host-icons"), &icon_layer, size)?;
                let glyph_ink = ink(&glyph_layer, &empty, size, c, scale).0;
                let icon_ink = ink(&icon_layer, &empty, size, c, scale).0;
                if case == "normal" && (glyph_ink == 0 || icon_ink == 0) {
                    failures.push(format!("{prefix}: missing positive glyph/icon ink"));
                }
                // Real GPU negative scenes: blank glyph layer and an un-clipped outside Quad.
                let mut leaked = Scene::new();
                leaked.push_quad(Quad::new(
                    Rect::new(1., 1., 4., 4.),
                    p.color("primary-text").unwrap(),
                ));
                let leak =
                    gpu.render_to_rgba8(&leaked, &mut GlyphAtlas::default(), size, scale, bg)?;
                let overflow_negative_detected = ink(&leak, &empty, size, c, scale).1 > 0;
                ensure!(overflow_negative_detected);
                if case == "normal" {
                    save(&out, &format!("{prefix}-negative-overflow"), &leak, size)?;
                    save(&out, &format!("{prefix}-negative-blank"), &empty, size)?;
                }
                let delta = old.iter().zip(&new).filter(|(a, b)| a != b).count();
                let (positive, outside) = ink(&new, &empty, size, c, scale);
                let (old_positive, old_outside) = ink(&old, &empty, size, c, scale);
                // Deliberately removing all primitives must fail the same positive-ink oracle.
                let missing_ink_rejected =
                    case == "zero" || ink(&empty, &empty, size, c, scale).0 == 0;
                if delta != 0
                    || outside != 0
                    || old_outside != 0
                    || (case != "zero" && positive == 0)
                    || (case == "zero" && positive != 0)
                {
                    failures.push(format!("{prefix}: delta={delta}, ink={positive}, outside={outside}, oldoutside={old_outside}"));
                }
                let row_geometry=h.semantics().iter().find(|n|n.id==ComponentId(10)).map(|n|n.parts.iter().map(|p|json!({"frame":[p.frame.min_x(),p.frame.min_y(),p.frame.size.width,p.frame.size.height],"role":format!("{:?}",p.semantics.role)})).collect::<Vec<_>>());
                let mut custom = ContentHost::new(Content::new(ComponentId(10), content(true)))?;
                let mut ct = Labels::new(scale);
                let mut cache = SvgIconCache::default();
                let mut cs = Scene::new();
                let mut custom_icon_packets = vec![];
                custom.render(
                    &mut cs,
                    &mut WithIconRenderer::new(
                        &mut ct,
                        &mut TraceIcons {
                            inner: &mut cache.renderer(&mut gpu, scale),
                            packets: &mut custom_icon_packets,
                        },
                    ),
                    f,
                    c,
                    p,
                )?;
                custom.focus(Some(ComponentId(10)), true);
                cs.clear();
                ct.atlas.begin_frame();
                custom.render(
                    &mut cs,
                    &mut WithIconRenderer::new(
                        &mut ct,
                        &mut TraceIcons {
                            inner: &mut cache.renderer(&mut gpu, scale),
                            packets: &mut custom_icon_packets,
                        },
                    ),
                    f,
                    c,
                    p,
                )?;
                let cp = gpu.render_to_rgba8(&cs, &mut ct.atlas, size, scale, bg)?;
                save(&out, &format!("{prefix}-custom"), &cp, size)?;
                let (custom_positive, custom_outside) = ink(&cp, &empty, size, c, scale);
                if custom_outside != 0 || (case != "zero" && custom_positive == 0) {
                    failures.push(format!(
                        "{prefix}: custom ink={custom_positive} outside={custom_outside}"
                    ));
                }
                let custom_glyph =
                    gpu.render_to_rgba8(&layer(&cs, true), &mut ct.atlas, size, scale, bg)?;
                let custom_icon =
                    gpu.render_to_rgba8(&layer(&cs, false), &mut ct.atlas, size, scale, bg)?;
                let custom_glyph_ink = ink(&custom_glyph, &empty, size, c, scale).0;
                let custom_icon_ink = ink(&custom_icon, &empty, size, c, scale).0;
                if case == "normal" {
                    save(
                        &out,
                        &format!("{prefix}-custom-glyphs"),
                        &custom_glyph,
                        size,
                    )?;
                    save(&out, &format!("{prefix}-custom-icons"), &custom_icon, size)?;
                    ensure!(custom_glyph_ink > 0 && custom_icon_ink > 0);
                }
                let custom_display_frames = [201, 202, 203].map(|id| {
                    custom.get(ComponentId(id)).map(|n| {
                        let r = n.frame();
                        json!({"id":id,"frame":[r.min_x(),r.min_y(),r.size.width,r.size.height]})
                    })
                });
                let resolved = |usage: &str, state: &str| {
                    let c = tekes_ui_tokens::surface_palette::resolve_surface(
                        p, "canvas", usage, state,
                    )
                    .unwrap();
                    json!({"usage":usage,"state":state,"text":[c.text.r,c.text.g,c.text.b,c.text.a],"icon":[c.icon.r,c.icon.g,c.icon.b,c.icon.a]})
                };
                let state_rows=h.semantics().into_iter().find(|n|n.id==ComponentId(10)).map(|n|n.parts.into_iter().map(|p|json!({"label":p.semantics.name,"selected":p.semantics.selected,"disabled":p.semantics.disabled,"focused":p.focused})).collect::<Vec<_>>());
                cases.push(json!({"attribution":{"view_state_rows":view_state_rows,"view_icon_packets":view_icon_packets,"mounted_icon_packets":host_icon_packets,"custom_slot_icon_packets":custom_icon_packets,"theme":format!("{t:?}"),"view_surface_argument":"base -> canvas fallback","mounted_explicit_surface":h.get(ComponentId(10)).unwrap().surface(),"mounted_inherited_surface":"canvas","view_text_packets":view_packets,"mounted_text_packets":text.packets,"custom_slot_text_packets":ct.packets,"state_rows":state_rows,"quiet_rest":resolved("action-quiet","rest"),"quiet_selected":resolved("action-quiet","selected"),"slot_child_theme":resolved("content","rest")},"reference_rows":reference_rows,"focus_visible_changed_channels":focus_changed,"custom_display_frames":custom_display_frames,"host_glyph_ink":glyph_ink,"host_icon_ink":icon_ink,"custom_glyph_ink":custom_glyph_ink,"custom_icon_ink":custom_icon_ink,"GPU_negative_overflow_detected":overflow_negative_detected,"case":prefix,"size":size,"palette":format!("{:?}",p.source()),"clip":[c.min_x(),c.min_y(),c.size.width,c.size.height],"default_changed_channels":delta,"default_ink":positive,"view_ink":old_positive,"default_outside":outside,"view_outside":old_outside,"custom_ink":custom_positive,"custom_outside":custom_outside,"host_text_draws":text.draws,"custom_text_draws":ct.draws,"missing_ink_oracle_rejects_blank":missing_ink_rejected,"original_layout":format!("{layout:?}"),"host_row_geometry":row_geometry}));
            }
        }
    }
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"actual_GPU_readback":true,"formal_external_theme":theme_id,"cases":cases,"failures":failures}),
        )?,
    )?;
    println!(
        "{}",
        json!({"cases":cases.len(),"failures":failures,"output":out,"real_CoreText_SVG_Metal_readback":true})
    );
    ensure!(
        failures.is_empty(),
        "GPU acceptance failed; original images retained"
    );
    Ok(())
}
