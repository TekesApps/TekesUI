//! Identical program in baseline and candidate: actual unshaped output compatibility.
use std::{
    fs,
    path::{Path, PathBuf},
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{
    AtlasConfig, CornerRadii, FontKey, GlyphAtlas, GlyphKey, GlyphRun, ImageQuad, Point,
    PositionedGlyph, Quad, RasterFormat, RasterGlyph, Rect, Scene, ShadowQuad,
    TransformedImageQuad,
};
use tekes_ui_tokens::{ColorRole, Rgba, Theme};
fn save(out: &Path, name: &str, size: (u32, u32), data: &[u8]) {
    fs::write(out.join(format!("{name}.rgba")), data).unwrap();
    let mut png = png::Encoder::new(
        fs::File::create(out.join(format!("{name}.png"))).unwrap(),
        size.0,
        size.1,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header().unwrap().write_image_data(data).unwrap();
}
#[test]
fn actual_unshaped_pixels_are_byte_identical_to_frozen_baseline() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = std::env::var_os("TEKESUI_CLIP_TEST_OUTPUT")
        .or_else(|| std::env::var_os("CARGO_TARGET_DIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.ancestors().nth(2).unwrap().join("target"));
    let out = root.join("gpu-legacy");
    fs::create_dir_all(&out).unwrap();
    let mut renderer = Renderer::new_headless().expect("actual Metal GPU required, no skip");
    let image = renderer.register_image(&[90, 25, 50, 160], 1, 1).unwrap();
    for (theme_name, theme) in [("light", Theme::LIGHT), ("dark", Theme::DARK)] {
        for scale in [0.5_f32, 1., 1.25, 1.5, 2.] {
            let mut atlas = GlyphAtlas::new(AtlasConfig {
                page_size: 256,
                max_pages: 2,
                padding: 1,
            });
            let mut glyphs = Vec::new();
            for (id, format) in [(1, RasterFormat::Mask), (2, RasterFormat::Color)] {
                let w = (23. * scale) as u32;
                let h = (18. * scale) as u32;
                let glyph = atlas
                    .get_or_insert_with(
                        GlyphKey::new(FontKey(80), id, GlyphKey::scale_key_for(scale)),
                        || {
                            Some(RasterGlyph {
                                left: 0,
                                top: 0,
                                width: w,
                                height: h,
                                format,
                                pixels: [40, 60, 90, 160].repeat((w * h) as usize),
                            })
                        },
                    )
                    .unwrap();
                glyphs.push(PositionedGlyph {
                    origin: Point::new(8. + f32::from(id) * 9., 23.),
                    glyph,
                });
            }
            let mut scene = Scene::new();
            scene.push_clip(Rect::new(1.25, 0.75, 52.5, 58.25));
            scene.push_shadow(
                ShadowQuad::new(
                    Rect::new(7.375, 8.125, 36., 34.5),
                    Rgba::new(0.1, 0.2, 0.3, 0.45),
                )
                .with_corner_radius(8.)
                .with_offset(Point::new(3.5, -0.25))
                .with_blur_radius(4.5)
                .with_outset(0.5),
            );
            scene.push_quad(
                Quad::new(
                    Rect::new(7.375, 8.125, 36., 34.5),
                    Rgba::new(0.2, 0.4, 0.8, 0.43),
                )
                .with_corner_radii(CornerRadii {
                    top_left: 8.,
                    top_right: 3.,
                    bottom_right: 12.,
                    bottom_left: 1.,
                })
                .with_border(0.6, Rgba::new(0.9, 0.1, 0.2, 0.32)),
            );
            scene.push_clip(Rect::new(4.1, 5.9, 48., 48.));
            scene.push_image(ImageQuad {
                rect: Rect::new(12.25, 12.5, 33., 30.),
                image,
                opacity: 0.7,
            });
            scene.push_transformed_image(TransformedImageQuad::new(
                ImageQuad {
                    rect: Rect::new(9., 7., 37., 34.),
                    image,
                    opacity: 0.5,
                },
                0.37,
            ));
            scene.push_glyphs(GlyphRun {
                color: Rgba::new(0.9, 0.7, 0.1, 0.65),
                glyphs,
            });
            scene.pop_clip();
            scene.pop_clip();
            let size = ((64. * scale) as u32, (64. * scale) as u32);
            let data = renderer
                .render_to_rgba8(
                    &scene,
                    &mut atlas,
                    size,
                    scale,
                    ColorRole::DocumentCanvas.resolve(theme),
                )
                .unwrap();
            let suffix = format!("{theme_name}-{}", (scale * 100.) as u32);
            save(&out, &format!("current-{suffix}"), size, &data);
            let baseline = manifest
                .join("tests/fixtures/clip-legacy")
                .join(format!("baseline-{suffix}.png"));
            let decoder =
                png::Decoder::new(std::io::BufReader::new(fs::File::open(baseline).unwrap()));
            let mut reader = decoder.read_info().unwrap();
            let mut bytes = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut bytes).unwrap();
            assert_eq!((info.width, info.height), size);
            assert_eq!(info.color_type, png::ColorType::Rgba);
            assert_eq!(
                data,
                bytes[..info.buffer_size()],
                "unshaped legacy output {suffix}"
            );
        }
    }
}
