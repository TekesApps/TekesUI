//! Real public SplitView painter on Metal. Independent of Gallery and AppHost.
#[cfg(target_os = "macos")]
#[test]
fn original_split_view_gpu_keeps_layout_and_reduces_only_decorative_divider_ink() {
    use tekes_ui_controls::{
        extended::{Axis, SplitView},
        layout_view::SplitViewView,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{GlyphAtlas, Rect, Scene};
    use tekes_ui_tokens::{Contrast, Palette, Theme};
    let mut renderer = Renderer::new_headless().expect("actual GPU required");
    let mut atlas = GlyphAtlas::default();
    let output = std::env::var_os("TEKESUI_SPLIT_VIEW_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(out) = &output {
        std::fs::create_dir_all(out).unwrap();
    }
    let mut evidence = vec![];
    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
        for brand in 0..3 {
            for scale in [1., 1.5, 2.] {
                for (ai, axis) in [Axis::Horizontal, Axis::Vertical].into_iter().enumerate() {
                    let palette = Palette { theme, brand };
                    let bounds = Rect::new(8., 8., 80., 80.);
                    let mut before_model = SplitView::new(0.5);
                    let mut after_model = SplitView::new(0.5);
                    let mut before_view = SplitViewView::new(axis);
                    let mut after_view = SplitViewView::new(axis);
                    let mut before = Scene::new();
                    let mut after = Scene::new();
                    let before_layout = before_view.render(
                        &mut before,
                        &mut before_model,
                        bounds,
                        Some(bounds),
                        palette,
                    );
                    let after_layout = after_view.render_at_scale(
                        &mut after,
                        &mut after_model,
                        bounds,
                        Some(bounds),
                        palette,
                        scale,
                    );
                    assert_eq!(before_layout, after_layout);
                    let size = ((96. * scale) as u32, (96. * scale) as u32);
                    let clear = palette.color("surface-1").unwrap();
                    let old = renderer
                        .render_to_rgba8(&before, &mut atlas, size, scale, clear)
                        .unwrap();
                    let new = renderer
                        .render_to_rgba8(&after, &mut atlas, size, scale, clear)
                        .unwrap();
                    let energy = |pixels: &[u8]| -> u64 {
                        let background = &pixels[..4];
                        pixels
                            .chunks_exact(4)
                            .map(|p| {
                                (0..3)
                                    .map(|c| p[c].abs_diff(background[c]) as u64)
                                    .sum::<u64>()
                            })
                            .sum()
                    };
                    let old_energy = energy(&old);
                    let new_energy = energy(&new);
                    assert!(
                        old_energy > 0 && new_energy > 0,
                        "divider must actually paint"
                    );
                    let changed = old
                        .chunks_exact(4)
                        .zip(new.chunks_exact(4))
                        .filter(|(a, b)| a != b)
                        .count();
                    if scale == 1. || theme.contrast == Contrast::Increased {
                        assert_eq!(old, new, "legacy 1x and increased contrast are preserved");
                    } else {
                        assert!(
                            changed > 0 && new_energy < old_energy,
                            "real scaled divider must be lighter than the old 1pt divider"
                        );
                    }
                    let name = format!("divider-t{ti}-b{brand}-s{scale}-a{ai}");
                    if let Some(out) = &output {
                        std::fs::write(out.join(format!("{name}-before.rgba")), &old).unwrap();
                        std::fs::write(out.join(format!("{name}-after.rgba")), &new).unwrap();
                    }
                    evidence.push(serde_json::json!({"case":name,"width":size.0,"height":size.1,"scale":scale,"increased_contrast":theme.contrast==Contrast::Increased,"changed_pixels":changed,"before_contrast_energy":old_energy,"after_contrast_energy":new_energy,"layout_and_hit_geometry_equal":true}));
                }
            }
        }
    }
    if let Some(out) = &output {
        std::fs::write(out.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"scope":"Original SplitViewView divider only; real GPU, not desktop interaction or full Gold7 acceptance", "cases":evidence})).unwrap()).unwrap();
    }
    println!("SPLIT_VIEW_REAL_GPU_OK cases={}", evidence.len());
}
