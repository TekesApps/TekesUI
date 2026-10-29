//! Independent raster contracts. These tests do not certify the original
//! Reader/CoreText input, transport lifecycle, pointer, AX or text coordinates.
use tekes_ui_render::{ClipFrameBudget, RenderError, Renderer};
use tekes_ui_scene::{
    AtlasConfig, FontKey, GlyphAtlas, GlyphKey, ImageQuad, RasterFormat, RasterGlyph, Rect, Scene,
    TransformedImageQuad,
};
use tekes_ui_tokens::Rgba;

#[test]
#[ignore = "explicit GPU validation under the existing build/GPU locks"]
fn independent_layer_refusals_keep_dirty_atlas_retirement_scene_and_old_pixels() {
    let mut renderer = Renderer::new_headless().expect("actual GPU required");
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        page_size: 64,
        max_pages: 1,
        padding: 1,
    });
    atlas
        .get_or_insert_with(GlyphKey::new(FontKey(81), 1, 1000), || {
            Some(RasterGlyph {
                left: 0,
                top: 0,
                width: 1,
                height: 1,
                format: RasterFormat::Mask,
                pixels: vec![0, 0, 0, 255],
            })
        })
        .unwrap();
    let lease = renderer
        .register_bitmap_image(&[19, 87, 233, 255].repeat(16), 4, 4)
        .unwrap();
    let quad = ImageQuad {
        rect: Rect::new(0., 0., 4., 4.),
        image: lease.id(),
        opacity: 1.,
    };
    let mut original = Scene::new();
    original.push_controlled_image(quad, lease.scene_token());
    let original_before = original.clone();
    let output = renderer
        .render_layer_to_rgba8(&original, &mut atlas, (4, 4), 1.)
        .unwrap();
    assert_eq!(output.dimensions_px(), (4, 4));
    assert_eq!(output.backing_scale(), 1.);
    assert_eq!(&output.rgba8()[..4], &[19, 87, 233, 255]);
    let old_pixels = output.rgba8().to_vec();
    let pending = renderer
        .register_bitmap_image(&[1, 2, 3, 255], 1, 1)
        .unwrap();
    drop(pending);
    atlas.mark_all_dirty();
    let stats = atlas.stats();
    let page = atlas.page_pixels(0).unwrap().to_vec();
    for (rotation, uv, expected) in [
        (
            f32::INFINITY,
            Rect::new(0., 0., 1., 1.),
            "nonfinite image transform",
        ),
        (0., Rect::new(0., 0., -1., 1.), "invalid image UV geometry"),
    ] {
        let mut invalid = Scene::new();
        let mut transformed = TransformedImageQuad::new(quad, rotation);
        transformed.uv = uv;
        invalid.push_transformed_image(transformed);
        let before = invalid.clone();
        assert_eq!(
            renderer
                .render_layer_to_rgba8(&invalid, &mut atlas, (4, 4), 1.)
                .unwrap_err(),
            RenderError::InvalidClipFrame(expected)
        );
        assert_eq!(invalid, before);
        assert_eq!(renderer.image_count(), 2);
        assert_eq!(atlas.stats(), stats);
        assert_eq!(atlas.page_pixels(0).unwrap(), page);
        assert_eq!(output.rgba8(), old_pixels);
    }
    let budget = renderer.clip_frame_budget();
    renderer.set_clip_frame_budget(ClipFrameBudget {
        max_instances: 0,
        ..budget
    });
    assert_eq!(
        renderer
            .render_layer_to_rgba8(&original, &mut atlas, (4, 4), 1.)
            .unwrap_err(),
        RenderError::ClipBudgetExceeded("instances")
    );
    assert_eq!(renderer.image_count(), 2);
    assert_eq!(atlas.stats(), stats);
    assert_eq!(atlas.page_pixels(0).unwrap(), page);
    assert_eq!(original, original_before);
    assert_eq!(output.rgba8(), old_pixels);
    renderer.set_clip_frame_budget(budget);
    renderer
        .update_image(lease.id(), &[41, 23, 17, 255].repeat(16))
        .unwrap();
    assert_eq!(
        renderer
            .render_layer_to_rgba8(&original, &mut atlas, (4, 4), 1.)
            .unwrap_err(),
        RenderError::InvalidArgument("stale or foreign controlled bitmap lease")
    );
    assert_eq!(renderer.image_count(), 2);
    assert_eq!(atlas.stats(), stats);
    assert_eq!(atlas.page_pixels(0).unwrap(), page);
    assert_eq!(original, original_before);
    assert_eq!(output.rgba8(), old_pixels);
    assert_eq!(renderer.collect_retired_images(), 1);
}

