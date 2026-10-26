//! Actual Metal + CoreText exercise of the single Button content contract.
#![cfg(target_os = "macos")]
use tekes_ui_controls::{Button, composition::*};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};
#[path = "support/coretext.rs"]
mod coretext;
use coretext::Text;
#[test]
fn text_image_mixed_and_original_content_use_the_real_shared_button_on_metal() {
    let mut renderer = tekes_ui_render::Renderer::new_headless().expect("real GPU");
    let output = std::env::var_os("TEKESUI_BUTTON_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(out) = &output {
        let root = std::path::Path::new("/Volumes/HP P900/DEV/TekesUI/artifacts/issue-136/unified-button-main36dd132-intake/gpu");
        assert!(out.is_absolute() && out.starts_with(root) && !out.components().any(|c| c == std::path::Component::ParentDir), "GPU output must use the coordinated SSD artifact directory");
        std::fs::create_dir_all(out).unwrap();
    }
    let mut evidence = vec![];
    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
        for scale in [1., 1.5, 2.] {
            let palette = Palette::from(theme);
            let ink = tekes_ui_tokens::surface_palette::resolve_surface(palette,"canvas","action-quiet","rest").unwrap().icon;
            let glyph = tekes_ui_icons::Glyph::ALL
                .iter()
                .find(|g| g.name == "tekes-settings")
                .expect("approved regular settings SVG");
            let channel = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
            let options = resvg::usvg::Options {
                style_sheet: Some(format!(
                    "svg {{ color: rgb({}, {}, {}); }}",
                    channel(ink.r),
                    channel(ink.g),
                    channel(ink.b)
                )),
                ..Default::default()
            };
            let tree = resvg::usvg::Tree::from_str(glyph.svg, &options).unwrap();
            let side = (proposal::metric("size", "size-icon-standard").unwrap() * scale * 2.).ceil()
                as u32;
            let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
            let factor = side as f32 / tree.size().width();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(factor, factor),
                &mut pixels.as_mut(),
            );
            let image = renderer.register_image(pixels.data(), side, side).unwrap();
            let mut text = Text::new(scale);
            let mut scene = Scene::new();
            for kind in 0..4 {
                let button = match kind {
                    0 => Button::new("Save · 保存"),
                    1 => Button::new(image).with_accessible_name("Settings"),
                    2 => Button::new((image, "Settings · 设置")),
                    _ => Button::new(Content::new(
                        ComponentId(2),
                        TextContent::new("Custom · Local"),
                    ))
                    .with_accessible_name("Open local settings"),
                };
                let mut host = ContentHost::new(Content::new(ComponentId(1), button)).unwrap();
                let frame = Rect::new(
                    12.,
                    12. + kind as f32 * 72. * scale,
                    320. * scale,
                    56. * scale,
                );
                host.render(
                    &mut scene,
                    &mut text,
                    frame,
                    Rect::new(0., 0., 700., 620.),
                    palette,
                )
                .unwrap();
                assert_eq!(host.semantics().len(), 1);
                assert_eq!(host.focus_targets().len(), 1);
            }
            let size = (1400, 1240);
            let rgba = renderer
                .render_to_rgba8(
                    &scene,
                    &mut text.atlas,
                    size,
                    2.,
                    palette.color("surface-1").unwrap(),
                )
                .unwrap();
            assert!(
                rgba.chunks_exact(4).any(|p| p != &rgba[0..4]),
                "real content painted"
            );
            if let Some(out) = &output {
                resvg::tiny_skia::Pixmap::from_vec(
                    rgba,
                    resvg::tiny_skia::IntSize::from_wh(size.0, size.1).unwrap(),
                )
                .unwrap()
                .save_png(out.join(format!("button-theme-{ti}-scale-{scale}.png")))
                .unwrap();
            }
            evidence.push(serde_json::json!({"theme":format!("{theme:?}"),"type_scale":scale,"backing_scale":2,"content_shapes":4,"renderer":"actual Metal with CoreText glyph atlas and approved registered SVG","native_interaction":false,"full_gold7_acceptance":false}));
            renderer.unregister_image(image);
        }
    }
    if let Some(out) = &output {
        std::fs::write(
            out.join("receipt.json"),
            serde_json::to_vec_pretty(&evidence).unwrap(),
        )
        .unwrap();
    }
}
