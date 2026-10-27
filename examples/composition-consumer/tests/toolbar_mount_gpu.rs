#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use coretext::Text;
use tekes_ui_composition_consumer::toolbar_mount::*;
use tekes_ui_controls::{composition::ContentEvent, paint};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Theme;
#[test]
fn original_multi_group_progress_and_help_render_on_actual_metal_at_narrow_width_and_zoom() {
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let output = std::env::var_os("TEKESUI_TOOLBAR_MOUNT_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(output) = &output {
        let root = std::path::Path::new("/Volumes/HP P900/DEV/TekesUI/artifacts/issue-136/toolbar69-main2fdddf8-intake/gpu");
        assert!(output.is_absolute() && output.starts_with(root)
            && !output.components().any(|c| c == std::path::Component::ParentDir),
            "GPU output must use the coordinated SSD artifact directory");
        std::fs::create_dir_all(output).unwrap();
    }
    let mut frames = 0;
    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
        for width in [340., 600., 900.] {
            for scale in [1., 1.5, 2.] {
                let mut host = build(width).unwrap();
                let mut text = Text::new(scale);
                text.services = true;
                let mut scene = Scene::new();
                paint::surface(&mut scene, Rect::new(0., 0., width, 360.), theme, false);
                host.render(&mut scene, &mut text, theme.into()).unwrap();
                let target = host
                    .semantics()
                    .into_iter()
                    .find(|n| n.node.id == EDIT_CONTROL)
                    .unwrap()
                    .parts
                    .into_iter()
                    .find(|(_, p)| p.semantics.name == "Undo");
                if let Some((_, part)) = target {
                    host.event(&ContentEvent::PointerMove(part.frame.center()));
                    scene.clear();
                    host.render(&mut scene, &mut text, theme.into()).unwrap();
                    text.now = std::time::Duration::from_secs(1);
                    scene.clear();
                    host.render(&mut scene, &mut text, theme.into()).unwrap();
                    assert!(host.help_tooltip().is_some());
                }
                let size = ((width * 2.) as u32, 720);
                let rgba = gpu
                    .render_to_rgba8(
                        &scene,
                        &mut text.atlas,
                        size,
                        2.,
                        paint::color(theme, "surface-1"),
                    )
                    .unwrap();
                assert!(rgba.chunks_exact(4).any(|p| p != &rgba[0..4]));
                if let Some(output) = &output {
                    resvg::tiny_skia::Pixmap::from_vec(
                        rgba,
                        resvg::tiny_skia::IntSize::from_wh(size.0, size.1).unwrap(),
                    )
                    .unwrap()
                    .save_png(
                        output.join(format!("mount-theme-{ti}-width-{width}-scale-{scale}.png")),
                    )
                    .unwrap();
                }
                frames += 1;
            }
        }
    }
    assert_eq!(frames, 36);
}
