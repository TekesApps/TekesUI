#![cfg(target_os = "macos")]

use tekes_ui_module_runtime::panel_image_rect;
use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, ImageQuad, Point, Rect, Scene};
use tekes_ui_tokens::Rgba;

#[test]
fn fractional_panel_origins_preserve_rasterized_edges() {
    let mut renderer = Renderer::new_headless().unwrap();
    // Narrow stems plus gray antialiased edges, like an already rasterized glyph.
    let source: Vec<u8> = (0..8)
        .flat_map(|row| {
            [255, 224, 0, 192, 255, 64, 0, 255].map(|v| if row % 2 == 0 { v } else { 255 - v })
        })
        .flat_map(|v| [v, v, v, 255])
        .collect();
    let id = renderer.register_image(&source, 8, 8).unwrap();
    for scale in [1.0, 1.5, 2.0] {
        for origin in [Point::new(4.0 / 3.0, 2.2), Point::new(2.7, 4.0 / 3.0)] {
            let aligned = panel_image_rect(origin, (8, 8), scale);
            let capture = |renderer: &mut Renderer, rect| {
                let mut scene = Scene::new();
                scene.push_image(ImageQuad {
                    rect,
                    image: id,
                    opacity: 1.0,
                });
                renderer
                    .render_to_rgba8(
                        &scene,
                        &mut GlyphAtlas::default(),
                        (24, 24),
                        scale,
                        Rgba::gray(1.0),
                    )
                    .unwrap()
            };
            let pixels = capture(&mut renderer, aligned);
            let x = (aligned.min_x() * scale).round() as usize;
            let y = (aligned.min_y() * scale).round() as usize;
            for row in 0..8 {
                for col in 0..8 {
                    let actual = ((y + row) * 24 + x + col) * 4;
                    let expected = (row * 8 + col) * 4;
                    for channel in 0..4 {
                        assert!(pixels[actual + channel].abs_diff(source[expected + channel]) <= 1);
                    }
                }
            }
            let old = capture(
                &mut renderer,
                Rect::new(origin.x, origin.y, 8.0 / scale, 8.0 / scale),
            );
            let delta = old
                .iter()
                .zip(&pixels)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            eprintln!(
                "scale={scale} origin={origin:?} old_max_channel_error={delta} aligned_max_channel_error<=1"
            );
            assert!(
                delta > 2,
                "fractional placement must reproduce resampling: scale={scale} origin={origin:?} delta={delta}"
            );
        }
    }
}
