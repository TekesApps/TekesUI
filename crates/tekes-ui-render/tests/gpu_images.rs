//! Real GPU tests; readback is only the final assertion/capture, never the
//! component-to-UI path. These exercise ownership, color space and painter order.
use tekes_ui_render::{
    Renderer,
    gpu::{GpuContext, GpuImage},
};
use tekes_ui_scene::{GlyphAtlas, ImageQuad, Quad, Rect, Scene};
use tekes_ui_tokens::Rgba;

fn clear(context: &GpuContext, image: &GpuImage, color: wgpu::Color) {
    let mut encoder = context.device().create_command_encoder(&Default::default());
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: image.render_view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    }
    context.queue().submit([encoder.finish()]);
}
fn capture(ui: &mut Renderer, scene: &Scene) -> Vec<u8> {
    ui.render_to_rgba8(
        scene,
        &mut GlyphAtlas::new(Default::default()),
        (8, 8),
        1.,
        Rgba::gray(0.),
    )
    .unwrap()
}
#[test]
#[ignore = "requires a real wgpu GPU adapter"]
fn gpu_storage_color_order_clipping_replacement_and_lifetime() {
    let mut ui = Renderer::new_headless().unwrap();
    let context = ui.gpu_context();
    let image = context.create_image((4, 4)).unwrap();
    assert!(context.create_image((0, 1)).is_err());
    clear(
        &context,
        &image,
        wgpu::Color {
            r: 0.25,
            g: 0.5,
            b: 0.75,
            a: 1.,
        },
    );
    let id = ui.register_gpu_image(&image).unwrap();
    assert!(ui.update_image(id, &[0; 64]).is_err());
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(
        Rect::new(0., 0., 8., 8.),
        Rgba::new(1., 0., 0., 1.),
    ));
    scene.push_clip(Rect::new(2., 2., 4., 4.));
    scene.push_image(ImageQuad {
        rect: Rect::new(0., 0., 8., 8.),
        image: id,
        opacity: 1.,
    });
    scene.pop_clip();
    scene.push_quad(Quad::new(
        Rect::new(3., 3., 2., 2.),
        Rgba::new(0., 1., 0., 1.),
    ));
    let frame = capture(&mut ui, &scene);
    let pixel =
        |frame: &[u8], x: usize, y: usize| frame[(y * 8 + x) * 4..(y * 8 + x) * 4 + 4].to_vec();
    let encoded = pixel(&frame, 2, 2);
    // These are linear -> sRGB results, preserved by encoded UI sampling.
    for (actual, expected) in encoded[..3].iter().zip([137_u8, 188, 225]) {
        assert!(actual.abs_diff(expected) <= 1, "{encoded:?}");
    }
    assert_eq!(pixel(&frame, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 3, 3), [0, 255, 0, 255]);
    drop(image);
    assert_eq!(frame, capture(&mut ui, &scene));
    let replacement = context.create_image((7, 3)).unwrap();
    clear(&context, &replacement, wgpu::Color::BLUE);
    ui.replace_gpu_image(id, &replacement).unwrap();
    assert_eq!(ui.image_count(), 1);
    assert_eq!(pixel(&capture(&mut ui, &scene), 2, 2), [0, 0, 255, 255]);
    clear(&context, &replacement, wgpu::Color::TRANSPARENT);
    assert_eq!(pixel(&capture(&mut ui, &scene), 2, 2), [255, 0, 0, 255]);
    assert!(ui.unregister_image(id));
    assert!(ui.replace_gpu_image(id, &replacement).is_err());
}
#[test]
#[ignore = "requires real wgpu GPU adapters"]
fn foreign_context_and_wrong_image_kind_are_rejected_before_gpu_binding() {
    let mut a = Renderer::new_headless().unwrap();
    let mut b = Renderer::new_headless().unwrap();
    let image = a.gpu_context().create_image((2, 2)).unwrap();
    assert!(b.register_gpu_image(&image).is_err());
    let b_image = b.gpu_context().create_image((2, 2)).unwrap();
    let id = b.register_gpu_image(&b_image).unwrap();
    assert!(b.replace_gpu_image(id, &image).is_err());
    let cpu = a.register_image(&[255; 16], 2, 2).unwrap();
    assert!(a.replace_gpu_image(cpu, &image).is_err());
    a.update_image(cpu, &[0; 16]).unwrap();
}
