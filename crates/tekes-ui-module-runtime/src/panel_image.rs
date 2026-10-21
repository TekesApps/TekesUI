use tekes_ui_scene::{Point, Rect};

/// Place an already rasterized panel on device pixels, with one source texel
/// per destination pixel. Fractional split positions must not filter text a
/// second time through the image compositor.
pub fn panel_image_rect(origin: Point, pixel_size: (u32, u32), scale: f32) -> Rect {
    assert!(scale.is_finite() && scale > 0.0);
    Rect::new(
        (origin.x * scale).round() / scale,
        (origin.y * scale).round() / scale,
        pixel_size.0 as f32 / scale,
        pixel_size.1 as f32 / scale,
    )
}
