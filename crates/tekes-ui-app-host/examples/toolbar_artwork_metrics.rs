//! Read-only raster ink measurement; never normalizes individual images at runtime.
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    let mut rows = Vec::new();
    for glyph in tekes_ui_icons::Glyph::ALL {
        let tree = resvg::usvg::Tree::from_str(glyph.svg, &resvg::usvg::Options::default())?;
        let mut image = resvg::tiny_skia::Pixmap::new(240, 240).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(10., 10.),
            &mut image.as_mut(),
        );
        let mut bounds = (240_u32, 240_u32, 0_u32, 0_u32);
        let mut count = 0;
        for y in 0..240 {
            for x in 0..240 {
                if image.data()[((y * 240 + x) * 4 + 3) as usize] >= 16 {
                    bounds.0 = bounds.0.min(x);
                    bounds.1 = bounds.1.min(y);
                    bounds.2 = bounds.2.max(x + 1);
                    bounds.3 = bounds.3.max(y + 1);
                    count += 1;
                }
            }
        }
        anyhow::ensure!(count > 0, "empty artwork: {}", glyph.name);
        rows.push(serde_json::json!({"name":glyph.name,"ink_bounds_artboard":[bounds.0 as f32/10.,bounds.1 as f32/10.,bounds.2 as f32/10.,bounds.3 as f32/10.],"visible_height_at_16pt":(bounds.3-bounds.1) as f32/10.*16./24.}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"actual raster alpha >=16/255; fixed 24-unit artboard at 10x; read-only measurement, no runtime per-icon scaling","icons":rows})
        )?
    );
    Ok(())
}
