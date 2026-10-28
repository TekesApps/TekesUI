use std::sync::Arc;
use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::{
    FragmentBudget, FragmentTransform, GlyphAtlas, ImageQuad, OpaqueImageLease, Point, Rect, Scene,
};
use tekes_ui_tokens::Rgba;

fn scene_for(lease: &tekes_ui_render::gpu::BitmapImageLease) -> Scene {
    let mut scene = Scene::new();
    scene.push_controlled_image(
        ImageQuad {
            rect: Rect::new(0., 0., 4., 4.),
            image: lease.id(),
            opacity: 1.,
        },
        lease.scene_token(),
    );
    scene
}

#[test]
fn controlled_lease_authenticates_renderer_revision_and_lifetime() {
    let mut a = Renderer::new_headless().expect("actual headless GPU required");
    let lease = a
        .register_bitmap_image(&[255, 0, 0, 255].repeat(16), 4, 4)
        .unwrap();
    let scene = scene_for(&lease);
    let clone = scene.clone();
    assert_eq!(scene, clone);

    let mut b = Renderer::new_headless().expect("actual headless GPU required");
    let foreign = b
        .register_bitmap_image(&[0, 255, 0, 255].repeat(16), 4, 4)
        .unwrap();
    assert_eq!(lease.id(), foreign.id());
    let mut callback_count = 0;
    let rejected = b.render_to_rgba8_with_gpu(
        &scene,
        &mut GlyphAtlas::default(),
        (4, 4),
        1.,
        Rgba::gray(0.),
        |_| {
            callback_count += 1;
            Ok(())
        },
    );
    assert!(rejected.is_err());
    assert_eq!(callback_count, 0);

    drop(lease); // Scene is now the sole owner and must still render.
    let pixels = a
        .render_to_rgba8(
            &scene,
            &mut GlyphAtlas::default(),
            (4, 4),
            1.,
            Rgba::gray(0.),
        )
        .unwrap();
    assert_eq!(&pixels[0..4], &[255, 0, 0, 255]);

    let mut fragment = Scene::new();
    fragment
        .try_append_fragment(
            &clone,
            FragmentBudget::default(),
            FragmentTransform {
                translation: Point::new(1., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    let mut replay = Scene::new();
    replay
        .try_append_fragment(&fragment, FragmentBudget::default(), Default::default())
        .unwrap();
    assert!(
        a.render_to_rgba8(
            &replay,
            &mut GlyphAtlas::default(),
            (4, 4),
            1.,
            Rgba::gray(0.)
        )
        .is_ok()
    );

    drop(scene);
    drop(clone);
    drop(fragment);
    drop(replay);
    assert_eq!(a.collect_retired_images(), 1);
    assert_eq!(a.image_count(), 0);
}

#[test]
fn raw_update_unregister_and_fabricated_lease_fail_before_gpu_callback() {
    let mut renderer = Renderer::new_headless().expect("actual headless GPU required");
    let lease = renderer
        .register_bitmap_image(&[20, 30, 40, 255].repeat(4), 2, 2)
        .unwrap();
    let stale = scene_for(&lease);
    assert!(renderer.update_image(lease.id(), &[1, 2, 3]).is_err());
    assert!(
        renderer
            .render_to_rgba8(
                &stale,
                &mut GlyphAtlas::default(),
                (4, 4),
                1.,
                Rgba::gray(0.)
            )
            .is_ok()
    );
    renderer
        .update_image(lease.id(), &[50, 60, 70, 255].repeat(4))
        .unwrap();
    // A new Scene cannot refresh an old immutable lease after raw mutation.
    let reauthored = scene_for(&lease);
    let reauthored_error = renderer
        .render_to_rgba8(
            &reauthored,
            &mut GlyphAtlas::default(),
            (4, 4),
            1.,
            Rgba::gray(0.),
        )
        .unwrap_err();
    assert_eq!(
        reauthored_error,
        RenderError::InvalidArgument("stale or foreign controlled bitmap lease")
    );
    let mut callbacks = 0;
    assert!(
        renderer
            .render_to_rgba8_with_gpu(
                &stale,
                &mut GlyphAtlas::default(),
                (0, 4),
                1.,
                Rgba::gray(0.),
                |_| {
                    callbacks += 1;
                    Ok(())
                }
            )
            .is_err()
    );
    assert_eq!(callbacks, 0);

    let mut surface_callbacks = 0;
    let surface_error = renderer
        .render_with_gpu(&stale, &mut GlyphAtlas::default(), Rgba::gray(0.), |_| {
            surface_callbacks += 1;
            Ok(())
        })
        .unwrap_err();
    assert_eq!(
        surface_error,
        RenderError::InvalidArgument("stale or foreign controlled bitmap lease")
    );
    assert_eq!(surface_callbacks, 0);

    let context = renderer.gpu_context();
    let target = context.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("stale controlled image target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let texture_error = renderer
        .render_to_texture(
            &stale,
            &mut GlyphAtlas::default(),
            &target,
            1.,
            Rgba::gray(0.),
        )
        .unwrap_err();
    assert_eq!(
        texture_error,
        RenderError::InvalidArgument("stale or foreign controlled bitmap lease")
    );

    let mut fake = Scene::new();
    fake.push_controlled_image(
        ImageQuad {
            rect: Rect::new(0., 0., 2., 2.),
            image: lease.id(),
            opacity: 1.,
        },
        OpaqueImageLease::from_owner(Arc::new(42_u32)),
    );
    assert!(
        renderer
            .render_to_rgba8(
                &fake,
                &mut GlyphAtlas::default(),
                (4, 4),
                1.,
                Rgba::gray(0.)
            )
            .is_err()
    );
    let retired_id = lease.id();
    assert!(renderer.unregister_image(retired_id));
    assert!(
        renderer
            .render_to_rgba8(
                &stale,
                &mut GlyphAtlas::default(),
                (4, 4),
                1.,
                Rgba::gray(0.)
            )
            .is_err()
    );
    drop(stale);
    drop(reauthored);
    drop(lease);
    // Explicit unregister already removed this entry; the owner-drop queue is
    // stale and must not report a second retirement.
    assert_eq!(renderer.collect_retired_images(), 0);
    assert_eq!(renderer.image_count(), 0);
    let newer = renderer
        .register_image(&[90, 80, 70, 255].repeat(4), 2, 2)
        .unwrap();
    assert_ne!(newer, retired_id);
    assert_eq!(renderer.collect_retired_images(), 0);
    assert_eq!(renderer.image_count(), 1);
}

#[test]
fn proof_entry_resource_and_work_refusals_preserve_atlas_retirement_and_output() {
    use tekes_ui_render::BoundaryProofLimits;
    use tekes_ui_scene::{
        AtlasConfig, ClipShape, CornerRadii, FontKey, GlyphKey, RasterFormat, RasterGlyph,
    };

    let mut renderer = Renderer::new_headless().expect("actual headless GPU required");
    let lease = renderer
        .register_bitmap_image(&[255, 0, 0, 255].repeat(16), 4, 4)
        .unwrap();
    let scene = scene_for(&lease);
    let scene_before = scene.clone();
    let parents: Arc<[ClipShape]> = vec![
        ClipShape::normalize(
            Rect::new(-8., -8., 20., 20.),
            CornerRadii::uniform(2.),
            Point::new(0., 0.),
        )
        .unwrap(),
    ]
    .into();
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        page_size: 64,
        max_pages: 1,
        padding: 1,
    });
    atlas
        .get_or_insert_with(GlyphKey::new(FontKey(127), 1, 1000), || {
            Some(RasterGlyph {
                left: 0,
                top: 0,
                width: 1,
                height: 1,
                format: RasterFormat::Mask,
                pixels: vec![0, 0, 0, 255],
            })
        })
        .expect("real atlas bitmap entry");
    let output = renderer
        .render_to_rgba8_with_parent_boundary_proof(
            &scene,
            &mut atlas,
            (4, 4),
            1.,
            Rgba::new(0., 0., 0., 0.),
            parents.clone(),
            BoundaryProofLimits::default(),
        )
        .unwrap();
    let output_before = output.rgba8().to_vec();
    assert_eq!(&output_before[..4], &[255, 0, 0, 255]);

    // A pending real retirement and dirty atlas must survive a proof-work refusal.
    let pending = renderer
        .register_bitmap_image(&[0, 0, 255, 255], 1, 1)
        .unwrap();
    drop(pending);
    assert_eq!(renderer.image_count(), 2);
    atlas.mark_all_dirty();
    let atlas_before = atlas.stats();
    let page_before = atlas.page_pixels(0).unwrap().to_vec();
    let refused = renderer
        .render_to_rgba8_with_parent_boundary_proof(
            &scene,
            &mut atlas,
            (4, 4),
            1.,
            Rgba::new(0., 0., 0., 0.),
            parents.clone(),
            BoundaryProofLimits { max_work: 0 },
        )
        .unwrap_err();
    assert_eq!(
        refused,
        RenderError::ClipBudgetExceeded("boundary proof work")
    );
    assert_eq!(
        renderer.image_count(),
        2,
        "refusal must precede retirement collection"
    );
    assert_eq!(
        atlas.stats(),
        atlas_before,
        "refusal must not upload/drain the dirty atlas"
    );
    assert_eq!(atlas.page_pixels(0).unwrap(), page_before);
    assert_eq!(scene, scene_before);
    assert_eq!(output.rgba8(), output_before);
    assert_eq!(renderer.collect_retired_images(), 1);

    // Use invalid dimensions as a second independent error: the existing
    // resource gate must still reject first at this new public proof entry.
    let reject_resource = |renderer: &mut Renderer, atlas: &mut GlyphAtlas, expected| {
        let images_before = renderer.image_count();
        let atlas_before = atlas.stats();
        let error = renderer
            .render_to_rgba8_with_parent_boundary_proof(
                &scene,
                atlas,
                (0, 4),
                1.,
                Rgba::new(0., 0., 0., 0.),
                parents.clone(),
                BoundaryProofLimits::default(),
            )
            .unwrap_err();
        assert_eq!(error, RenderError::InvalidArgument(expected));
        assert_eq!(renderer.image_count(), images_before);
        assert_eq!(atlas.stats(), atlas_before);
        assert_eq!(output.rgba8(), output_before);
    };
    let mut foreign_renderer = Renderer::new_headless().expect("actual foreign GPU namespace");
    let foreign = foreign_renderer
        .register_bitmap_image(&[0, 255, 0, 255].repeat(16), 4, 4)
        .unwrap();
    assert_eq!(
        lease.id(),
        foreign.id(),
        "same numeric slot is not the same renderer resource"
    );
    reject_resource(
        &mut foreign_renderer,
        &mut atlas,
        "stale or foreign controlled bitmap lease",
    );
    renderer
        .update_image(lease.id(), &[0, 255, 0, 255].repeat(16))
        .unwrap();
    reject_resource(
        &mut renderer,
        &mut atlas,
        "stale or foreign controlled bitmap lease",
    );
    assert!(renderer.unregister_image(lease.id()));
    reject_resource(&mut renderer, &mut atlas, "stale controlled bitmap lease");
    assert_eq!(scene, scene_before);
    assert_eq!(
        output.rgba8(),
        output_before,
        "old bound pixels remain immutable after registry replacement/removal"
    );
}
