use std::path::PathBuf;
use tekes_ui_controls::{extended::Axis, layout_view::SeparatorView, paint};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};
fn write_png(path: PathBuf, pixels: &[u8], size: (u32, u32)) {
    let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), size.0, size.1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels)
        .unwrap();
}
fn main() {
    let out =
        PathBuf::from(std::env::var_os("TEKESUI_SEPARATOR_SCALE_OUT").expect("explicit output"));
    std::fs::create_dir_all(&out).unwrap();
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut atlas = tekes_ui_scene::GlyphAtlas::new(Default::default());
    let mut cases = Vec::new();
    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
        for brand in 0..3 {
            for scale in [1., 2.] {
                for vertical in [false, true] {
                    for phase in [0., 0.25, 0.5] {
                        let palette = Palette { theme, brand };
                        let frame = if vertical {
                            Rect::new(20. + phase, 8., 12., 100.)
                        } else {
                            Rect::new(8., 20. + phase, 100., 12.)
                        };
                        let axis = if vertical {
                            Axis::Vertical
                        } else {
                            Axis::Horizontal
                        };
                        let mut view = SeparatorView::default();
                        let mut actual = Scene::new();
                        view.render_at_scale(&mut actual, frame, axis, None, palette, scale);
                        let q = actual
                            .primitives()
                            .iter()
                            .find_map(|p| {
                                if let Primitive::Quad(q) = p {
                                    Some(*q)
                                } else {
                                    None
                                }
                            })
                            .unwrap();
                        let (width, color) = paint::decorative_border(palette, scale);
                        let expected_frame = if vertical {
                            Rect::new(
                                frame.center().x - width / 2.,
                                frame.min_y(),
                                width,
                                frame.size.height,
                            )
                        } else {
                            Rect::new(
                                frame.min_x(),
                                frame.center().y - width / 2.,
                                frame.size.width,
                                width,
                            )
                        };
                        let mut expected = Scene::new();
                        expected.push_quad(Quad::new(expected_frame, color));
                        let size = ((128. * scale) as u32, (128. * scale) as u32);
                        let clear = palette.color("surface-1").unwrap();
                        let a = gpu
                            .render_to_rgba8(&actual, &mut atlas, size, scale, clear)
                            .unwrap();
                        let b = gpu
                            .render_to_rgba8(&expected, &mut atlas, size, scale, clear)
                            .unwrap();
                        let changed = a
                            .chunks_exact(4)
                            .zip(b.chunks_exact(4))
                            .filter(|(a, b)| a != b)
                            .count();
                        let name =
                            format!("t{ti}-b{brand}-d{scale}-vertical{vertical}-phase{phase}");
                        write_png(out.join(format!("{name}-actual.png")), &a, size);
                        write_png(out.join(format!("{name}-reference.png")), &b, size);
                        let actual_width = if vertical {
                            q.rect.size.width
                        } else {
                            q.rect.size.height
                        };
                        let before_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .join("../../separator-scale-diagnostic-20261021/gpu");
                        let mut reader = png::Decoder::new(
                            std::fs::File::open(before_root.join(format!("{name}-actual.png")))
                                .unwrap(),
                        )
                        .read_info()
                        .unwrap();
                        let mut before = vec![0; reader.output_buffer_size()];
                        let info = reader.next_frame(&mut before).unwrap();
                        before.truncate(info.buffer_size());
                        assert_eq!(before.len(), a.len());
                        let before_changed = before
                            .chunks_exact(4)
                            .zip(a.chunks_exact(4))
                            .filter(|(a, b)| a != b)
                            .count();
                        if theme.contrast != tekes_ui_tokens::Contrast::Increased && scale == 2. {
                            assert!(before_changed > 0);
                        } else {
                            assert_eq!(
                                before_changed, 0,
                                "1x/increased contrast must remain pixel-identical"
                            );
                        }
                        cases.push(serde_json::json!({"theme":ti,"brand":brand,"scale":scale,"vertical":vertical,"phase":phase,"actual_logical_width":actual_width,"expected_logical_width":width,"actual_physical_width":actual_width*scale,"expected_physical_width":width*scale,"before_after_changed_pixels":before_changed,"mismatch_pixels":changed,"matches_shared_rule":a==b,"actual_image":format!("{name}-actual.png"),"reference_image":format!("{name}-reference.png")}));
                    }
                }
            }
        }
    }
    let failures = cases
        .iter()
        .filter(|c| c["matches_shared_rule"] == false)
        .count();
    assert_eq!(cases.len(), 144);
    assert_eq!(
        failures, 0,
        "diagnostic expected scope changed; review source and shared rules"
    );
    std::fs::write(out.parent().unwrap().join("gpu-result.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"shared-scale GPU conformance passed; native/whole experience unverified","cases":cases,"failed_shared_rule_cases":failures,"frames":288,"scope":"real public SeparatorView/Metal diagnostic vs existing paint::decorative_border; reference is shared contract, not a new design proposal; no native/performance or ClientKit acceptance"})).unwrap()).unwrap();
    println!("Recorded144cases:all shared-scale GPU references match, native separate");
}
