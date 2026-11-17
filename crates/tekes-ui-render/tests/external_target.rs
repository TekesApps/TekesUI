use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, ImageQuad, Quad, Rect, Scene};
use tekes_ui_tokens::Rgba;

#[test]
fn external_target_is_rendered_and_sampled_without_cpu_upload() {
    let mut renderer = Renderer::new_headless().unwrap();
    let context = renderer.gpu_context();
    let texture = context.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("public API external target consumer"),
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
    let mut source = Scene::new();
    source.push_quad(Quad::new(
        Rect::new(0., 0., 64., 64.),
        Rgba::rgb8(20, 90, 180),
    ));
    let mut atlas = GlyphAtlas::default();
    renderer
        .render_to_texture(&source, &mut atlas, &texture, 1., Rgba::gray(0.))
        .unwrap();
    let image = context.import_image(texture).unwrap();
    assert_eq!(image.render_format(), wgpu::TextureFormat::Bgra8Unorm);
    let id = renderer.register_gpu_image(&image).unwrap();
    let mut dest = Scene::new();
    dest.push_image(ImageQuad {
        rect: Rect::new(0., 0., 64., 64.),
        image: id,
        opacity: 1.,
    });
    let pixels = renderer
        .render_to_rgba8(&dest, &mut atlas, (64, 64), 1., Rgba::gray(0.))
        .unwrap();
    let px = &pixels[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4];
    for (actual, expected) in px.iter().zip([20u8, 90, 180, 255]) {
        assert!(actual.abs_diff(expected) <= 1, "{px:?}");
    }
}

#[test]
fn direct_layer_and_boundary_proof_texture_paths_match_rgba8_readback() {
    use std::sync::Arc;
    use tekes_ui_render::BoundaryProofLimits;
    use tekes_ui_scene::{ClipShape, CornerRadii, Point};

    let mut renderer = Renderer::new_headless().unwrap();
    let context = renderer.gpu_context();
    let size = (64u32, 64u32);
    let make_target = |label| {
        context.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    };

    let mut source = Scene::new();
    source.push_quad(Quad::new(
        Rect::new(8., 8., 48., 48.),
        Rgba::rgb8(180, 70, 40),
    ));
    let mut atlas = GlyphAtlas::default();

    // 1. Layer direct-to-texture parity against render_layer_to_rgba8
    let expected_layer = renderer
        .render_layer_to_rgba8(&source, &mut atlas, size, 1.)
        .unwrap();
    let layer_tex = make_target("layer direct target");
    renderer
        .render_layer_to_texture(&source, &mut atlas, &layer_tex, 1.)
        .unwrap();
    let layer_img = context.import_image(layer_tex).unwrap();
    let layer_id = renderer.register_gpu_image(&layer_img).unwrap();
    let mut composite = Scene::new();
    composite.push_image(ImageQuad {
        rect: Rect::new(0., 0., 64., 64.),
        image: layer_id,
        opacity: 1.,
    });
    let actual_layer = renderer
        .render_to_rgba8(&composite, &mut atlas, size, 1., Rgba::new(0., 0., 0., 0.))
        .unwrap();
    renderer.unregister_image(layer_id);
    assert_eq!(actual_layer, expected_layer.rgba8());

    // 2. Parent-boundary-proof direct-to-texture parity against render_to_rgba8_with_parent_boundary_proof
    let parent_shape = ClipShape::normalize(
        Rect::new(0., 0., 64., 64.),
        CornerRadii::uniform(12.),
        Point::default(),
    )
    .unwrap();
    let mut proof_scene = Scene::new();
    proof_scene.push_clip(Rect::new(16., 16., 32., 32.));
    proof_scene.push_quad(Quad::new(
        Rect::new(16., 16., 32., 32.),
        Rgba::rgb8(30, 140, 90),
    ));
    proof_scene.pop_clip();
    let limits = BoundaryProofLimits::default();
    let expected_proof = renderer
        .render_to_rgba8_with_parent_boundary_proof(
            &proof_scene,
            &mut atlas,
            size,
            1.,
            Rgba::new(0., 0., 0., 0.),
            Arc::from([parent_shape]),
            limits,
        )
        .unwrap();
    let proof_tex = make_target("proof direct target");
    let submission = renderer
        .render_to_texture_with_parent_boundary_proof(
            &proof_scene,
            &mut atlas,
            &proof_tex,
            1.,
            Rgba::new(0., 0., 0., 0.),
            Arc::from([parent_shape]),
            limits,
        )
        .unwrap();
    assert_eq!(submission.dimensions_px(), size);
    let _index = submission.into_submission_index();
    let proof_img = context.import_image(proof_tex).unwrap();
    let proof_id = renderer.register_gpu_image(&proof_img).unwrap();
    let mut proof_composite = Scene::new();
    proof_composite.push_image(ImageQuad {
        rect: Rect::new(0., 0., 64., 64.),
        image: proof_id,
        opacity: 1.,
    });
    let actual_proof = renderer
        .render_to_rgba8(
            &proof_composite,
            &mut atlas,
            size,
            1.,
            Rgba::new(0., 0., 0., 0.),
        )
        .unwrap();
    renderer.unregister_image(proof_id);
    assert_eq!(actual_proof, expected_proof.rgba8());
}
