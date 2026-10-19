use tekes_ui_scene3d::{Asset3D, Repeat, Scene3D};
fn component() -> Scene3D {
    Scene3D::new(Asset3D::from_slice(include_bytes!("../fixtures/animated-tetra.glb")).unwrap())
}
#[test]
fn formats_and_clip_metadata() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    for file in [
        "animated-tetra.glb",
        "animated-tetra.gltf",
        "skinned-tetra.glb",
    ] {
        let a = Asset3D::load(root.join(file)).unwrap();
        assert_eq!(a.mesh_count(), 1);
        assert_eq!(a.clips().len(), 2);
        assert_eq!(a.clips().next().unwrap().duration, 2.);
    }
    assert!(Asset3D::from_slice(b"broken").is_err());
    assert!(Asset3D::load("test.blend").is_err());
}
#[test]
fn lifecycle_freezes_hidden_and_reduced_motion_without_catchup() {
    let mut c = component();
    c.select_clip(0).unwrap();
    c.play();
    c.advance(0.5).unwrap();
    c.visible = false;
    c.advance(10.).unwrap();
    assert_eq!(c.time(), 0.5);
    assert!(!c.needs_frame());
    c.visible = true;
    c.reduced_motion = true;
    c.advance(10.).unwrap();
    assert_eq!(c.time(), 0.5);
    c.reduced_motion = false;
    c.advance(0.5).unwrap();
    assert_eq!(c.time(), 1.);
    c.pause();
    assert!(!c.needs_frame());
    c.play();
    c.repeat = Repeat::Once;
    c.advance(20.).unwrap();
    assert_eq!(c.time(), 2.);
    assert!(!c.needs_frame());
    c.seek(0.).unwrap();
    c.repeat = Repeat::Loop;
    c.play();
    c.advance(2.5).unwrap();
    assert_eq!(c.time(), 0.5);
    assert!(c.select_clip(999).is_err());
    assert!(c.seek(f32::NAN).is_err());
    assert!(c.set_speed(-1.).is_err());
    assert!(c.advance(f32::INFINITY).is_err());
}
#[test]
fn scene_composition_uses_host_image_and_visibility() {
    let mut c = component();
    let mut scene = tekes_ui_scene::Scene::new();
    c.paint(
        &mut scene,
        tekes_ui_scene::Rect::new(5., 10., 80., 60.),
        tekes_ui_scene::ImageId(7),
    );
    assert_eq!(scene.len(), 1);
    c.visible = false;
    c.paint(
        &mut scene,
        tekes_ui_scene::Rect::default(),
        tekes_ui_scene::ImageId(7),
    );
    assert_eq!(scene.len(), 1);
}
/// Explicit opt-in: a real wgpu adapter is required. Never skip-success on errors.
#[test]
#[ignore = "requires a real wgpu GPU adapter: cargo test -p tekes-ui-scene3d -- --ignored"]
fn gpu_animation_changes_pixels_and_resizes() {
    use tekes_ui_scene3d::Renderer3D;
    let mut c = component();
    c.select_clip(0).unwrap();
    let mut ui = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut gpu = Renderer3D::new(&ui.gpu_context(), &c, (127, 93)).unwrap();
    gpu.render(&c, 0., 1.).unwrap();
    let image = ui.register_gpu_image(gpu.output()).unwrap();
    let capture = |ui: &mut tekes_ui_render::Renderer, c: &Scene3D, size: (u32, u32)| {
        let mut scene = tekes_ui_scene::Scene::new();
        c.paint(
            &mut scene,
            tekes_ui_scene::Rect::new(0., 0., size.0 as f32, size.1 as f32),
            image,
        );
        ui.render_to_rgba8(
            &scene,
            &mut tekes_ui_scene::GlyphAtlas::new(Default::default()),
            size,
            1.,
            tekes_ui_tokens::Rgba::new(0., 0., 0., 0.),
        )
        .unwrap()
    };
    let first = capture(&mut ui, &c, (127, 93));
    let allocations = gpu.vertex_buffer_allocations();
    let view = gpu.output().render_view().clone();
    c.seek(0.75).unwrap();
    gpu.render(&c, 0., 1.).unwrap();
    assert_eq!(gpu.vertex_buffer_allocations(), allocations);
    assert_eq!(gpu.output().render_view(), &view);
    assert_eq!(ui.image_count(), 1);
    let second = capture(&mut ui, &c, (127, 93));
    assert_eq!(first.len(), 127 * 93 * 4);
    assert_ne!(first, second);
    assert!(first.chunks_exact(4).any(|p| p[3] == 255));
    assert!(first.chunks_exact(4).any(|p| p[3] == 0));
    gpu.resize((63, 65)).unwrap();
    gpu.render(&c, 0., 1.).unwrap();
    ui.replace_gpu_image(image, gpu.output()).unwrap();
    let resized = capture(&mut ui, &c, (63, 65));
    assert_eq!(resized.len(), 63 * 65 * 4);
    assert_eq!(ui.image_count(), 1);
    assert!(gpu.resize((0, 65)).is_err());
    assert!(gpu.render(&c, f32::NAN, 1.).is_err());
    assert!(gpu.render(&component(), 0., 1.).is_err());
    drop(gpu);
    assert_eq!(resized, capture(&mut ui, &c, (63, 65)));
    assert!(ui.unregister_image(image));
    assert_eq!(ui.image_count(), 0);
}
