//! Current public label consumer with actual CoreText fallback runs and Metal.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use std::rc::Rc;
use tekes_ui_composition_consumer::label_content::{self, SOURCE};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, TextSurfaceHost, TextSurfaceTextRenderer, composition::*,
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene, Size};
use tekes_ui_text::{LabelLayout, LabelOptions, Paragraph};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

struct Text {
    inner: coretext::Text,
    shaped: Vec<Rc<LabelLayout>>,
    painted: Vec<(Rc<LabelLayout>, Point, Rect, Rgba)>,
    glyphs: Scene,
}
impl ButtonTextRenderer for Text {
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.inner.resolved_font_size(role)
    }
    fn backing_scale(&self) -> f32 {
        self.inner.backing_scale()
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        self.inner.measure(s, role, width)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
        panic!("opt-in labels must paint their exact measured paragraph");
    }
}
impl TextSurfaceHost for Text {
    fn type_scale(&self) -> f32 {
        self.inner.scale
    }
    fn shape_surface(
        &mut self,
        s: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        self.inner.shape_surface(s, role, width)
    }
    fn shape_label(
        &mut self,
        s: &str,
        role: proposal::TextRole,
        width: f32,
        options: LabelOptions,
    ) -> Option<Rc<LabelLayout>> {
        let layout = self.inner.shape_label(s, role, width, options)?;
        self.shaped.push(layout.clone());
        Some(layout)
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        scene: &mut Scene,
        p: &Paragraph,
        origin: Point,
        ink: Rgba,
        clip: Rect,
    ) {
        let measured = self
            .shaped
            .iter()
            .rev()
            .find(|layout| std::ptr::eq(layout.paragraph(), p))
            .expect("paint borrows the same measured LabelLayout")
            .clone();
        assert_eq!(measured.source(), SOURCE);
        assert_eq!(p.utf16_len, SOURCE.encode_utf16().count());
        assert_eq!(p.lines.len(), 1);
        assert!(measured.height() <= clip.size.height as f64 + 0.01);
        self.painted.push((measured, origin, clip, ink));
        self.inner.push_visible(scene, p, origin, ink, clip);
        // A separate glyph-only capture makes visible text observable even when
        // original Button chrome has its own nonempty pixels.
        self.glyphs.push_clip(clip);
        self.inner
            .push_visible(&mut self.glyphs, p, origin, ink, clip);
        self.glyphs.pop_clip();
    }
}

