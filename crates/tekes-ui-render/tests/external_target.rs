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