#[cfg(all(target_os = "macos", feature = "macos-shared-surface"))]
#[test]
#[ignore = "explicit Metal surface/pixel validation under the existing build/GPU locks"]
fn independent_surface_matches_same_raster_parent_mask_at_fractional_placement_and_scale() {
    use std::{fs, io::Write, path::PathBuf};
    use tekes_ui_render::shared_surface_macos;
    use tekes_ui_scene::{
        ClipShape, CornerRadii, FragmentBudget, FragmentTransform, Point, Quad, ShapeTable,
    };
    let root = PathBuf::from(
        std::env::var_os("TEKESUI_LAYER_TEST_OUTPUT")
            .expect("fresh SSD evidence directory required"),
    );
    assert!(root.starts_with("/Volumes/HP P900/DEV/TekesUI/artifacts/issue-81/"));
    fs::create_dir(&root).expect("never overwrite prior layer evidence");
    let mut renderer = Renderer::new_headless().expect("actual Metal required");
    for (index, (scale, origin)) in [
        (1., Point::new(333.333_34, 0.)),
        (2., Point::new(666.666_7, 0.375)),
        (3., Point::new(333.333_34, 0.25)),
    ]
    .into_iter()
    .enumerate()
    {
        let physical_origin = [origin.x * scale, origin.y * scale];
        let pixel_origin = physical_origin.map(f32::floor);
        let phase = [
            physical_origin[0] - pixel_origin[0],
            physical_origin[1] - pixel_origin[1],
        ];
        // Keep original Request 333x500 and parent allocation 333.33334x500.
        let size = (
            (333_f32 * scale + phase[0]).ceil() as u32,
            (500_f32 * scale + phase[1]).ceil() as u32,
        );
        let mut child = Scene::new();
        child.push_clip(Rect::new(0., 0., 333., 500.));
        child.push_quad(Quad::new(
            Rect::new(0., 0., 333., 500.),
            Rgba::new(0.13, 0.37, 0.71, 1.),
        ));
        child.push_quad(
            Quad::new(Rect::new(0., 372., 333., 32.), Rgba::gray(1.))
                .with_corner_radius(8.)
                .with_border(1., Rgba::gray(0.45)),
        );
        child.push_quad(
            Quad::new(Rect::new(0., 412., 75.117_676, 24.), Rgba::gray(0.9)).with_corner_radius(8.),
        );
        // One-physical-pixel contrast bars reveal any extra texture resampling.
        for x in 0..16 {
            child.push_quad(Quad::new(
                Rect::new((40 + x) as f32 / scale, 30., 1. / scale, 20.),
                Rgba::gray(if x % 2 == 0 { 0. } else { 1. }),
            ));
        }
        child.pop_clip();
        let before = child.clone();
        let mut translated = Scene::new();
        translated
            .try_append_fragment(
                &child,
                FragmentBudget::default(),
                FragmentTransform {
                    translation: Point::new(phase[0] / scale, phase[1] / scale),
                    ..Default::default()
                },
            )
            .unwrap();
        let translated_before = translated.clone();
        let mut atlas = GlyphAtlas::default();
        let ordinary = renderer
            .render_to_rgba8(
                &translated,
                &mut atlas,
                size,
                scale,
                Rgba::new(0., 0., 0., 0.),
            )
            .unwrap();
        let layer = renderer
            .render_layer_to_rgba8(&translated, &mut atlas, size, scale)
            .unwrap();
        assert_eq!(layer.rgba8(), ordinary);
        assert_eq!(child, before);
        assert_eq!(translated, translated_before);
        let context = renderer.gpu_context();
        let (texture, surface) =
            shared_surface_macos::create_from_layer_frame(&context, &layer).unwrap();
        let image = context.import_image(texture).unwrap();
        let registration = renderer.register_owned_gpu_image(&image).unwrap();
        let reference = renderer
            .register_image(layer.rgba8(), size.0, size.1)
            .unwrap();
        let parent = ClipShape::normalize(
            Rect::new(origin.x, origin.y, 333.333_34, 500.),
            CornerRadii::uniform(10.),
            Point::default(),
        )
        .unwrap();
        let mut table = ShapeTable::new(81, 1);
        let key = table.intern(parent).unwrap();
        let mut actual = Scene::with_clip_shapes(&table);
        actual.push_shape_clip(key.clone());
        // The texture's destination is its pixel extent / backing scale, never
        // stretched to the fractional parent's logical allocation.
        let placement = Rect::new(
            pixel_origin[0] / scale,
            pixel_origin[1] / scale,
            size.0 as f32 / scale,
            size.1 as f32 / scale,
        );
        actual.push_image(ImageQuad {
            rect: placement,
            image: registration.id(),
            opacity: 1.,
        });
        actual.pop_clip();
        let mut expected = Scene::with_clip_shapes(&table);
        expected.push_shape_clip(key);
        expected.push_image(ImageQuad {
            rect: placement,
            image: reference,
            opacity: 1.,
        });
        expected.pop_clip();
        let target = (
            ((origin.x + 340.) * scale).ceil() as u32,
            ((origin.y + 508.) * scale).ceil() as u32,
        );
        let got = renderer
            .render_to_rgba8(
                &actual,
                &mut atlas,
                target,
                scale,
                Rgba::new(0., 0., 0., 0.),
            )
            .unwrap();
        let want = renderer
            .render_to_rgba8(
                &expected,
                &mut atlas,
                target,
                scale,
                Rgba::new(0., 0., 0., 0.),
            )
            .unwrap();
        for (label, bytes, dimensions) in [
            ("child", layer.rgba8(), size),
            ("surface-parent-mask", got.as_slice(), target),
            ("same-raster-reference", want.as_slice(), target),
        ] {
            let path = root.join(format!("{index}-{label}.rgba"));
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap();
            file.write_all(bytes).unwrap();
            let png_file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(root.join(format!("{index}-{label}.png")))
                .unwrap();
            let mut encoder = png::Encoder::new(png_file, dimensions.0, dimensions.1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(bytes)
                .unwrap();
        }
        fs::write(root.join(format!("{index}-geometry.txt")), format!(
            "scale={scale} logical_origin={origin:?} physical_origin={physical_origin:?} pixel_origin={pixel_origin:?} phase={phase:?} original_request=333x500 original_allocation=333.33334x500 source_pixels={size:?} parent_target={target:?} parent={parent:?}\nRGBA raw buffers are exact pixel truth; PNG is a visual encoding.\n"
        )).unwrap();
        assert_eq!(got, want, "same independent raster reference, case {index}");
        // Independent quality checks: pixel-aligned transport must preserve
        // source contrast exactly, not merely agree with another mask render.
        let stripe_y = (40. * scale + phase[1]).floor() as u32;
        let mut contrast_min = 255_u8;
        let mut contrast_max = 0_u8;
        let mut max_transport_delta = 0_u8;
        for x in 40..56_u32 {
            let source = ((stripe_y * size.0 + x) * 4) as usize;
            let destination =
                (((pixel_origin[1] as u32 + stripe_y) * target.0 + pixel_origin[0] as u32 + x) * 4)
                    as usize;
            for channel in 0..4 {
                max_transport_delta = max_transport_delta
                    .max(got[destination + channel].abs_diff(layer.rgba8()[source + channel]));
            }
            contrast_min = contrast_min.min(got[destination]);
            contrast_max = contrast_max.max(got[destination]);
        }
        assert_eq!(
            max_transport_delta, 0,
            "transport must not blur contrast bars"
        );
        assert!(
            contrast_max > contrast_min,
            "contrast bars must remain distinct"
        );
        let content_right = physical_origin[0] + 333. * scale;
        let allocation_right = (origin.x + 333.333_34) * scale;
        let remainder_x = (content_right - 0.5).ceil() as u32;
        let remainder_y = (physical_origin[1] + 100. * scale).floor() as u32;
        let remainder = ((remainder_y * target.0 + remainder_x) * 4) as usize;
        if remainder_x as f32 + 0.5 < allocation_right {
            assert_eq!(
                &got[remainder..remainder + 4],
                &[0; 4],
                "allocation remainder must not be filled by stretching the child"
            );
            let background = renderer
                .render_to_rgba8(
                    &actual,
                    &mut atlas,
                    target,
                    scale,
                    Rgba::new(0., 0., 1., 1.),
                )
                .unwrap();
            assert_eq!(
                &background[remainder..remainder + 4],
                &[0, 0, 255, 255],
                "transparent allocation remainder must reveal the parent background"
            );
        }
        fs::write(root.join(format!("{index}-quality.txt")), format!(
            "same_raster_reference_diff_bytes=0 max_transport_delta={max_transport_delta} contrast_min={contrast_min} contrast_max={contrast_max} content_right_px={content_right} allocation_right_px={allocation_right} remainder_sample_x={remainder_x}\n"
        )).unwrap();
        let at = |x: u32, y: u32| ((y * target.0 + x) * 4) as usize;
        let corner = at(
            (origin.x * scale).ceil() as u32,
            (origin.y * scale).ceil() as u32,
        );
        assert_eq!(
            &got[corner..corner + 4],
            &[0; 4],
            "rounded parent corner must clip"
        );
        assert!(
            got.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255),
            "retain parent edge AA"
        );
        let outside = at(target.0 - 1, target.1 - 1);
        assert_eq!(
            &got[outside..outside + 4],
            &[0; 4],
            "no leakage outside layer/parent"
        );
        drop(registration);
        renderer.collect_retired_images();
        assert!(renderer.unregister_image(reference));
        drop(surface);
    }
}
