//! Real headless GPU equality against a directly authored single-source Scene.
use tekes_ui_render::Renderer;
use tekes_ui_scene::*;
use tekes_ui_tokens::Rgba;
fn normalized(rect: Rect) -> ClipShape {
    ClipShape::normalize(rect, CornerRadii::uniform(8.), Point::new(0., 0.)).unwrap()
}
fn draw(scene: &mut Scene, key: ShapeKey, rect: Rect, alpha: f32) {
    scene.push_shape_clip(key.clone());
    let mut quad = Quad::new(rect, Rgba::new(0.8, 0.2, 0.1, alpha))
        .with_corner_radius(8.)
        .with_border(1., Rgba::new(0.1, 0.6, 0.2, alpha));
    scene.push_keyed_quad(quad, key);
    quad.rect = Rect::new(rect.origin.x + 4., rect.origin.y + 4., 5., 5.);
    quad.corner_radii = CornerRadii::ZERO;
    scene.push_quad(quad);
    scene.pop_clip();
}
fn caller(scene: &mut Scene, key: ShapeKey) {
    scene.push_clip(Rect::new(1., 1., 62., 62.));
    draw(scene, key, Rect::new(3., 3., 24., 24.), 1.);
    scene.push_quad(Quad::new(
        Rect::new(45., 3., 8., 4.),
        Rgba::new(0.2, 0.3, 0.9, 1.),
    ));
}
#[test]
fn multi_owner_translated_fragment_preserves_existing_curve_rect_and_painter_pixels() {
    let mut renderer = Renderer::new_headless().expect("actual GPU is required; no skip");
    let out = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_FRAGMENT_GPU_OUT").expect("explicit QA output"),
    );
    std::fs::create_dir_all(&out).unwrap();
    for scale in [0.5, 1., 1.25, 1.5, 2.] {
        for alpha in [0.5, 1.] {
            let a = normalized(Rect::new(3., 3., 24., 24.));
            let b = normalized(Rect::new(30.25, 30.5, 24., 24.));
            let mut reference_table = ShapeTable::new(1, 4);
            let ra = reference_table.intern(a).unwrap();
            let rb = reference_table.intern(b).unwrap();
            let mut reference = Scene::with_clip_shapes(&reference_table);
            caller(&mut reference, ra);
            draw(&mut reference, rb, b.rect(), alpha);
            reference.pop_clip();
            let mut ta = ShapeTable::new(1, 4);
            let ka = ta.intern(a).unwrap();
            let mut composed = Scene::with_clip_shapes(&ta);
            caller(&mut composed, ka);
            let mut tb = ShapeTable::new(1, 4);
            let base = normalized(Rect::new(0., 0., 24., 24.));
            let kb = tb.intern(base).unwrap();
            let mut fragment = Scene::with_clip_shapes(&tb);
            draw(&mut fragment, kb, base.rect(), 1.);
            composed
                .try_append_fragment(
                    &fragment,
                    Default::default(),
                    FragmentTransform {
                        translation: Point::new(30.25, 30.5),
                        opacity: alpha,
                        ..Default::default()
                    },
                )
                .unwrap();
            composed.pop_clip();
            let size = ((64_f32 * scale) as u32, (64_f32 * scale) as u32);
            let expected = renderer
                .render_to_rgba8(
                    &reference,
                    &mut GlyphAtlas::default(),
                    size,
                    scale,
                    Rgba::gray(0.),
                )
                .unwrap();
            let actual = renderer
                .render_to_rgba8(
                    &composed,
                    &mut GlyphAtlas::default(),
                    size,
                    scale,
                    Rgba::gray(0.),
                )
                .unwrap();
            let name = format!("scale-{scale}-alpha-{alpha}");
            std::fs::write(out.join(format!("{name}-reference.rgba")), &expected).unwrap();
            std::fs::write(out.join(format!("{name}-composed.rgba")), &actual).unwrap();
            assert_eq!(actual, expected, "{name}: full-target bytes differ");
            assert!(
                actual.chunks_exact(4).any(|p| p[0] > 100),
                "positive ink required"
            );
            let pixel = |x: u32, y: u32| {
                let start = ((y * size.0 + x) * 4) as usize;
                &actual[start..start + 4]
            };
            // Caller sibling survives; empty rounded corner remains clear. These sites
            // have distinct positive/negative roles and are well away from AA support.
            assert!(pixel((48. * scale) as u32, (5. * scale) as u32)[2] > 100);
            assert_eq!(
                &pixel((30.5 * scale) as u32, (30.5 * scale) as u32)[..3],
                &[0, 0, 0]
            );
        }
    }
}
#[test]
fn failed_scene_rejects_all_submission_entries_before_callbacks_and_preserves_external_pixels() {
    use tekes_ui_render::RenderError;
    let mut r = Renderer::new_headless().expect("actual GPU required");
    let context = r.gpu_context();
    let target = context.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("fragment reject target"),
        size: wgpu::Extent3d {
            width: 16,
            height: 16,
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
    let mut bad = Scene::new();
    bad.push_quad(Quad::new(
        Rect::new(0., 0., 16., 16.),
        Rgba::new(1., 0., 0., 1.),
    ));
    bad.record_composition_error(FragmentError::BudgetExceeded);
    let error = RenderError::SceneComposition(FragmentError::BudgetExceeded);
    let called = std::cell::Cell::new(false);
    assert_eq!(
        r.render_to_rgba8_with_gpu(&bad, &mut atlas, (16, 16), 1., Rgba::gray(0.), |_| {
            called.set(true);
            Ok(())
        })
        .unwrap_err(),
        error
    );
    assert!(!called.get());
    assert_eq!(
        r.render_with_gpu(&bad, &mut atlas, Rgba::gray(0.), |_| {
            called.set(true);
            Ok(())
        })
        .unwrap_err(),
        error
    );
    assert!(!called.get());
    assert_eq!(
        r.render_to_texture(&bad, &mut atlas, &target, 1., Rgba::gray(0.))
            .unwrap_err(),
        error
    );
    let imported = context.import_image(target).unwrap();
    let id = r.register_gpu_image(&imported).unwrap();
    let mut read = Scene::new();
    read.push_image(ImageQuad {
        rect: Rect::new(0., 0., 16., 16.),
        image: id,
        opacity: 1.,
    });
    let pixels = r
        .render_to_rgba8(&read, &mut atlas, (16, 16), 1., Rgba::gray(0.))
        .unwrap();
    assert!(
        pixels
            .chunks_exact(4)
            .all(|pixel| pixel == [0, 255, 0, 255])
    );
    let out = std::path::PathBuf::from(std::env::var_os("TEKESUI_FRAGMENT_GPU_OUT").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(
        out.join("rejected-frame-external-target-still-green.rgba"),
        pixels,
    )
    .unwrap();
    bad.clear();
    assert!(
        r.render_to_rgba8(&bad, &mut atlas, (16, 16), 1., Rgba::gray(0.))
            .is_ok()
    );
}
