//! Second public-API consumer: asset -> wgpu -> ordinary TekesUI image -> PNG.
use tekes_ui_scene3d::{Asset3D, Renderer3D, Scene3D};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let path = args.get(1).map(String::as_str).unwrap_or(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/animated-tetra.glb"
    ));
    let out = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("/tmp/tekes-scene3d.png");
    let mut view = Scene3D::new(Asset3D::load(path)?);
    if view.asset().clips().len() > 0 {
        view.select_clip(0)?;
        view.seek(0.75)?;
    }
    let mut ui = tekes_ui_render::Renderer::new_headless()?;
    let mut gpu = Renderer3D::new(&ui.gpu_context(), &view, (512, 512))?;
    gpu.render(&view, 0., 1.)?;
    let image = ui.register_gpu_image(gpu.output())?;
    let mut scene = tekes_ui_scene::Scene::new();
    view.paint(
        &mut scene,
        tekes_ui_scene::Rect::new(0., 0., 512., 512.),
        image,
    );
    let rgba = ui.render_to_rgba8(
        &scene,
        &mut tekes_ui_scene::GlyphAtlas::new(Default::default()),
        (512, 512),
        1.,
        tekes_ui_tokens::Rgba::gray(0.12),
    )?;
    let mut encoder = png::Encoder::new(std::fs::File::create(out)?, 512, 512);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgba)?;
    println!("wgpu + UI composition: {out}");
    Ok(())
}