#[test]
fn retained_public_labels_paint_measured_fallback_runs_across_theme_type_and_width() {
    let mut gpu = tekes_ui_render::Renderer::new_headless().expect("real Metal renderer");
    let output = std::env::var_os("TEKESUI_LABEL_CONTENT_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(out) = &output {
        std::fs::create_dir_all(out).unwrap();
    }
    let mut host = label_content::build().unwrap();
    let mounts = [ComponentId(9901), ComponentId(9902), ComponentId(9903)]
        .map(|id| host.semantic_mount(id).unwrap());
    let mut text = Text {
        inner: coretext::Text::new(1.),
        shaped: vec![],
        painted: vec![],
        glyphs: Scene::new(),
    };
    text.inner.services = true;
    let mut reports = Vec::new();
    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
        for type_scale in [1., 1.5, 2.] {
            for backing in [1., 2.] {
                for (wi, width) in [140., 1800., 220., 1800.].into_iter().enumerate() {
                    text.inner.scale = type_scale;
                    text.inner.backing = backing;
                    text.shaped.clear();
                    text.painted.clear();
                    text.glyphs.clear();
                    text.inner.atlas.begin_frame();
                    let palette = Palette::from(theme);
                    let viewport = Rect::new(0., 0., width + 40., 400.);
                    let frame = Rect::new(20., 20., width, 1.);
                    let mut scene = Scene::new();
                    host.render(&mut scene, &mut text, frame, viewport, palette)
                        .unwrap();
                    assert_eq!(text.painted.len(), 3);
                    assert_eq!(
                        mounts,
                        [ComponentId(9901), ComponentId(9902), ComponentId(9903)]
                            .map(|id| host.semantic_mount(id).unwrap())
                    );
                    let semantics = host.semantics();
                    assert_eq!(
                        semantics
                            .iter()
                            .map(|node| (node.id, node.parent, node.semantics.role))
                            .collect::<Vec<_>>(),
                        vec![
                            (label_content::ROOT, None, ContentRole::Group),
                            (
                                ComponentId(9901),
                                Some(label_content::ROOT),
                                ContentRole::Text
                            ),
                            (
                                ComponentId(9902),
                                Some(label_content::ROOT),
                                ContentRole::Button
                            ),
                            (
                                ComponentId(9903),
                                Some(label_content::ROOT),
                                ContentRole::Button
                            ),
                        ]
                    );
                    assert!(semantics[0].semantics.name.is_empty());
                    assert!(
                        semantics[1..]
                            .iter()
                            .all(|node| node.semantics.name == SOURCE)
                    );
                    assert_eq!(host.focus_targets().len(), 2);
                    assert_eq!(text.painted[0].3, palette.color("primary-text").unwrap());
                    for (layout, _, _, _) in &text.painted {
                        assert!(
                            layout.height()
                                >= f64::from(
                                    proposal::TextRole::UiBody.style().line_height * type_scale
                                )
                        );
                        if width == 1800. {
                            assert!(
                                !layout.is_truncated(),
                                "wide label must retain all fallback runs"
                            );
                            assert!(layout.paragraph().lines[0].runs.len() > 1);
                            assert!(layout.paragraph().fonts.len() > 1);
                        } else {
                            assert!(layout.is_truncated());
                        }
                    }
                    assert!(
                        scene
                            .primitives()
                            .iter()
                            .any(|p| matches!(p, Primitive::Glyphs(_)))
                    );
                    let size = (((width + 40.) * backing) as u32, (400. * backing) as u32);
                    let clear = palette.color("surface-1").unwrap();
                    let actual = gpu
                        .render_to_rgba8(&scene, &mut text.inner.atlas, size, backing, clear)
                        .unwrap();
                    let glyphs = gpu
                        .render_to_rgba8(&text.glyphs, &mut text.inner.atlas, size, backing, clear)
                        .unwrap();
                    let blank = gpu
                        .render_to_rgba8(&Scene::new(), &mut text.inner.atlas, size, backing, clear)
                        .unwrap();
                    let ink_pixels = glyphs
                        .chunks_exact(4)
                        .zip(blank.chunks_exact(4))
                        .filter(|(a, b)| a != b)
                        .count();
                    assert!(
                        ink_pixels > 0,
                        "real glyph pixels must be visible, independently of Button chrome"
                    );
                    let row = host.get(ComponentId(9901)).unwrap().frame();
                    assert!(
                        row.size.height
                            < 2. * proposal::TextRole::UiBody.style().line_height * type_scale
                    );
                    for y in 0..size.1 {
                        let py = (y as f32 + 0.5) / backing;
                        if py < row.min_y() || py >= row.max_y() {
                            continue;
                        }
                        for x in 0..size.0 {
                            let at = ((y * size.0 + x) * 4) as usize;
                            assert_eq!(
                                &actual[at..at + 4],
                                &glyphs[at..at + 4],
                                "original TextContent must paint its measured glyphs once at the recorded origin"
                            );
                        }
                    }
                    if let Some(out) = &output {
                        for (phase, pixels) in [("actual", actual), ("glyphs", glyphs)] {
                            resvg::tiny_skia::Pixmap::from_vec(
                                pixels,
                                resvg::tiny_skia::IntSize::from_wh(size.0, size.1).unwrap(),
                            )
                            .unwrap()
                            .save_png(out.join(format!(
                                "label-{ti}-{type_scale}-{backing}-{wi}-{phase}.png"
                            )))
                            .unwrap();
                        }
                    }
                    reports.push(serde_json::json!({"theme":format!("{theme:?}"),
                        "type_scale":type_scale,"backing_scale":backing,"width":width,
                        "label_count":3,"visible_glyph_pixels":ink_pixels,
                        "painted_runs":text.painted.iter().map(|(layout,_,_,_)|
                            layout.paragraph().lines[0].runs.len()).collect::<Vec<_>>(),
                        "measured_paragraph_identity":true,"full_AX_names":true,
                        "native_input":false,"independent_native_truncation_oracle":false}));
                }
            }
        }
    }
    assert_eq!(reports.len(), 96);
    if let Some(out) = &output {
        std::fs::write(
            out.join("receipt.json"),
            serde_json::to_vec_pretty(&reports).unwrap(),
        )
        .unwrap();
    }
}
