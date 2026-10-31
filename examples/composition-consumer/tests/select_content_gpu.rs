#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{ControlKey, Select, composition::*, layout_view::SelectView};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};
#[test]
#[ignore = "explicit actual GPU evidence; fresh output directory required"]
fn select_content_matches_original_view_pixels_clip_and_positive_text_ink() {
    let out = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_SELECT89_GPU_OUT").expect("fresh SSD output"),
    );
    assert!(out.starts_with("/Volumes/HP P900/DEV/TekesUI/artifacts/issue-89/"));
    assert!(!out.exists());
    std::fs::create_dir_all(&out).unwrap();
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut receipts = vec![];
    let definitions: &[&str] = if std::env::var_os("TEKESUI_SELECT89_EXTERNAL").is_some() {
        &["external-default", "external-inputs"]
    } else {
        &["builtin"]
    };
    for definition in definitions {
        for (mode, theme) in Theme::ALL.into_iter().enumerate() {
            for scale in [1., 2.] {
                for surface in ["sidebar", "code"] {
                    let palette = if *definition == "builtin" {
                        Palette::from(theme)
                    } else {
                        Palette::for_theme_definition(theme, definition)
                            .expect("compiled independent consumer Theme")
                    };
                    let mut text = coretext::Text::new(1.);
                    text.backing = scale;
                    text.services = true;
                    text.reduced_motion = true;
                    text.atlas.begin_frame();
                    let mut owner =
                        SelectContent::new("Mode", vec!["One 阅读".into(), "Two".into()]);
                    let mut model = Select::new(2);
                    let mut view = SelectView::new(2);
                    view.set_reduced_motion(true);
                    view.set_surface(surface).unwrap();
                    let frame = Rect::new(20., 20., 220., 40.);
                    let clip = Rect::new(10., 10., 280., 200.);
                    let context = ContentTheme::from(palette).with_surface(surface).unwrap();
                    owner.paint(&mut Scene::new(), &mut text, frame, clip, context);
                    view.render_trigger(
                        &mut Scene::new(),
                        &mut text,
                        &mut model,
                        &["One 阅读", "Two"],
                        frame,
                        Some(clip),
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    owner.set_focus(true, true);
                    view.focus(true);
                    for down in [true, false] {
                        owner.event(&ContentEvent::Key {
                            key: ControlKey::Enter,
                            down,
                        });
                        view.key(ControlKey::Enter, down, &mut model);
                    }
                    assert!(owner.model().menu.open && model.menu.open);
                    let mut actual = Scene::new();
                    owner.paint(&mut actual, &mut text, frame, clip, context);
                    owner.paint_overlay(&mut actual, &mut text, frame, clip, context);
                    let mut reference = Scene::new();
                    view.render(
                        &mut reference,
                        &mut text,
                        &mut model,
                        &["One 阅读", "Two"],
                        frame,
                        clip,
                        Some(clip),
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    );
                    assert_eq!(
                        owner.layout().popup.as_ref().unwrap().popup,
                        view.layout().popup.as_ref().unwrap().popup
                    );
                    let dims = ((320. * scale) as u32, (240. * scale) as u32);
                    let clear = tekes_ui_tokens::Rgba::new(0.91, 0.87, 0.79, 1.);
                    let pixels = gpu
                        .render_to_rgba8(&actual, &mut text.atlas, dims, scale, clear)
                        .unwrap();
                    let expected = gpu
                        .render_to_rgba8(&reference, &mut text.atlas, dims, scale, clear)
                        .unwrap();
                    assert_eq!(
                        pixels, expected,
                        "adapter/original mismatch {mode}/{scale}/{surface}"
                    );
                    let mut no_text = Scene::new();
                    for p in actual.primitives() {
                        if !matches!(p, Primitive::Glyphs(_)) {
                            no_text.push(p.clone());
                        }
                    }
                    let without = gpu
                        .render_to_rgba8(&no_text, &mut text.atlas, dims, scale, clear)
                        .unwrap();
                    let ink = pixels
                        .chunks_exact(4)
                        .zip(without.chunks_exact(4))
                        .filter(|(a, b)| a != b)
                        .count();
                    assert!(ink > 25, "positive glyph pixels required");
                    let file = format!("{definition}-{mode}-{scale}-{surface}.png");
                    resvg::tiny_skia::Pixmap::from_vec(
                        pixels,
                        resvg::tiny_skia::IntSize::from_wh(dims.0, dims.1).unwrap(),
                    )
                    .unwrap()
                    .save_png(out.join(&file))
                    .unwrap();
                    let id = ComponentId(89);
                    let mut hosted = ContentHost::new(Content::new(
                        id,
                        SelectContent::new("Mode", vec!["One 阅读".into(), "Two".into()]),
                    ))
                    .unwrap();
                    hosted.set_inherited_surface(surface).unwrap();
                    hosted
                        .render(&mut Scene::new(), &mut text, frame, clip, palette)
                        .unwrap();
                    assert!(hosted.focus(Some(id), true));
                    assert!(
                        hosted
                            .accessibility_action(id, None, AccessibilityAction::Press)
                            .consumed
                    );
                    let mut host_scene = Scene::new();
                    hosted
                        .render(&mut host_scene, &mut text, frame, clip, palette)
                        .unwrap();
                    let host_pixels = gpu
                        .render_to_rgba8(&host_scene, &mut text.atlas, dims, scale, clear)
                        .unwrap();
                    assert_eq!(
                        host_pixels, expected,
                        "actual Host / original View pixels {mode}/{scale}/{surface}"
                    );
                    let host_file = format!("host-{definition}-{mode}-{scale}-{surface}.png");
                    resvg::tiny_skia::Pixmap::from_vec(
                        host_pixels,
                        resvg::tiny_skia::IntSize::from_wh(dims.0, dims.1).unwrap(),
                    )
                    .unwrap()
                    .save_png(out.join(&host_file))
                    .unwrap();
                    // Same real host clock and original runtime on logical close.
                    assert!(
                        hosted
                            .accessibility_action(id, None, AccessibilityAction::Press)
                            .consumed
                    );
                    view.key(ControlKey::Escape, true, &mut model);
                    text.reduced_motion = false;
                    text.now = std::time::Duration::from_millis(500);
                    view.set_reduced_motion(false);
                    view.tick(0.5);
                    let mut begin_exit = Scene::new();
                    hosted
                        .render(&mut begin_exit, &mut text, frame, clip, palette)
                        .unwrap();
                    let mut begin_reference = Scene::new();
                    view.render(
                        &mut begin_reference,
                        &mut text,
                        &mut model,
                        &["One 阅读", "Two"],
                        frame,
                        clip,
                        Some(clip),
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    );
                    text.now = std::time::Duration::from_millis(516);
                    view.tick(0.016);
                    let mut exit = Scene::new();
                    hosted
                        .render(&mut exit, &mut text, frame, clip, palette)
                        .unwrap();
                    let mut exit_reference = Scene::new();
                    view.render(
                        &mut exit_reference,
                        &mut text,
                        &mut model,
                        &["One 阅读", "Two"],
                        frame,
                        clip,
                        Some(clip),
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    );
                    let exited = gpu
                        .render_to_rgba8(&exit, &mut text.atlas, dims, scale, clear)
                        .unwrap();
                    let exit_expected = gpu
                        .render_to_rgba8(&exit_reference, &mut text.atlas, dims, scale, clear)
                        .unwrap();
                    assert_eq!(
                        exited, exit_expected,
                        "actual Host outgoing / original View pixels"
                    );
                    assert!(
                        !hosted
                            .get(id)
                            .unwrap()
                            .component::<SelectContent>()
                            .unwrap()
                            .overlay_open()
                    );
                    assert!(
                        hosted
                            .get(id)
                            .unwrap()
                            .component::<SelectContent>()
                            .unwrap()
                            .semantic_parts()
                            .is_empty()
                    );
                    let exit_file = format!("exit-{definition}-{mode}-{scale}-{surface}.png");
                    resvg::tiny_skia::Pixmap::from_vec(
                        exited,
                        resvg::tiny_skia::IntSize::from_wh(dims.0, dims.1).unwrap(),
                    )
                    .unwrap()
                    .save_png(out.join(&exit_file))
                    .unwrap();
                    receipts.push(serde_json::json!({"definition":definition,"theme":mode,"scale":scale,"surface":surface,"phase":"actual-host-exit","file":exit_file,"original_view_pixels_equal":true,"input_parts":0}));
                    receipts.push(serde_json::json!({"definition":definition,"theme":mode,"scale":scale,"surface":surface,"phase":"actual-host-open","file":host_file,"original_view_pixels_equal":true}));
                    receipts.push(serde_json::json!({"definition":definition,"theme":mode,"scale":scale,"surface":surface,"file":file,"positive_text_pixels":ink,"original_view_pixels_equal":true}));
                }
            }
        }
    }
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"cases":receipts,"native_window":false,"physical_ime":false,"voiceover":false})).unwrap()).unwrap();
}
