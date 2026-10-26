use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, Insets, composition::*, layout::MeasuredChild,
};
use tekes_ui_scene::*;
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text(f32);
impl ButtonTextRenderer for Text {
    fn backing_scale(&self) -> f32 {
        self.0
    }
    fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
        Size::new(20., 20.)
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
    }
}
#[derive(Debug)]
struct Ink(Vec<Primitive>);
impl Component for Ink {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(64., 64.))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        _: ContentTheme,
    ) {
        for p in &self.0 {
            s.push(p.clone());
        }
    }
}
fn scene(primitives: Vec<Primitive>, scale: f32, theme: Theme) -> Scene {
    let child = Content::new(ComponentId(8101), Ink(primitives));
    let mut panel = PanelContent::new(child).unwrap();
    panel.set_padding(Some(Insets::default())).unwrap();
    panel.set_canvas(true);
    let mut host = ContentHost::new(Content::new(ComponentId(8100), panel)).unwrap();
    let mut s = Scene::new();
    let f = Rect::new(0., 0., 64., 64.);
    host.render(&mut s, &mut Text(scale), f, f, theme.into())
        .unwrap();
    s
}
fn gpu_artifact_dir() -> std::path::PathBuf {
    let out = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_PANEL81_GPU_ARTIFACT_DIR")
            .expect("set a fresh SSD Issue #81 GPU artifact directory"),
    );
    assert!(out.starts_with("/Volumes/HP P900/DEV/TekesUI/artifacts/issue-81/"));
    std::fs::create_dir_all(&out).expect("create SSD Panel GPU artifact directory");
    out
}

#[test]
fn actual_original_panel_subtree_clips_all_pipelines_at_five_scales() {
    let mut renderer =
        tekes_ui_render::Renderer::new_headless().expect("actual Metal required; no skip");
    let image = renderer.register_image(&[230, 51, 13, 255], 1, 1).unwrap();
    let out = gpu_artifact_dir();
    for scale in [0.5, 1., 1.25, 1.5, 2.] {
        for (name, theme) in [("light", Theme::LIGHT), ("dark", Theme::DARK)] {
            let mut atlas = GlyphAtlas::default();
            let size = ((64. * scale) as u32, (64. * scale) as u32);
            let clear = Rgba::new(0., 1., 0., 1.);
            let base = renderer
                .render_to_rgba8(&scene(vec![], scale, theme), &mut atlas, size, scale, clear)
                .unwrap();
            let glyph = atlas
                .get_or_insert_with(
                    GlyphKey::new(FontKey(81), 1, GlyphKey::scale_key_for(scale)),
                    || {
                        let side = (64. * scale) as u32;
                        Some(RasterGlyph {
                            left: 0,
                            top: 0,
                            width: side,
                            height: side,
                            format: RasterFormat::Mask,
                            pixels: [255, 255, 255, 255].repeat((side * side) as usize),
                        })
                    },
                )
                .unwrap();
            let full = Rect::new(-32., -32., 128., 128.);
            let red = Rgba::rgb8(230, 51, 13);
            let primitives = [
                ("quad", Primitive::Quad(Quad::new(full, red))),
                (
                    "image",
                    Primitive::Image(ImageQuad {
                        rect: full,
                        image,
                        opacity: 1.,
                    }),
                ),
                (
                    "transformed",
                    Primitive::TransformedImage(TransformedImageQuad::new(
                        ImageQuad {
                            rect: full,
                            image,
                            opacity: 1.,
                        },
                        0.37,
                    )),
                ),
                (
                    "glyph",
                    Primitive::Glyphs(GlyphRun {
                        color: red,
                        glyphs: vec![PositionedGlyph {
                            origin: Point::default(),
                            glyph,
                        }],
                    }),
                ),
                (
                    "shadow",
                    Primitive::Shadow(ShadowQuad::new(full, red).with_blur_radius(2.)),
                ),
            ];
            std::fs::write(out.join(format!("base-{name}-{scale}.rgba")), &base).unwrap();
            for (kind, p) in primitives {
                let actual_scene = scene(vec![p], scale, theme);
                std::fs::write(
                    out.join(format!("{kind}-{name}-{scale}.scene.txt")),
                    format!("{actual_scene:#?}"),
                ).unwrap();
                let pixels = renderer
                    .render_to_rgba8(
                        &actual_scene,
                        &mut atlas,
                        size,
                        scale,
                        clear,
                    )
                    .unwrap();
                std::fs::write(out.join(format!("{kind}-{name}-{scale}.rgba")), &pixels).unwrap();
                assert_eq!(&pixels[..4], &base[..4], "curve gap leaked {kind} {scale}");
                let center = (((size.1 / 2) * size.0 + size.0 / 2) * 4) as usize;
                assert_ne!(
                    &pixels[center..center + 4],
                    &base[center..center + 4],
                    "positive ink missing {kind} {scale}"
                );
            }
        }
    }
}

#[test]
fn original_panel_boundary_respects_ancestor_rect_and_zero_width() {
    let mut renderer =
        tekes_ui_render::Renderer::new_headless().expect("actual Metal required; no skip");
    let out = gpu_artifact_dir();
    for (name, frame, clip) in [
        (
            "outside",
            Rect::new(0., 0., 64., 64.),
            Rect::new(70., 70., 10., 10.),
        ),
        (
            "zero-width",
            Rect::new(0., 0., 0., 64.),
            Rect::new(0., 0., 0., 64.),
        ),
    ] {
        let child = Content::new(
            ComponentId(8101),
            Ink(vec![Primitive::Quad(Quad::new(
                Rect::new(0., 0., 64., 64.),
                Rgba::new(1., 0., 0., 1.),
            ))]),
        );
        let mut panel = PanelContent::new(child).unwrap();
        panel.set_padding(Some(Insets::default())).unwrap();
        let mut host = ContentHost::new(Content::new(ComponentId(8100), panel)).unwrap();
        let mut scene = Scene::new();
        host.render(&mut scene, &mut Text(1.), frame, clip, Theme::LIGHT.into())
            .unwrap();
        let pixels = renderer
            .render_to_rgba8(
                &scene,
                &mut GlyphAtlas::default(),
                (64, 64),
                1.,
                Rgba::new(0., 1., 0., 1.),
            )
            .unwrap();
        std::fs::write(out.join(format!("negative-{name}.rgba")), &pixels).unwrap();
        assert!(
            pixels.chunks_exact(4).all(|p| p == [0, 255, 0, 255]),
            "ancestor/empty clipping leaked {name}"
        );
    }
}
