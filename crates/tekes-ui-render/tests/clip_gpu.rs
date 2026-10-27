//! Actual Metal readback, no adapter skips. Every image is saved before assertions.
//! The CPU reference enumerates closest points on segments/arcs, independently of
//! the shader's quadrant signed-distance formula. This finite corpus is not a proof.
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
};
use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::{
    AtlasConfig, ClipError, ClipShape, CornerRadii, FontKey, GlyphAtlas, GlyphKey, GlyphRun,
    ImageId, ImageQuad, Point, PositionedGlyph, Quad, RasterFormat, RasterGlyph, Rect, Scene,
    ShadowQuad, ShapeKey, ShapeTable, TransformedImageQuad,
};
use tekes_ui_tokens::{ColorRole, Rgba, Theme};
fn output() -> PathBuf {
    let root = std::env::var_os("TEKESUI_CLIP_TEST_OUTPUT")
        .or_else(|| std::env::var_os("CARGO_TARGET_DIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .unwrap()
                .join("target")
        });
    let p = root.join("gpu-shape");
    fs::create_dir_all(&p).unwrap();
    p
}
fn save(name: &str, size: (u32, u32), data: &[u8]) {
    let out = output();
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
fn rr(rect: Rect, radii: CornerRadii) -> ClipShape {
    ClipShape::normalize(rect, radii, Point::new(0., 0.)).unwrap()
}
fn table_scene(shapes: &[ClipShape]) -> (Scene, Vec<ShapeKey>, ShapeTable) {
    let mut table = ShapeTable::new(80, shapes.len());
    let keys = shapes.iter().map(|s| table.intern(*s).unwrap()).collect();
    (Scene::with_clip_shapes(&table), keys, table)
}
fn render(
    r: &mut Renderer,
    s: &Scene,
    atlas: &mut GlyphAtlas,
    scale: f32,
    name: &str,
    clear: Rgba,
) -> Vec<u8> {
    let size = ((64. * scale) as u32, (64. * scale) as u32);
    let p = r.render_to_rgba8(s, atlas, size, scale, clear).unwrap();
    save(name, size, &p);
    p
}
#[derive(Clone, Copy)]
struct RefShape {
    rect: [f64; 4],
    r: [f64; 4],
}
impl From<ClipShape> for RefShape {
    fn from(s: ClipShape) -> Self {
        let r = s.rect();
        Self {
            rect: [
                f64::from(r.origin.x),
                f64::from(r.origin.y),
                f64::from(r.size.width),
                f64::from(r.size.height),
            ],
            r: s.radii().to_array().map(f64::from),
        }
    }
}
impl RefShape {
    fn inset(self, b: f64) -> Option<Self> {
        let [x, y, w, h] = self.rect;
        if w <= 2. * b || h <= 2. * b {
            return None;
        }
        Some(Self {
            rect: [x + b, y + b, w - 2. * b, h - 2. * b],
            r: self.r.map(|r| (r - b).max(0.)),
        })
    }
    fn coverage(self, p: [f64; 2], scale: f64) -> f64 {
        let [x, y, w, h] = self.rect;
        if w <= 0. || h <= 0. {
            return 0.;
        }
        let [tl, tr, br, bl] = self.r;
        let right = x + w;
        let bottom = y + h;
        let lines = [
            ([x + tl, y], [right - tr, y]),
            ([right, y + tr], [right, bottom - br]),
            ([x + bl, bottom], [right - br, bottom]),
            ([x, y + tl], [x, bottom - bl]),
        ];
        let arcs = [
            ([x + tl, y + tl], tl, [-1., -1.]),
            ([right - tr, y + tr], tr, [1., -1.]),
            ([right - br, bottom - br], br, [1., 1.]),
            ([x + bl, bottom - bl], bl, [-1., 1.]),
        ];
        let mut distance = f64::INFINITY;
        let mut inside = p[0] >= x && p[0] <= right && p[1] >= y && p[1] <= bottom;
        for (a, b) in lines {
            let q = [
                p[0].clamp(a[0].min(b[0]), a[0].max(b[0])),
                p[1].clamp(a[1].min(b[1]), a[1].max(b[1])),
            ];
            distance = distance.min((p[0] - q[0]).hypot(p[1] - q[1]));
        }
        for (c, r, s) in arcs {
            if r == 0. {
                continue;
            }
            let d = [p[0] - c[0], p[1] - c[1]];
            if d[0] * s[0] >= 0. && d[1] * s[1] >= 0. {
                distance = distance.min((d[0].hypot(d[1]) - r).abs());
                if d[0] * d[0] + d[1] * d[1] > r * r {
                    inside = false;
                }
            } else {
                for q in [[c[0] + s[0] * r, c[1]], [c[0], c[1] + s[1] * r]] {
                    distance = distance.min((p[0] - q[0]).hypot(p[1] - q[1]));
                }
            }
        }
        let signed = if inside { -distance } else { distance };
        (0.5 - signed * scale).clamp(0., 1.)
    }
}
fn byte(v: f64) -> u8 {
    (v.clamp(0., 1.) * 255.).round() as u8
}
fn expect_pixel(actual: &[u8], expected: [f64; 4], tolerance: u8, where_: &str) {
    for i in 0..4 {
        assert!(
            actual[i].abs_diff(byte(expected[i])) <= tolerance,
            "{where_}: {actual:?} vs {:?}",
            expected.map(byte)
        );
    }
}
fn draw(
    kind: &str,
    r: &mut Renderer,
    s: &mut Scene,
    atlas: &mut GlyphAtlas,
    scale: f32,
    image: ImageId,
) {
    let full = Rect::new(-32., -32., 128., 128.);
    match kind {
        "quad" => s.push_quad(Quad::new(full, Rgba::rgb8(230, 51, 13))),
        "image" => s.push_image(ImageQuad {
            rect: full,
            image,
            opacity: 1.,
        }),
        "transformed" => s.push_transformed_image(TransformedImageQuad::new(
            ImageQuad {
                rect: full,
                image,
                opacity: 1.,
            },
            0.37,
        )),
        "mask" | "color" => {
            let side = (64. * scale) as u32;
            let format = if kind == "mask" {
                RasterFormat::Mask
            } else {
                RasterFormat::Color
            };
            let glyph = atlas
                .get_or_insert_with(
                    GlyphKey::new(FontKey(80), 1, GlyphKey::scale_key_for(scale)),
                    || {
                        Some(RasterGlyph {
                            left: 0,
                            top: 0,
                            width: side,
                            height: side,
                            format,
                            pixels: [13, 51, 230, 255].repeat((side * side) as usize),
                        })
                    },
                )
                .unwrap();
            s.push_glyphs(GlyphRun {
                color: if kind == "color" {
                    Rgba::new(0., 1., 0., 1.)
                } else {
                    Rgba::rgb8(230, 51, 13)
                },
                glyphs: vec![PositionedGlyph {
                    origin: Point::new(0., 0.),
                    glyph,
                }],
            });
        }
        "shadow" => s.push_shadow(
            ShadowQuad::new(Rect::new(0., 0., 64., 64.), Rgba::new(0.9, 0.2, 0.05, 0.7))
                .with_corner_radius(5.)
                .with_blur_radius(10.),
        ),
        _ => panic!("unknown test primitive"),
    }
    let _ = r;
}
#[test]
fn every_primitive_has_positive_ink_and_no_curve_gap_leak_at_fractional_scales() {
    let mut r = Renderer::new_headless().expect("actual Metal adapter required");
    let image = r.register_image(&[230, 51, 13, 255], 1, 1).unwrap();
    let shape = rr(
        Rect::new(8.25, 9.75, 40.5, 38.5),
        CornerRadii {
            top_left: 10.,
            top_right: 6.,
            bottom_right: 14.,
            bottom_left: 2.,
        },
    );
    let reference = RefShape::from(shape);
    let mut results = Vec::new();
    for scale in [0.5_f32, 1., 1.25, 1.5, 2.] {
        for (theme_name, theme) in [("light", Theme::LIGHT), ("dark", Theme::DARK)] {
            for kind in ["quad", "mask", "color", "image", "transformed", "shadow"] {
                let (mut s, keys, _) = table_scene(&[shape]);
                s.push_shape_clip(keys[0].clone());
                let mut atlas = GlyphAtlas::new(AtlasConfig {
                    page_size: 256,
                    max_pages: 2,
                    padding: 1,
                });
                draw(kind, &mut r, &mut s, &mut atlas, scale, image);
                s.pop_clip();
                let name = format!("primitive-{kind}-{theme_name}-{}", (scale * 100.) as u32);
                let clear = ColorRole::DocumentCanvas.resolve(theme);
                let data = render(&mut r, &s, &mut atlas, scale, &name, clear);
                let w = (64. * scale) as u32;
                let mut ink = 0;
                let mut gap = 0;
                let mut aa = 0;
                for y in 0..w {
                    for x in 0..w {
                        let p = [
                            (f64::from(x) + 0.5) / f64::from(scale),
                            (f64::from(y) + 0.5) / f64::from(scale),
                        ];
                        let coverage = reference.coverage(p, f64::from(scale));
                        let a = &data[((y * w + x) * 4) as usize..((y * w + x) * 4 + 4) as usize];
                        if coverage == 0. {
                            expect_pixel(
                                a,
                                [
                                    f64::from(clear.r),
                                    f64::from(clear.g),
                                    f64::from(clear.b),
                                    1.,
                                ],
                                1,
                                &name,
                            );
                            gap += 1;
                        } else if kind != "shadow" {
                            expect_pixel(
                                a,
                                [
                                    230. / 255. * coverage + f64::from(clear.r) * (1. - coverage),
                                    51. / 255. * coverage + f64::from(clear.g) * (1. - coverage),
                                    13. / 255. * coverage + f64::from(clear.b) * (1. - coverage),
                                    1.,
                                ],
                                2,
                                &name,
                            );
                            ink += 1;
                            if coverage < 1. {
                                aa += 1;
                            }
                        } else if a[0] != byte(f64::from(clear.r))
                            || a[1] != byte(f64::from(clear.g))
                            || a[2] != byte(f64::from(clear.b))
                        {
                            ink += 1;
                        }
                    }
                }
                assert!(
                    ink > 20 && gap > 20,
                    "{name}: both positive ink and rejecting gap required ({ink},{gap})"
                );
                if kind != "shadow" {
                    assert!(aa > 0, "{name}: no AA samples observed");
                }
                results.push(format!(
                    "{{\"case\":\"{name}\",\"ink\":{ink},\"gap\":{gap},\"aa\":{aa}}}"
                ));
            }
        }
    }
    fs::write(
        output().join("primitives-results.json"),
        format!("[{}]\n", results.join(",")),
    )
    .unwrap();
}
#[test]
fn normalized_self_identity_dedup_preserves_transparent_border_and_real_draw_alpha() {
    let mut r = Renderer::new_headless().unwrap();
    let shape = rr(Rect::new(10.5, 10.5, 40., 40.), CornerRadii::uniform(8.));
    let q = Quad::new(shape.rect(), Rgba::new(0., 0., 1., 0.6))
        .with_corner_radii(shape.radii())
        .with_border(0.3, Rgba::new(1., 0., 0., 0.4));
    for scale in [0.5_f32, 1., 1.25, 1.5, 2.] {
        let mut outputs = Vec::new();
        for repeats in 0..=3 {
            let (mut s, keys, _) = table_scene(&[shape]);
            for _ in 0..repeats {
                s.push_shape_clip(keys[0].clone());
            }
            s.push_keyed_quad(q, keys[0].clone());
            for _ in 0..repeats {
                s.pop_clip();
            }
            outputs.push(render(
                &mut r,
                &s,
                &mut GlyphAtlas::default(),
                scale,
                &format!("dedup-{repeats}-{}", (scale * 100.) as u32),
                Rgba::new(0., 0., 0., 0.),
            ));
        }
        for x in &outputs[1..] {
            assert_eq!(
                x, &outputs[0],
                "same identity must be covered once at {scale}"
            );
        }
        let reference = RefShape::from(shape);
        let inner = reference.inset(f64::from(q.border_width)).unwrap();
        let w = (64. * scale) as u32;
        let mut edge_samples = 0;
        for y in 0..w {
            for x in 0..w {
                let p = [
                    (f64::from(x) + 0.5) / f64::from(scale),
                    (f64::from(y) + 0.5) / f64::from(scale),
                ];
                let u = reference.coverage(p, f64::from(scale));
                let v = inner.coverage(p, f64::from(scale)).min(u);
                let ring = (u - v).max(0.);
                let expected = [
                    0.4 * ring,
                    0.,
                    0.6 * (u - 0.4 * ring),
                    0.4 * ring + 0.6 * (u - 0.4 * ring),
                ];
                let actual =
                    &outputs[0][((y * w + x) * 4) as usize..((y * w + x) * 4 + 4) as usize];
                expect_pixel(actual, expected, 2, "transparent fill/border");
                if u > 0. && u < 1. {
                    edge_samples += 1;
                }
            }
        }
        assert!(edge_samples > 10);
    }
    let (mut one, keys, table) = table_scene(&[shape]);
    one.push_shape_clip(keys[0].clone());
    one.push_keyed_quad(q, keys[0].clone());
    one.pop_clip();
    let first = render(
        &mut r,
        &one,
        &mut GlyphAtlas::default(),
        1.,
        "alpha-one",
        Rgba::new(0., 0., 0., 0.),
    );
    let mut two = Scene::with_clip_shapes(&table);
    two.push_shape_clip(keys[0].clone());
    two.push_keyed_quad(q, keys[0].clone());
    two.push_keyed_quad(q, keys[0].clone());
    two.pop_clip();
    let second = render(
        &mut r,
        &two,
        &mut GlyphAtlas::default(),
        1.,
        "alpha-two",
        Rgba::new(0., 0., 0., 0.),
    );
    let i = ((30 * 64 + 10) * 4) as usize;
    expect_pixel(
        &first[i..i + 4],
        [0.12, 0., 0.228, 0.348],
        1,
        "specified alpha example",
    );
    assert!(
        second[i + 3].abs_diff(byte(0.574896)) <= 2,
        "independent real draws must source-over: {:?}",
        &second[i..i + 4]
    );
    assert!(second[i + 3] > first[i + 3] + 30);
}
#[test]
fn shadow_gradient_is_multiplied_by_clip_instead_of_min_or_identity_exclusion() {
    let mut r = Renderer::new_headless().unwrap();
    let shape = rr(Rect::new(10.5, 10.5, 40., 40.), CornerRadii::uniform(8.));
    let reference = RefShape::from(shape);
    for scale in [0.5_f32, 1., 1.25, 1.5, 2.] {
        let shadow = ShadowQuad::new(Rect::new(12., 10., 28., 32.), Rgba::new(0.3, 0.5, 0.9, 0.6))
            .with_corner_radius(6.)
            .with_offset(Point::new(2., -1.))
            .with_blur_radius(18.)
            .with_outset(0.5);
        let mut plain = Scene::new();
        plain.push_shadow(shadow);
        let unclip = render(
            &mut r,
            &plain,
            &mut GlyphAtlas::default(),
            scale,
            &format!("shadow-reference-{}", (scale * 100.) as u32),
            Rgba::new(0., 0., 0., 0.),
        );
        let (mut clipped, keys, _) = table_scene(&[shape]);
        clipped.push_shape_clip(keys[0].clone());
        clipped.push_shadow(shadow);
        clipped.pop_clip();
        let data = render(
            &mut r,
            &clipped,
            &mut GlyphAtlas::default(),
            scale,
            &format!("shadow-product-{}", (scale * 100.) as u32),
            Rgba::new(0., 0., 0., 0.),
        );
        let w = (64. * scale) as u32;
        let mut discriminating = 0;
        let mut full_gradient = 0;
        for y in 0..w {
            for x in 0..w {
                let p = [
                    (f64::from(x) + 0.5) / f64::from(scale),
                    (f64::from(y) + 0.5) / f64::from(scale),
                ];
                let k = reference.coverage(p, f64::from(scale));
                let i = ((y * w + x) * 4) as usize;
                for c in 0..4 {
                    assert!(
                        data[i + c].abs_diff((f64::from(unclip[i + c]) * k).round() as u8) <= 2,
                        "shadow product at {x},{y}: {} vs {} * {k}",
                        data[i + c],
                        unclip[i + c]
                    );
                }
                if k > 0.1
                    && k < 0.9
                    && unclip[i + 3] > 25
                    && (f64::from(unclip[i + 3]) / 255.).min(k)
                        - f64::from(unclip[i + 3]) / 255. * k
                        > 0.025
                {
                    discriminating += 1;
                }
                if k == 1. && unclip[i + 3] > 0 && unclip[i + 3] < 140 {
                    full_gradient += 1;
                }
            }
        }
        assert!(
            discriminating > 2 && full_gradient > 20,
            "actual gradient/product witnesses required"
        );
    }
}
#[test]
fn nested_domains_fractional_rect_ancestors_zero_and_tangent_support() {
    let mut r = Renderer::new_headless().unwrap();
    let a = rr(
        Rect::new(5.25, 4.75, 48., 50.),
        CornerRadii {
            top_left: 8.,
            top_right: 12.,
            bottom_right: 2.,
            bottom_left: 6.,
        },
    );
    let b = rr(
        Rect::new(10.5, 8.25, 42., 42.),
        CornerRadii {
            top_left: 5.,
            top_right: 15.,
            bottom_right: 5.,
            bottom_left: 9.,
        },
    );
    for scale in [0.5_f32, 1., 1.25, 1.5, 2.] {
        let (mut s, keys, _) = table_scene(&[a, b]);
        for key in &keys {
            s.push_shape_clip(key.clone());
        }
        s.push_quad(Quad::new(Rect::new(-32., -32., 128., 128.), Rgba::gray(1.)));
        s.pop_clip();
        s.pop_clip();
        let data = render(
            &mut r,
            &s,
            &mut GlyphAtlas::default(),
            scale,
            &format!("nested-{}", (scale * 100.) as u32),
            Rgba::new(0., 0., 0., 0.),
        );
        let w = (64. * scale) as u32;
        let mut positive = 0;
        for y in 0..w {
            for x in 0..w {
                let p = [
                    (f64::from(x) + 0.5) / f64::from(scale),
                    (f64::from(y) + 0.5) / f64::from(scale),
                ];
                let k = RefShape::from(a)
                    .coverage(p, f64::from(scale))
                    .min(RefShape::from(b).coverage(p, f64::from(scale)));
                let i = ((y * w + x) * 4) as usize;
                expect_pixel(&data[i..i + 4], [k; 4], 2, "nested region");
                if k > 0. {
                    positive += 1;
                }
            }
        }
        assert!(positive > 20);
        let outer = rr(Rect::new(260., 0., 40., 48.), CornerRadii::uniform(4.));
        let mut pair = Vec::new();
        for rect_first in [true, false] {
            let (mut s, keys, _) = table_scene(&[outer]);
            let thin = Rect::new(280.007_4, 0., 1., 48.);
            if rect_first {
                s.push_clip(thin);
                s.push_shape_clip(keys[0].clone());
            } else {
                s.push_shape_clip(keys[0].clone());
                s.push_clip(thin);
            }
            s.push_quad(Quad::new(Rect::new(250., -8., 64., 64.), Rgba::gray(1.)));
            s.pop_clip();
            s.pop_clip();
            let size = ((320. * scale) as u32, (64. * scale) as u32);
            let p = r
                .render_to_rgba8(
                    &s,
                    &mut GlyphAtlas::default(),
                    size,
                    scale,
                    Rgba::new(0., 0., 0., 0.),
                )
                .unwrap();
            save(
                &format!("thin-{rect_first}-{}", (scale * 100.) as u32),
                size,
                &p,
            );
            assert!(
                p.chunks_exact(4).any(|v| v[3] > 20),
                "fractional Rect ancestor must retain positive ink"
            );
            pair.push(p);
        }
        assert_eq!(
            pair[0], pair[1],
            "Rect ancestors and children share analytic support"
        );
    }
    let empty = rr(Rect::new(10., 10., 0., 20.), CornerRadii::uniform(2.));
    let (mut s, keys, _) = table_scene(&[empty]);
    s.push_shape_clip(keys[0].clone());
    s.push_quad(Quad::new(Rect::new(0., 0., 64., 64.), Rgba::gray(1.)));
    s.pop_clip();
    let data = render(
        &mut r,
        &s,
        &mut GlyphAtlas::default(),
        1.,
        "empty-domain",
        Rgba::new(0., 0., 0., 0.),
    );
    assert!(data.iter().all(|v| *v == 0));
    let a = rr(Rect::new(8.5, 20.5, 24., 24.), CornerRadii::uniform(12.));
    let b = rr(Rect::new(32.5, 20.5, 24., 24.), CornerRadii::uniform(12.));
    let (mut s, keys, _) = table_scene(&[a, b]);
    for key in keys {
        s.push_shape_clip(key);
    }
    s.push_quad(Quad::new(Rect::new(0., 0., 64., 64.), Rgba::gray(1.)));
    s.pop_clip();
    s.pop_clip();
    let data = render(
        &mut r,
        &s,
        &mut GlyphAtlas::default(),
        1.,
        "tangent-aa-support",
        Rgba::new(0., 0., 0., 0.),
    );
    assert!(
        data[((32 * 64 + 32) * 4 + 3) as usize].abs_diff(128) <= 1,
        "zero logical area still has declared AA intersection support"
    );
}
#[test]
fn rejected_frames_do_not_invoke_callbacks_or_partially_modify_external_target() {
    let mut r = Renderer::new_headless().unwrap();
    let context = r.gpu_context();
    let target = context.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("clip pre-submit atomicity target"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let mut atlas = GlyphAtlas::default();
    r.render_to_texture(
        &Scene::new(),
        &mut atlas,
        &target,
        1.,
        Rgba::new(0., 1., 0., 1.),
    )
    .unwrap();
    let shape = rr(Rect::new(8., 8., 40., 40.), CornerRadii::uniform(8.));
    let (mut bad, keys, _) = table_scene(&[shape]);
    bad.push_quad(Quad::new(
        Rect::new(0., 0., 64., 64.),
        Rgba::new(1., 0., 0., 1.),
    ));
    bad.push_shape_clip(keys[0].clone());
    let called = Cell::new(false);
    let err = r
        .render_to_rgba8_with_gpu(&bad, &mut atlas, (64, 64), 1., Rgba::gray(0.), |_| {
            called.set(true);
            Ok(())
        })
        .unwrap_err();
    assert!(!called.get());
    assert!(matches!(err, RenderError::InvalidClipFrame(_)));
    assert!(
        r.render_to_texture(&bad, &mut atlas, &target, 1., Rgba::gray(0.))
            .is_err()
    );
    let image = context.import_image(target).unwrap();
    let id = r.register_gpu_image(&image).unwrap();
    let mut read = Scene::new();
    read.push_image(ImageQuad {
        rect: Rect::new(0., 0., 64., 64.),
        image: id,
        opacity: 1.,
    });
    let p = render(
        &mut r,
        &read,
        &mut atlas,
        1.,
        "atomic-target-preserved",
        Rgba::gray(0.),
    );
    assert!(p.chunks_exact(4).all(|x| x == [0, 255, 0, 255]));
    bad.pop_clip();
    let base = r.clip_frame_budget();
    for (name, budget) in [
        (
            "query",
            tekes_ui_render::ClipFrameBudget {
                max_queries: 0,
                ..base
            },
        ),
        (
            "instance",
            tekes_ui_render::ClipFrameBudget {
                max_instances: 0,
                ..base
            },
        ),
        (
            "upload",
            tekes_ui_render::ClipFrameBudget {
                max_upload_bytes: 0,
                ..base
            },
        ),
        (
            "CPU",
            tekes_ui_render::ClipFrameBudget {
                max_cpu_work: 0,
                ..base
            },
        ),
        (
            "records",
            tekes_ui_render::ClipFrameBudget {
                max_records: 0,
                ..base
            },
        ),
    ] {
        r.set_clip_frame_budget(budget);
        let called = Cell::new(false);
        assert!(
            r.render_to_rgba8_with_gpu(&bad, &mut atlas, (64, 64), 1., Rgba::gray(0.), |_| {
                called.set(true);
                Ok(())
            })
            .is_err(),
            "{name}"
        );
        assert!(!called.get(), "{name} invoked callback");
    }
    r.set_clip_frame_budget(base);
    let (mut unknown, _, _) = table_scene(&[shape]);
    unknown.require_clip_geometry(99);
    assert!(matches!(
        r.render_to_rgba8(&unknown, &mut atlas, (64, 64), 1., Rgba::gray(0.)),
        Err(RenderError::UnsupportedClipGeometry(99))
    ));
    let (mut unsupported, _, _) = table_scene(&[shape]);
    unsupported.push_quad(Quad::new(
        Rect::new(2_f32.powi(27), 0., 4., 4.),
        Rgba::gray(1.),
    ));
    assert!(matches!(
        r.render_to_rgba8(&unsupported, &mut atlas, (64, 64), 1., Rgba::gray(0.)),
        Err(RenderError::ClipGeometry(
            ClipError::UnsupportedNumericRange
        ))
    ));
}
#[test]
fn numerical_indeterminate_and_nonfinite_ink_reject_before_callback() {
    let mut renderer = Renderer::new_headless().unwrap();
    let mut atlas = GlyphAtlas::default();
    let small = 2_f32.powi(-50);
    let big = 2_f32.powi(50);
    let a = rr(
        Rect::new(-small, -small, 2. * small, 2. * small),
        CornerRadii::uniform(small),
    );
    let b = rr(
        Rect::new(0., -big, 2. * big, 2. * big),
        CornerRadii::uniform(big),
    );
    let (mut scene, keys, _) = table_scene(&[a, b]);
    for k in keys {
        scene.push_shape_clip(k);
    }
    scene.push_quad(Quad::new(Rect::new(0., 0., 64., 64.), Rgba::gray(1.)));
    scene.pop_clip();
    scene.pop_clip();
    let called = Cell::new(false);
    assert!(matches!(
        renderer.render_to_rgba8_with_gpu(&scene, &mut atlas, (64, 64), 1., Rgba::gray(0.), |_| {
            called.set(true);
            Ok(())
        }),
        Err(RenderError::ClipGeometry(
            ClipError::NumericallyIndeterminate
        ))
    ));
    assert!(!called.get());
    let shape = rr(Rect::new(8., 8., 40., 40.), CornerRadii::uniform(8.));
    let (mut scene, keys, _) = table_scene(&[shape]);
    scene.push_shape_clip(keys[0].clone());
    scene.push_quad(Quad::new(
        Rect::new(0., 0., 64., 64.),
        Rgba::new(f32::NAN, 1., 1., 1.),
    ));
    scene.pop_clip();
    let called = Cell::new(false);
    assert!(
        renderer
            .render_to_rgba8_with_gpu(&scene, &mut atlas, (64, 64), 1., Rgba::gray(0.), |_| {
                called.set(true);
                Ok(())
            })
            .is_err()
    );
    assert!(!called.get());
}
#[test]
fn image_and_glyph_real_alpha_multiply_coverage_and_preserve_independent_draws() {
    let mut renderer = Renderer::new_headless().unwrap();
    let image = renderer.register_image(&[64, 0, 0, 128], 1, 1).unwrap();
    let shape = rr(Rect::new(10.5, 10.5, 40., 40.), CornerRadii::uniform(8.));
    let reference = RefShape::from(shape);
    for scale in [0.5_f32, 1., 1.25, 1.5, 2.] {
        for kind in ["image", "mask", "color"] {
            let mut atlas = GlyphAtlas::new(AtlasConfig {
                page_size: 256,
                max_pages: 2,
                padding: 1,
            });
            let primitive = if kind == "image" {
                tekes_ui_scene::Primitive::Image(ImageQuad {
                    rect: Rect::new(0., 0., 64., 64.),
                    image,
                    opacity: 0.6,
                })
            } else {
                let side = (64. * scale) as u32;
                let format = if kind == "mask" {
                    RasterFormat::Mask
                } else {
                    RasterFormat::Color
                };
                let glyph = atlas
                    .get_or_insert_with(
                        GlyphKey::new(FontKey(81), 1, GlyphKey::scale_key_for(scale)),
                        || {
                            Some(RasterGlyph {
                                left: 0,
                                top: 0,
                                width: side,
                                height: side,
                                format,
                                pixels: [0, 0, 64, 128].repeat((side * side) as usize),
                            })
                        },
                    )
                    .unwrap();
                tekes_ui_scene::Primitive::Glyphs(GlyphRun {
                    color: if kind == "mask" {
                        Rgba::new(1., 0., 0., 0.6)
                    } else {
                        Rgba::new(0., 1., 0., 0.6)
                    },
                    glyphs: vec![PositionedGlyph {
                        origin: Point::new(0., 0.),
                        glyph,
                    }],
                })
            };
            let mut plain = Scene::new();
            plain.push(primitive.clone());
            let name = format!("real-alpha-{kind}-{}", (scale * 100.) as u32);
            let unclip = render(
                &mut renderer,
                &plain,
                &mut atlas,
                scale,
                &format!("{name}-reference"),
                Rgba::new(0., 0., 0., 0.),
            );
            let mut outputs = Vec::new();
            for (clips, draws) in [(1, 1), (2, 1), (1, 2)] {
                let (mut scene, keys, _) = table_scene(&[shape]);
                for _ in 0..clips {
                    scene.push_shape_clip(keys[0].clone());
                }
                for _ in 0..draws {
                    scene.push(primitive.clone());
                }
                for _ in 0..clips {
                    scene.pop_clip();
                }
                outputs.push(render(
                    &mut renderer,
                    &scene,
                    &mut atlas,
                    scale,
                    &format!("{name}-clips{clips}-draws{draws}"),
                    Rgba::new(0., 0., 0., 0.),
                ));
            }
            assert_eq!(
                outputs[0], outputs[1],
                "same geometry clip must not darken real content alpha"
            );
            let w = (64. * scale) as u32;
            let mut product_witnesses = 0;
            let mut overlap_witnesses = 0;
            for y in 0..w {
                for x in 0..w {
                    let p = [
                        (f64::from(x) + 0.5) / f64::from(scale),
                        (f64::from(y) + 0.5) / f64::from(scale),
                    ];
                    let k = reference.coverage(p, f64::from(scale));
                    let i = ((y * w + x) * 4) as usize;
                    let a = f64::from(unclip[i + 3]) / 255. * k;
                    for c in 0..4 {
                        let first = f64::from(unclip[i + c]) * k;
                        let twice = first * (2. - a);
                        assert!(
                            outputs[0][i + c].abs_diff(first.round() as u8) <= 2,
                            "{name}: real content multiplied by geometry"
                        );
                        assert!(
                            outputs[2][i + c].abs_diff(twice.round() as u8) <= 2,
                            "{name}: independent content draws source-over"
                        );
                    }
                    let real = f64::from(unclip[i + 3]) / 255.;
                    if k > 0.05 && k < 0.95 && real.min(k) - real * k > 0.025 {
                        product_witnesses += 1;
                    }
                    if outputs[2][i + 3] > outputs[0][i + 3] + 20 {
                        overlap_witnesses += 1;
                    }
                }
            }
            assert!(
                product_witnesses > 2 && overlap_witnesses > 20,
                "{name}: discriminating alpha samples required"
            );
        }
    }
}
#[test]
fn degenerate_inner_and_maximum_finite_border_keep_the_defined_composite() {
    let mut renderer = Renderer::new_headless().unwrap();
    let shape = rr(Rect::new(10.5, 10.5, 40., 40.), CornerRadii::uniform(8.));
    let reference = RefShape::from(shape);
    for scale in [0.5_f32, 1., 1.25, 1.5, 2.] {
        for border in [0_f32, 20., 100., f32::MAX] {
            let (mut scene, keys, _) = table_scene(&[shape]);
            scene.push_shape_clip(keys[0].clone());
            scene.push_keyed_quad(
                Quad::new(shape.rect(), Rgba::new(0., 0., 1., 0.6))
                    .with_corner_radii(shape.radii())
                    .with_border(border, Rgba::new(1., 0., 0., 0.4)),
                keys[0].clone(),
            );
            scene.pop_clip();
            let name = format!(
                "empty-inner-{:08x}-{}",
                border.to_bits(),
                (scale * 100.) as u32
            );
            let data = render(
                &mut renderer,
                &scene,
                &mut GlyphAtlas::default(),
                scale,
                &name,
                Rgba::new(0., 0., 0., 0.),
            );
            let w = (64. * scale) as u32;
            let mut ink = 0;
            for y in 0..w {
                for x in 0..w {
                    let p = [
                        (f64::from(x) + 0.5) / f64::from(scale),
                        (f64::from(y) + 0.5) / f64::from(scale),
                    ];
                    let u = reference.coverage(p, f64::from(scale));
                    let expected = if border == 0. {
                        [0., 0., 0.6 * u, 0.6 * u]
                    } else {
                        [0.4 * u, 0., 0.36 * u, 0.76 * u]
                    };
                    let i = ((y * w + x) * 4) as usize;
                    expect_pixel(
                        &data[i..i + 4],
                        expected,
                        2,
                        "empty inner contour composite",
                    );
                    if u > 0. {
                        ink += 1;
                    }
                }
            }
            assert!(ink > 20);
        }
    }
    let empty = rr(Rect::new(10.5, 10.5, 0., 40.), CornerRadii::uniform(8.));
    let (mut scene, keys, _) = table_scene(&[empty]);
    scene.push_keyed_quad(
        Quad::new(empty.rect(), Rgba::new(0., 0., 1., 0.6))
            .with_border(f32::MAX, Rgba::new(1., 0., 0., 0.4)),
        keys[0].clone(),
    );
    let data = render(
        &mut renderer,
        &scene,
        &mut GlyphAtlas::default(),
        1.,
        "empty-primitive-outer",
        Rgba::new(0., 0., 0., 0.),
    );
    assert!(
        data.iter().all(|v| *v == 0),
        "an empty outer cannot acquire an AA fringe"
    );
}
#[test]
fn legacy_rect_siblings_keep_original_pixels_inside_a_shape_capable_frame() {
    fn legacy_branch(scene: &mut Scene) {
        scene.push_clip(Rect::new(280.007_4, 0., 1., 48.));
        scene.push_quad(Quad::new(Rect::new(250., -8., 64., 64.), Rgba::gray(1.)));
        scene.pop_clip();
    }
    let mut renderer = Renderer::new_headless().unwrap();
    let offscreen = rr(Rect::new(400., 0., 16., 16.), CornerRadii::uniform(3.));
    for scale in [0.5_f32, 1., 1.25, 1.5, 2.] {
        let mut plain = Scene::new();
        legacy_branch(&mut plain);
        legacy_branch(&mut plain);
        let (mut mixed, keys, _) = table_scene(&[offscreen]);
        legacy_branch(&mut mixed);
        mixed.push_shape_clip(keys[0].clone());
        mixed.push_quad(Quad::new(offscreen.rect(), Rgba::new(1., 0., 0., 1.)));
        mixed.pop_clip();
        legacy_branch(&mut mixed);
        // Also switch between keyed and ordinary draws without a scope boundary:
        // the keyed offscreen draw promotes the Rect ancestor only for itself.
        legacy_branch(&mut plain);
        mixed.push_clip(Rect::new(280.007_4, 0., 1., 48.));
        mixed.push_keyed_quad(
            Quad::new(offscreen.rect(), Rgba::new(1., 0., 0., 1.))
                .with_corner_radii(offscreen.radii()),
            keys[0].clone(),
        );
        mixed.push_quad(Quad::new(Rect::new(250., -8., 64., 64.), Rgba::gray(1.)));
        mixed.pop_clip();
        let size = ((320. * scale) as u32, (64. * scale) as u32);
        let clear = Rgba::new(0., 0.1, 0.2, 1.);
        let a = renderer
            .render_to_rgba8(&plain, &mut GlyphAtlas::default(), size, scale, clear)
            .unwrap();
        let b = renderer
            .render_to_rgba8(&mixed, &mut GlyphAtlas::default(), size, scale, clear)
            .unwrap();
        save(
            &format!("mixed-reference-{}", (scale * 100.) as u32),
            size,
            &a,
        );
        save(
            &format!("mixed-candidate-{}", (scale * 100.) as u32),
            size,
            &b,
        );
        assert!(
            a == b,
            "ordinary Rect branches before/after a shape scope must retain legacy pixels at scale {scale}"
        );
    }
}
